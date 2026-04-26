//! CSP-style bounded channel.
//!
//! Lava's `PyPyChannel` uses a shared-memory ring buffer with two semaphores
//! (`req`, `ack`). On the Rust side we get the same semantics for free from
//! `crossbeam-channel::bounded`: it is MPMC, blocks the sender on full and
//! the receiver on empty, and supports non-blocking `try_*` variants
//! corresponding to Lava's `probe()` / `peek()`.

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SendError {
    #[error("channel disconnected")]
    Disconnected,
    #[error("channel full")]
    Full,
}

#[derive(Debug, Error)]
pub enum RecvError {
    #[error("channel disconnected")]
    Disconnected,
    #[error("channel empty")]
    Empty,
}

/// A typed, bounded, blocking-by-default message channel.
///
/// The `Channel<T>` value bundles both ends so a runtime builder can hand
/// the `Sender` to one Process and the `Receiver` to another at compile
/// time (cf. Lava's compiler emitting `runtime_to_service` / `service_to_runtime`
/// pairs).
pub struct Channel<T> {
    pub tx: Sender<T>,
    pub rx: Receiver<T>,
}

impl<T> Channel<T> {
    /// Create a bounded channel with `capacity` slots.
    pub fn bounded(capacity: usize) -> Self {
        let (tx, rx) = bounded(capacity);
        Self { tx, rx }
    }

    /// Split into the two halves. Mirrors Lava's `CspSendPort` / `CspRecvPort`.
    pub fn split(self) -> (Sender<T>, Receiver<T>) {
        (self.tx, self.rx)
    }

    /// Blocking send; fails iff the receiver was dropped.
    pub fn send(&self, value: T) -> Result<(), SendError> {
        self.tx
            .send(value)
            .map_err(|_| SendError::Disconnected)
    }

    /// Non-blocking send (`probe`-style for senders).
    pub fn try_send(&self, value: T) -> Result<(), SendError> {
        use crossbeam_channel::TrySendError;
        self.tx.try_send(value).map_err(|e| match e {
            TrySendError::Full(_) => SendError::Full,
            TrySendError::Disconnected(_) => SendError::Disconnected,
        })
    }

    /// Blocking receive.
    pub fn recv(&self) -> Result<T, RecvError> {
        self.rx.recv().map_err(|_| RecvError::Disconnected)
    }

    /// Non-blocking receive (`probe`).
    pub fn try_recv(&self) -> Result<T, RecvError> {
        self.rx.try_recv().map_err(|e| match e {
            TryRecvError::Empty => RecvError::Empty,
            TryRecvError::Disconnected => RecvError::Disconnected,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let ch = Channel::<u32>::bounded(2);
        ch.send(7).unwrap();
        assert_eq!(ch.recv().unwrap(), 7);
    }

    #[test]
    fn try_recv_empty() {
        let ch = Channel::<u32>::bounded(1);
        assert!(matches!(ch.try_recv(), Err(RecvError::Empty)));
    }

    #[test]
    fn try_send_full() {
        let ch = Channel::<u32>::bounded(1);
        ch.try_send(1).unwrap();
        assert!(matches!(ch.try_send(2), Err(SendError::Full)));
    }
}
