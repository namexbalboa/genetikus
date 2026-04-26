//! Per-tick phase loop.
//!
//! Lava: `SPK -> PRE_MGMT -> LRN -> POST_MGMT`. We rename for genetics
//! semantics; the four-phase shape is preserved because the same barrier
//! discipline is what guarantees deterministic outputs across orderings.

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Phase {
    /// "Spike" equivalent — propagate signals between Processes.
    Replication,
    /// Pre-management — apply stochastic mutation operators.
    Mutation,
    /// Learning equivalent — apply selection / fitness updates.
    Selection,
    /// Post-management — collect telemetry, emit reports.
    Reporting,
}

impl Phase {
    /// Iterator over the four phases in canonical tick order.
    pub fn cycle() -> [Phase; 4] {
        [
            Phase::Replication,
            Phase::Mutation,
            Phase::Selection,
            Phase::Reporting,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_is_canonical_order() {
        assert_eq!(
            Phase::cycle(),
            [
                Phase::Replication,
                Phase::Mutation,
                Phase::Selection,
                Phase::Reporting,
            ]
        );
    }
}
