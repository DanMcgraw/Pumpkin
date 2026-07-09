use pumpkin_data::Block;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

use crate::entity::player::Player;

use super::BlockEvent;

/// Fired when a block drops items after being broken.
///
/// The event is fired after the block has been replaced by air (or water) but
/// before item entities are spawned. Plugins can mutate the item list, cancel
/// drops entirely, or add bonus drops.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockDropItemEvent {
    /// The player who broke the block.
    pub player: Arc<Player>,

    /// The block that was broken (the pre-break type).
    pub block: &'static Block,

    /// The position of the broken block.
    pub block_position: BlockPos,

    /// The item stacks that will be dropped.
    pub items: Vec<ItemStack>,
}

impl BlockDropItemEvent {
    /// Creates a new [`BlockDropItemEvent`].
    ///
    /// # Arguments
    /// - `player`: The player who broke the block.
    /// - `block`: The block that was broken.
    /// - `block_position`: The position of the broken block.
    /// - `items`: The item stacks to drop.
    ///
    /// # Returns
    /// A new `BlockDropItemEvent`.
    #[must_use]
    pub fn new(
        player: Arc<Player>,
        block: &'static Block,
        block_position: BlockPos,
        items: Vec<ItemStack>,
    ) -> Self {
        Self {
            player,
            block,
            block_position,
            items,
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockDropItemEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}
