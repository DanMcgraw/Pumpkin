use std::sync::Arc;

use pumpkin_data::{Block, item_stack::ItemStack};
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;

use crate::world::World;

use super::BlockEvent;

/// Fired when a brewing stand brews potions.
#[cancellable]
#[derive(Event, Clone)]
pub struct BrewEvent {
    /// The brewing stand block.
    pub block: &'static Block,

    /// The position of the brewing stand block.
    pub block_position: BlockPos,

    /// The world containing the brewing stand.
    pub world: Arc<World>,

    /// The ingredient item used for brewing.
    pub ingredient: ItemStack,

    /// The potion stacks currently in the stand before brewing.
    pub potions: Vec<ItemStack>,

    /// The remaining fuel (blaze powder) ticks.
    pub fuel: i32,
}

impl BrewEvent {
    /// Creates a new [`BrewEvent`].
    #[must_use]
    pub const fn new(
        block: &'static Block,
        block_position: BlockPos,
        world: Arc<World>,
        ingredient: ItemStack,
        potions: Vec<ItemStack>,
        fuel: i32,
    ) -> Self {
        Self {
            block,
            block_position,
            world,
            ingredient,
            potions,
            fuel,
            cancelled: false,
        }
    }
}

impl BlockEvent for BrewEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}
