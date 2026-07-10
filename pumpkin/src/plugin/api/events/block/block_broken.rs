use std::sync::Arc;

use pumpkin_data::{Block, BlockDirection, BlockStateId};
use pumpkin_macros::Event;
use pumpkin_util::math::position::BlockPos;

use crate::{entity::player::Player, world::World};

use super::BlockEvent;

/// Fired after a block has been successfully replaced and its drops processed.
#[derive(Event, Clone)]
pub struct BlockBrokenEvent {
    /// The world in which the block was broken.
    pub world: Arc<World>,
    /// The player who broke the block, if applicable.
    pub player: Option<Arc<Player>>,
    /// The block type before replacement.
    pub block: &'static Block,
    /// The exact block state before replacement.
    pub block_state_id: BlockStateId,
    /// The state installed in place of the broken block.
    pub replacement_state_id: BlockStateId,
    /// The position of the broken block.
    pub block_position: BlockPos,
    /// The outward-facing side selected by the player, if known.
    pub face: Option<BlockDirection>,
    /// Whether normal item and experience drop processing was enabled.
    pub dropped_items: bool,
}

impl BlockEvent for BlockBrokenEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}
