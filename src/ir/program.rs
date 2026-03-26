//! Top-level IR program container produced by `IrBuilder::lower`.

use super::instruction::IrNode;

/// Root container produced by [`IrBuilder::lower`].
///
/// Holds a flat list of top-level [`IrNode`]s (function definitions, global
/// assignments, top-level expressions) in source order.  All constant folding
/// and dead-branch elimination have already been applied.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProgramIr {
    /// Top-level IR nodes in source order.
    pub nodes: Vec<IrNode>,
    /// Number of constant-folding reductions applied during lowering.
    pub fold_count: usize,
    /// Number of dead branches eliminated during lowering.
    pub dead_branch_count: usize,
}

impl ProgramIr {
    pub fn new() -> Self {
        Self::default()
    }

    /// Total number of top-level IR nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
