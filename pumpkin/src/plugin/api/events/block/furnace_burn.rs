use std::sync::Arc;

use pumpkin_data::{Block, item_stack::ItemStack};
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;

use crate::world::World;

use super::BlockEvent;

/// Fired when a furnace consumes a piece of fuel.
#[cancellable]
#[derive(Event, Clone)]
pub struct FurnaceBurnEvent {
    /// The furnace block.
    pub block: &'static Block,

    /// The position of the furnace block.
    pub block_position: BlockPos,

    /// The world containing the furnace.
    pub world: Arc<World>,

    /// The fuel item being consumed.
    pub fuel: ItemStack,

    /// The number of ticks the fuel will burn for.
    pub burn_time: u16,
}

impl FurnaceBurnEvent {
    /// Creates a new [`FurnaceBurnEvent`].
    #[must_use]
    pub const fn new(
        block: &'static Block,
        block_position: BlockPos,
        world: Arc<World>,
        fuel: ItemStack,
        burn_time: u16,
    ) -> Self {
        Self {
            block,
            block_position,
            world,
            fuel,
            burn_time,
            cancelled: false,
        }
    }
}

impl BlockEvent for FurnaceBurnEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}
