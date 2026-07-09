use std::sync::Arc;

use pumpkin_data::{Block, BlockStateId};
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;

use crate::world::World;

use super::BlockEvent;

/// An event that occurs when a block is formed by natural or environmental causes.
///
/// This covers transformations such as lava meeting water, lava igniting fire, coral dying,
/// farmland hydrating or dehydrating, and sponge absorption. Weather-driven ice/snow formation
/// and concrete powder solidification are not yet implemented in Pumpkin.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockFormEvent {
    /// The world where the block is forming.
    pub world: Arc<World>,

    /// The original block before formation.
    pub block: &'static Block,

    /// The position of the block being formed.
    pub block_pos: BlockPos,

    /// The new block that will replace the original block.
    pub new_block: &'static Block,

    /// The new block state id to apply.
    pub new_state_id: BlockStateId,
}

impl BlockFormEvent {
    /// Creates a new `BlockFormEvent`.
    ///
    /// # Arguments
    /// - `world`: The world where the formation is happening.
    /// - `block`: The original block before formation.
    /// - `block_pos`: The position of the block being formed.
    /// - `new_block`: The new block that will replace the original block.
    /// - `new_state_id`: The new block state id that will be applied if not cancelled.
    ///
    /// # Returns
    /// A new `BlockFormEvent`.
    #[must_use]
    pub const fn new(
        world: Arc<World>,
        block: &'static Block,
        block_pos: BlockPos,
        new_block: &'static Block,
        new_state_id: BlockStateId,
    ) -> Self {
        Self {
            world,
            block,
            block_pos,
            new_block,
            new_state_id,
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockFormEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}

/// Fires a [`BlockFormEvent`] for the state transition at `pos` and returns the
/// state id that should actually be applied.
///
/// Returns `None` if the event was cancelled, in which case callers should not
/// change the block state.
pub async fn fire_block_form(
    world: &Arc<World>,
    pos: BlockPos,
    new_state_id: BlockStateId,
) -> Option<BlockStateId> {
    let Some(server) = world.server.upgrade() else {
        return Some(new_state_id);
    };

    let (old_block, _old_state_id) = world.get_block_and_state_id(&pos);
    let new_block = Block::from_state_id(new_state_id);
    let event = BlockFormEvent::new(world.clone(), old_block, pos, new_block, new_state_id);
    let event = server.plugin_manager.fire(event).await;
    if event.cancelled {
        None
    } else {
        Some(event.new_state_id)
    }
}
