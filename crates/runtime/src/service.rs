//! RuntimeService: owns a partition of Processes and runs the phase loop.
//!
//! In Lava one `RuntimeService` instance is bound to one compute resource
//! (a Python sub-graph on a CPU, or a Loihi chip). It receives `MgmtCommand`s
//! from the Runtime over a CSP channel, and for each tick broadcasts each
//! phase to its ProcessModels. It blocks waiting for *all* PMs to respond
//! `Done` before advancing — this is the implicit barrier.
//!
//! We make the barrier explicit with `std::sync::Barrier`. Each Process runs
//! on its own thread; the service hands all of them through the same Barrier
//! once per phase.

use std::sync::{Arc, Barrier};
use std::thread::{self, JoinHandle};

use genetikus_magma::{MgmtCommand, MgmtResponse, Process};

use crate::phase::Phase;

/// A handle to a Process running on its own thread, gated by a phase barrier.
pub struct RuntimeService {
    name: String,
    handles: Vec<JoinHandle<()>>,
    /// Ticket-style command channel — one tx per worker.
    txs: Vec<crossbeam_channel::Sender<MgmtCommand>>,
    /// Aggregated response channel from all workers.
    resp_rx: crossbeam_channel::Receiver<MgmtResponse>,
    n_workers: usize,
}

impl RuntimeService {
    /// Spawn one thread per Process. All threads synchronise on a shared
    /// `Barrier` after every phase, providing the per-tick safety guarantee.
    ///
    /// `processes` is a Vec because we move ownership of each Process into
    /// its worker thread.
    pub fn spawn(
        name: impl Into<String>,
        processes: Vec<Box<dyn Process>>,
    ) -> Self {
        let n_workers = processes.len();
        // +1 for the service thread, which also crosses the barrier so it
        // can send the next phase command afterwards. Here the service waits
        // on responses instead, so workers-only is fine.
        let barrier = Arc::new(Barrier::new(n_workers.max(1)));
        let (resp_tx, resp_rx) = crossbeam_channel::unbounded::<MgmtResponse>();

        let mut txs = Vec::with_capacity(n_workers);
        let mut handles = Vec::with_capacity(n_workers);

        for mut proc in processes {
            let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<MgmtCommand>();
            let resp_tx = resp_tx.clone();
            let barrier = Arc::clone(&barrier);

            txs.push(cmd_tx);

            handles.push(thread::spawn(move || {
                proc.init();
                'outer: loop {
                    let cmd = match cmd_rx.recv() {
                        Ok(c) => c,
                        Err(_) => break 'outer,
                    };
                    match cmd {
                        MgmtCommand::Run { steps } => {
                            for tick in 0..steps {
                                for _phase in Phase::cycle() {
                                    proc.step(tick);
                                    // Barrier: wait until every peer Process
                                    // has completed this phase of this tick.
                                    barrier.wait();
                                }
                            }
                            let _ = resp_tx.send(MgmtResponse::Done);
                        }
                        MgmtCommand::Pause => {
                            let _ = resp_tx.send(MgmtResponse::Paused);
                        }
                        MgmtCommand::Stop => {
                            proc.shutdown();
                            let _ = resp_tx.send(MgmtResponse::Terminated);
                            break 'outer;
                        }
                        MgmtCommand::GetData | MgmtCommand::SetData => {
                            // Var R/W ports handled at a higher layer.
                            let _ = resp_tx.send(MgmtResponse::Done);
                        }
                    }
                }
            }));
        }

        Self {
            name: name.into(),
            handles,
            txs,
            resp_rx,
            n_workers,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Broadcast a command to every worker.
    pub fn broadcast(&self, cmd: MgmtCommand) {
        for tx in &self.txs {
            // Worker threads outlive this; ignore disconnect on shutdown.
            let _ = tx.send(cmd);
        }
    }

    /// Block until every worker reports a response. Returns the responses
    /// in arrival order (use unanimity to detect anomalies).
    pub fn await_all(&self) -> Vec<MgmtResponse> {
        (0..self.n_workers)
            .filter_map(|_| self.resp_rx.recv().ok())
            .collect()
    }

    /// Send `Stop` to every worker and join their threads.
    pub fn shutdown(self) {
        self.broadcast(MgmtCommand::Stop);
        let _ = self.await_all();
        for h in self.handles {
            let _ = h.join();
        }
    }
}
