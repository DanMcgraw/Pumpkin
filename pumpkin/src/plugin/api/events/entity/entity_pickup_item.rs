use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};

use crate::entity::{EntityBase, item::ItemEntity};

/// Fired when an entity picks up an item.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityPickupItemEvent {
    /// The entity picking up the item.
    pub entity: Arc<dyn EntityBase>,

    /// The item entity being picked up.
    pub item_entity: Arc<ItemEntity>,

    /// The item stack being picked up.
    pub item: ItemStack,

    /// The number of items that will be picked up.
    pub amount: u8,
}

impl EntityPickupItemEvent {
    /// Creates a new [`EntityPickupItemEvent`].
    #[must_use]
    pub fn new(
        entity: Arc<dyn EntityBase>,
        item_entity: Arc<ItemEntity>,
        item: ItemStack,
        amount: u8,
    ) -> Self {
        Self {
            entity,
            item_entity,
            item,
            amount,
            cancelled: false,
        }
    }
}
