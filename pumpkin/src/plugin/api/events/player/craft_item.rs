use std::sync::Arc;

use crate::entity::player::Player;
use pumpkin_data::{item_stack::ItemStack, screen::WindowType};
use pumpkin_macros::{Event, cancellable};

use super::PlayerEvent;

/// Fired when a player takes an item from a crafting result slot.
#[cancellable]
#[derive(Event, Clone)]
pub struct CraftItemEvent {
    /// The player crafting the item.
    pub player: Arc<Player>,

    /// The crafted result item stack.
    pub result: ItemStack,

    /// The window type of the crafting screen, if known.
    pub window_type: Option<WindowType>,
}

impl CraftItemEvent {
    /// Creates a new [`CraftItemEvent`].
    #[must_use]
    pub const fn new(
        player: Arc<Player>,
        result: ItemStack,
        window_type: Option<WindowType>,
    ) -> Self {
        Self {
            player,
            result,
            window_type,
            cancelled: false,
        }
    }
}

impl PlayerEvent for CraftItemEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
