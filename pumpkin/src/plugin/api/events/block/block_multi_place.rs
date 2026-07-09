use std::sync::Arc;

use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;

use crate::entity::player::Player;

use super::BlockEvent;

/// An event that occurs when a single item placement creates multiple blocks.
///
/// This is fired for multi-block items such as beds, doors, tall seagrass, and dripleaf,
/// where the primary block placement triggers one or more secondary block placements.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockMultiPlaceEvent {
    /// The player placing the multi-block item.
    pub player: Arc<Player>,

    /// The block being placed at the primary position.
    pub block_placed: &'static Block,

    /// The block the item was placed against.
    pub block_placed_against: &'static Block,

    /// The primary placement position.
    pub primary_pos: BlockPos,

    /// The secondary positions that will be set as part of this placement.
    pub affected_positions: Vec<BlockPos>,
}

impl BlockMultiPlaceEvent {
    /// Creates a new `BlockMultiPlaceEvent`.
    ///
    /// # Arguments
    /// - `player`: The player placing the multi-block item.
    /// - `block_placed`: The block being placed at the primary position.
    /// - `block_placed_against`: The block the item was placed against.
    /// - `primary_pos`: The primary placement position.
    /// - `affected_positions`: The secondary positions that will be set.
    ///
    /// # Returns
    /// A new `BlockMultiPlaceEvent`.
    #[must_use]
    pub const fn new(
        player: Arc<Player>,
        block_placed: &'static Block,
        block_placed_against: &'static Block,
        primary_pos: BlockPos,
        affected_positions: Vec<BlockPos>,
    ) -> Self {
        Self {
            player,
            block_placed,
            block_placed_against,
            primary_pos,
            affected_positions,
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockMultiPlaceEvent {
    fn get_block(&self) -> &Block {
        self.block_placed
    }
}
