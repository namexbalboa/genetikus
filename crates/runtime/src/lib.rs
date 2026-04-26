//! genetikus-runtime
//!
//! Fixed-step orchestrator. Mirrors the Runtime / RuntimeService split from
//! `lava.magma.runtime`:
//!
//!   * [`Runtime`]        – the top-level controller. The user calls `run`,
//!                           `pause`, `stop`. It dispatches `MgmtCommand`s
//!                           over CSP channels to one or more services.
//!   * [`RuntimeService`] – owns a partition of Processes. For each tick it
//!                           cycles through the phase loop, gating on a
//!                           `Barrier` so all peers complete phase P at
//!                           tick T before phase P+1 begins.
//!
//! Phases are a Lava-ism (`SPK`, `PRE_MGMT`, `LRN`, `POST_MGMT`). For genetic
//! simulation we relabel them: `Replication`, `Mutation`, `Selection`,
//! `Reporting`. The structure is identical.

pub mod phase;
pub mod runtime;
pub mod service;

pub use phase::Phase;
pub use runtime::{Runtime, RuntimeError};
pub use service::RuntimeService;
