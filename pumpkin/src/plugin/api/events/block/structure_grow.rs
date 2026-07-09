use std::sync::Arc;

use pumpkin_data::{Block, placed_feature::PlacedFeature};
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;

use crate::world::World;

use super::BlockEvent;

/// An event that occurs before a structure grows from a sapling or mushroom.
///
/// Examples include oak/birch/spruce/jungle/acacia/dark-oak/cherry trees and huge mushrooms.
/// Cancelling the event prevents the structure from generating.
#[cancellable]
#[derive(Event, Clone)]
pub struct StructureGrowEvent {
    /// The world where the structure is growing.
    pub world: Arc<World>,

    /// The block at the structure origin (e.g., the sapling or mushroom).
    pub block: &'static Block,

    /// The position of the origin block.
    pub block_pos: BlockPos,

    /// The placed feature that will be used to generate the structure.
    pub placed_feature: PlacedFeature,
}

impl StructureGrowEvent {
    /// Creates a new `StructureGrowEvent`.
    ///
    /// # Arguments
    /// - `world`: The world where the structure is growing.
    /// - `block`: The block at the structure origin.
    /// - `block_pos`: The position of the origin block.
    /// - `placed_feature`: The placed feature that will generate the structure.
    ///
    /// # Returns
    /// A new `StructureGrowEvent`.
    #[must_use]
    pub const fn new(
        world: Arc<World>,
        block: &'static Block,
        block_pos: BlockPos,
        placed_feature: PlacedFeature,
    ) -> Self {
        Self {
            world,
            block,
            block_pos,
            placed_feature,
            cancelled: false,
        }
    }
}

impl BlockEvent for StructureGrowEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}
