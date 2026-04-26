//! Ports: typed, push-based message endpoints.
//!
//! Lava's port hierarchy:
//!
//! ```text
//!   AbstractPort
//!     ├── AbstractIOPort
//!     │     ├── OutPort          (src, fan-out)
//!     │     └── InPort           (dst, fan-in)
//!     └── AbstractRVPort
//!           ├── RefPort          (1:1 src, reads/writes a remote Var)
//!           └── VarPort          (1:1 dst, exposes a local Var)
//! ```
//!
//! We mirror the four concrete kinds. Connections are declared at build
//! time; data flows push-based at runtime via the underlying `Channel`.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::channel::{Channel, RecvError, SendError};

/// Globally-unique port identifier (assigned by `PortId::fresh`).
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct PortId(u64);

impl PortId {
    pub fn fresh() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Source-side IO port. Sends `T` messages downstream.
pub struct OutPort<T> {
    pub id: PortId,
    pub name: &'static str,
    channel: Channel<T>,
}

impl<T> OutPort<T> {
    pub fn new(name: &'static str, capacity: usize) -> Self {
        Self {
            id: PortId::fresh(),
            name,
            channel: Channel::bounded(capacity),
        }
    }

    /// Push a message. Blocks if the bound is reached.
    pub fn send(&self, value: T) -> Result<(), SendError> {
        self.channel.send(value)
    }

    /// Wire this OutPort to an InPort. Both must agree on `T`.
    /// In Lava this is `OutPort.connect(InPort)`; we collapse to channel
    /// reuse — the InPort takes ownership of the receiving half.
    pub fn connect(self, inp: &mut InPort<T>) {
        inp.channel = Some(self.channel);
    }
}

/// Destination-side IO port. Receives `T` messages from upstream.
pub struct InPort<T> {
    pub id: PortId,
    pub name: &'static str,
    channel: Option<Channel<T>>,
}

impl<T> InPort<T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            id: PortId::fresh(),
            name,
            channel: None,
        }
    }

    /// Blocking receive. Returns `Disconnected` if not connected.
    pub fn recv(&self) -> Result<T, RecvError> {
        match &self.channel {
            Some(c) => c.recv(),
            None => Err(RecvError::Disconnected),
        }
    }

    /// Non-blocking receive (`probe`).
    pub fn try_recv(&self) -> Result<T, RecvError> {
        match &self.channel {
            Some(c) => c.try_recv(),
            None => Err(RecvError::Disconnected),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.channel.is_some()
    }
}

/// RefPort — 1:1 source side that reads/writes a remote Var via a VarPort.
///
/// We keep the message type generic so a Process can issue typed
/// `Read(addr)` / `Write(addr, value)` requests upstream of a VarPort.
pub struct RefPort<Req, Resp> {
    pub id: PortId,
    pub name: &'static str,
    pub req: OutPort<Req>,
    pub resp: InPort<Resp>,
}

impl<Req, Resp> RefPort<Req, Resp> {
    pub fn new(name: &'static str, capacity: usize) -> Self {
        Self {
            id: PortId::fresh(),
            name,
            req: OutPort::new(name, capacity),
            resp: InPort::new(name),
        }
    }
}

/// VarPort — 1:1 destination side that exposes a local Var.
pub struct VarPort<Req, Resp> {
    pub id: PortId,
    pub name: &'static str,
    pub req: InPort<Req>,
    pub resp: OutPort<Resp>,
}

impl<Req, Resp> VarPort<Req, Resp> {
    pub fn new(name: &'static str, capacity: usize) -> Self {
        Self {
            id: PortId::fresh(),
            name,
            req: InPort::new(name),
            resp: OutPort::new(name, capacity),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_in_round_trip() {
        let out = OutPort::<u8>::new("o", 4);
        let mut inp = InPort::<u8>::new("i");
        out.connect(&mut inp);
        assert!(inp.is_connected());
        // We can't reuse `out` after move, so build another for sending:
        let out2 = OutPort::<u8>::new("o2", 4);
        let mut inp2 = InPort::<u8>::new("i2");
        out2.send(42).unwrap();
        out2.connect(&mut inp2);
        assert_eq!(inp2.recv().unwrap(), 42);
    }

    #[test]
    fn unique_ids() {
        let a = PortId::fresh();
        let b = PortId::fresh();
        assert_ne!(a, b);
    }
}
