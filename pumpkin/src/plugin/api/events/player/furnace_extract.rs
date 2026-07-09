use std::sync::Arc;

use crate::entity::player::Player;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::Event;
use pumpkin_util::math::position::BlockPos;

use super::PlayerEvent;

/// Fired when a player takes an item from a furnace output slot.
#[derive(Event, Clone)]
pub struct FurnaceExtractEvent {
    /// The player extracting the item.
    pub player: Arc<Player>,

    /// The position of the furnace block.
    pub block_position: BlockPos,

    /// The item stack being extracted.
    pub item: ItemStack,

    /// The amount of experience awarded for the extraction.
    pub experience: i32,
}

impl FurnaceExtractEvent {
    /// Creates a new [`FurnaceExtractEvent`].
    #[must_use]
    pub const fn new(
        player: Arc<Player>,
        block_position: BlockPos,
        item: ItemStack,
        experience: i32,
    ) -> Self {
        Self {
            player,
            block_position,
            item,
            experience,
        }
    }
}

impl PlayerEvent for FurnaceExtractEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
