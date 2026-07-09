use pumpkin_data::BlockStateId;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

use crate::entity::EntityBase;

/// An event that occurs when an entity creates a new block in previously empty space.
///
/// Examples include an enderman placing a carried block, a ghast fireball placing fire,
/// or a falling block entity landing and placing its block.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityBlockFormEvent {
    /// The entity creating the block.
    pub entity: Arc<dyn EntityBase>,

    /// The position where the block is being created.
    pub block_pos: BlockPos,

    /// The block state id that will be placed if not cancelled.
    pub new_state_id: BlockStateId,
}

impl EntityBlockFormEvent {
    /// Creates a new `EntityBlockFormEvent`.
    ///
    /// # Arguments
    /// - `entity`: The entity creating the block.
    /// - `block_pos`: The position where the block is being created.
    /// - `new_state_id`: The block state id that will be placed if not cancelled.
    ///
    /// # Returns
    /// A new `EntityBlockFormEvent`.
    #[must_use]
    pub const fn new(
        entity: Arc<dyn EntityBase>,
        block_pos: BlockPos,
        new_state_id: BlockStateId,
    ) -> Self {
        Self {
            entity,
            block_pos,
            new_state_id,
            cancelled: false,
        }
    }
}
