//! Process abstraction.
//!
//! Lava separates the *interface* (the Process subclass with declared ports
//! and Vars) from the *behavior* (a `ProcessModel` selected by the compiler
//! based on the target resource). We do the same:
//!
//!   * `Process`      – any value implementing the trait carries the static
//!                       interface (id, name, port handles) plus a `step`
//!                       hook that's called once per tick.
//!   * `ProcessModel` – marker trait for a behavior implementation; a single
//!                       Process may have multiple ProcessModels (CPU / SIMD /
//!                       neuromorphic) selected at build time.
//!
//! The `Var<T>` wrapper mirrors `magma.core.process.variable.Var`.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::lifecycle::ProcessState;

/// Globally unique process identifier.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct ProcessId(u64);

impl ProcessId {
    pub fn fresh() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    pub fn raw(self) -> u64 {
        self.0
    }
}

/// A simulation Variable owned by a Process.
///
/// Mirrors `lava.magma.core.process.variable.Var` — declared on the Process,
/// backed by a buffer at compile time. Here we collapse interface + storage
/// since we have no separate "compile" phase yet.
#[derive(Debug, Clone)]
pub struct Var<T> {
    pub name: &'static str,
    pub value: T,
}

impl<T> Var<T> {
    pub const fn new(name: &'static str, value: T) -> Self {
        Self { name, value }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn set(&mut self, v: T) {
        self.value = v;
    }
}

/// A discrete-time computational entity.
///
/// One `step` per tick, gated by the runtime barrier. Implementations may
/// read from `InPort`s and write to `OutPort`s, mutate `Var`s, etc.
pub trait Process: Send {
    fn id(&self) -> ProcessId;
    fn name(&self) -> &str;
    fn state(&self) -> ProcessState;

    /// Called once during build, before the first tick.
    fn init(&mut self) {}

    /// Called once per tick. The runtime guarantees all peer Processes have
    /// completed tick `t-1` before any starts tick `t` (fixed-step barrier).
    fn step(&mut self, tick: u64);

    /// Called once during teardown.
    fn shutdown(&mut self) {}
}

/// Marker for a Process *behavior* (CPU / SIMD / future Loihi backends).
/// In Lava this is `AbstractProcessModel`. We keep it minimal: the Compiler
/// (when added) will pick one ProcessModel per Process based on a `RunCfg`.
pub trait ProcessModel: Process {
    /// Identifier for the backend (e.g. `"cpu"`, `"simd"`, `"loihi"`).
    fn backend(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Counter {
        id: ProcessId,
        state: ProcessState,
        v: Var<u64>,
    }

    impl Process for Counter {
        fn id(&self) -> ProcessId {
            self.id
        }
        fn name(&self) -> &str {
            "counter"
        }
        fn state(&self) -> ProcessState {
            self.state
        }
        fn step(&mut self, _tick: u64) {
            self.v.set(self.v.get() + 1);
        }
    }

    #[test]
    fn step_mutates_var() {
        let mut c = Counter {
            id: ProcessId::fresh(),
            state: ProcessState::Running,
            v: Var::new("v", 0),
        };
        c.step(0);
        c.step(1);
        assert_eq!(*c.v.get(), 2);
    }
}
