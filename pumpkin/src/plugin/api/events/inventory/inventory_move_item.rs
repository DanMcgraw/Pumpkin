use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::inventory::Inventory;

/// Fired when an item is moved between two inventories by an automated
/// mechanism such as a hopper, dropper, or dispenser.
#[cancellable]
#[derive(Event, Clone)]
pub struct InventoryMoveItemEvent {
    /// The item stack being moved.
    pub item: ItemStack,

    /// The inventory the item is moved from, if known.
    pub source: Option<Arc<dyn Inventory>>,

    /// The inventory the item is moved to, if known.
    pub destination: Option<Arc<dyn Inventory>>,

    /// The block position of the source inventory, if it is a block entity.
    pub source_pos: Option<BlockPos>,

    /// The block position of the destination inventory, if it is a block entity.
    pub destination_pos: Option<BlockPos>,
}

impl InventoryMoveItemEvent {
    /// Creates a new [`InventoryMoveItemEvent`].
    #[must_use]
    pub fn new(
        item: ItemStack,
        source: Option<Arc<dyn Inventory>>,
        destination: Option<Arc<dyn Inventory>>,
        source_pos: Option<BlockPos>,
        destination_pos: Option<BlockPos>,
    ) -> Self {
        Self {
            item,
            source,
            destination,
            source_pos,
            destination_pos,
            cancelled: false,
        }
    }
}
