use std::sync::Arc;

use crate::entity::player::Player;
use pumpkin_data::{item_stack::ItemStack, screen::WindowType};
use pumpkin_inventory::screen_handler::ClickType;
use pumpkin_macros::{Event, cancellable};

use super::PlayerEvent;
use crate::plugin::api::gui::PluginGuiEventContext;

/// Fired when a player drags items across multiple inventory slots.
#[cancellable]
#[derive(Event, Clone)]
pub struct InventoryDragEvent {
    /// The player performing the drag.
    pub player: Arc<Player>,

    /// The window type of the inventory being interacted with.
    pub window_type: Option<WindowType>,

    /// The slots affected by the drag.
    pub slots: Vec<i16>,

    /// The item stack on the cursor at the time of the drag.
    pub cursor: Option<ItemStack>,

    /// The click type of the drag (left/right/middle).
    pub click_type: ClickType,

    /// Ownership attribution for a plugin-created screen.
    pub plugin_gui: Option<PluginGuiEventContext>,
}

impl InventoryDragEvent {
    /// Creates a new [`InventoryDragEvent`].
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        player: &Arc<Player>,
        window_type: Option<WindowType>,
        slots: Vec<i16>,
        cursor: Option<ItemStack>,
        click_type: ClickType,
    ) -> Self {
        Self {
            player: Arc::clone(player),
            window_type,
            slots,
            cursor,
            click_type,
            plugin_gui: None,
            cancelled: false,
        }
    }

    #[must_use]
    pub fn with_plugin_gui(mut self, context: PluginGuiEventContext) -> Self {
        self.plugin_gui = Some(context);
        self
    }
}

impl PlayerEvent for InventoryDragEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
