use std::sync::Arc;

use crate::entity::player::Player;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};

use super::PlayerEvent;

/// Fired when a player drops an item.
#[cancellable]
#[derive(Event, Clone)]
pub struct PlayerDropItemEvent {
    /// The player dropping the item.
    pub player: Arc<Player>,

    /// The item stack being dropped.
    pub item: ItemStack,
}

impl PlayerDropItemEvent {
    /// Creates a new [`PlayerDropItemEvent`].
    #[must_use]
    pub fn new(player: &Arc<Player>, item: ItemStack) -> Self {
        Self {
            player: Arc::clone(player),
            item,
            cancelled: false,
        }
    }
}

impl PlayerEvent for PlayerDropItemEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
