//! Top-level Runtime — the user-facing controller.
//!
//! In Lava: `Runtime.start(RunSteps(N, blocking=True))` packages the run
//! command and dispatches it to every `RuntimeService`. Here it does the
//! same: hold a vector of services and broadcast `MgmtCommand`s to each.

use thiserror::Error;

use genetikus_magma::{MgmtCommand, MgmtResponse};

use crate::service::RuntimeService;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("at least one process reported an error during run")]
    ProcessError,
    #[error("runtime is already stopped")]
    AlreadyStopped,
}

pub struct Runtime {
    services: Vec<RuntimeService>,
    stopped: bool,
}

impl Runtime {
    pub fn new(services: Vec<RuntimeService>) -> Self {
        Self {
            services,
            stopped: false,
        }
    }

    /// Run for `steps` ticks, blocking until every service reports `Done`.
    pub fn run(&self, steps: u64) -> Result<(), RuntimeError> {
        if self.stopped {
            return Err(RuntimeError::AlreadyStopped);
        }
        for s in &self.services {
            s.broadcast(MgmtCommand::Run { steps });
        }
        for s in &self.services {
            for r in s.await_all() {
                if r == MgmtResponse::Error {
                    return Err(RuntimeError::ProcessError);
                }
            }
        }
        Ok(())
    }

    pub fn pause(&self) {
        for s in &self.services {
            s.broadcast(MgmtCommand::Pause);
            let _ = s.await_all();
        }
    }

    /// Stop every service and join all worker threads.
    pub fn stop(mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        for s in self.services.drain(..) {
            s.shutdown();
        }
    }
}
