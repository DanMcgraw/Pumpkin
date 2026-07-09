use pumpkin_data::BlockStateId;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

use crate::entity::EntityBase;

/// An event that occurs when an entity changes an existing block to another block or air.
///
/// Examples include an enderman picking up a block, a sheep eating grass, an ender dragon
/// breaking blocks on collision, or a splash potion extinguishing fire.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityChangeBlockEvent {
    /// The entity changing the block.
    pub entity: Arc<dyn EntityBase>,

    /// The position of the block being changed.
    pub block_pos: BlockPos,

    /// The original block state id.
    pub old_state_id: BlockStateId,

    /// The new block state id that will be applied if not cancelled.
    pub new_state_id: BlockStateId,
}

impl EntityChangeBlockEvent {
    /// Creates a new `EntityChangeBlockEvent`.
    ///
    /// # Arguments
    /// - `entity`: The entity changing the block.
    /// - `block_pos`: The position of the block being changed.
    /// - `old_state_id`: The original block state id.
    /// - `new_state_id`: The new block state id that will be applied if not cancelled.
    ///
    /// # Returns
    /// A new `EntityChangeBlockEvent`.
    #[must_use]
    pub const fn new(
        entity: Arc<dyn EntityBase>,
        block_pos: BlockPos,
        old_state_id: BlockStateId,
        new_state_id: BlockStateId,
    ) -> Self {
        Self {
            entity,
            block_pos,
            old_state_id,
            new_state_id,
            cancelled: false,
        }
    }
}
