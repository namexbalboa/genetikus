//! genetikus-magma
//!
//! Lava-inspired core abstractions:
//!   * `Process`  – discrete-time computational entity with a lifecycle.
//!   * `Port`     – typed, push-based message endpoint (In / Out / Ref / Var).
//!   * `Channel`  – CSP-style bounded queue connecting two ports.
//!   * Lifecycle  – `MgmtCommand` / `MgmtResponse` / `ProcessState`.
//!
//! The Lava framework separates *interface* (the Process) from *behavior*
//! (the ProcessModel). We mirror that: a `Process` is a struct holding ports
//! and `Var`s; behavior is supplied by an impl of `ProcessModel`.

pub mod channel;
pub mod lifecycle;
pub mod port;
pub mod process;

pub use channel::{Channel, RecvError, SendError};
pub use lifecycle::{MgmtCommand, MgmtResponse, ProcessState};
pub use port::{InPort, OutPort, PortId, RefPort, VarPort};
pub use process::{Process, ProcessId, ProcessModel, Var};
