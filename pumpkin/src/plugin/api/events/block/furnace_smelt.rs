use std::sync::Arc;

use pumpkin_data::{Block, item_stack::ItemStack};
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;

use crate::world::World;

use super::BlockEvent;

/// Fired when a furnace smelts an input item into a result item.
#[cancellable]
#[derive(Event, Clone)]
pub struct FurnaceSmeltEvent {
    /// The furnace block.
    pub block: &'static Block,

    /// The position of the furnace block.
    pub block_position: BlockPos,

    /// The world containing the furnace.
    pub world: Arc<World>,

    /// The input item being smelted.
    pub input: ItemStack,

    /// The fuel item currently burning.
    pub fuel: ItemStack,

    /// The output item that will be produced.
    pub output: ItemStack,
}

impl FurnaceSmeltEvent {
    /// Creates a new [`FurnaceSmeltEvent`].
    #[must_use]
    pub const fn new(
        block: &'static Block,
        block_position: BlockPos,
        world: Arc<World>,
        input: ItemStack,
        fuel: ItemStack,
        output: ItemStack,
    ) -> Self {
        Self {
            block,
            block_position,
            world,
            input,
            fuel,
            output,
            cancelled: false,
        }
    }
}

impl BlockEvent for FurnaceSmeltEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}
