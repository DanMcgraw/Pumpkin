use std::sync::Arc;

use crate::entity::player::Player;
use pumpkin_data::screen::WindowType;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;

use super::PlayerEvent;
use crate::plugin::api::gui::PluginGuiEventContext;

/// Fired before a player opens an inventory screen.
#[cancellable]
#[derive(Event, Clone)]
pub struct InventoryOpenEvent {
    /// The player opening the inventory.
    pub player: Arc<Player>,

    /// The type of screen being opened.
    pub window_type: WindowType,

    /// The block position of the container, if it is a block-based screen.
    pub block_pos: Option<BlockPos>,

    /// Ownership attribution for a plugin-created screen.
    pub plugin_gui: Option<PluginGuiEventContext>,
}

impl InventoryOpenEvent {
    /// Creates a new [`InventoryOpenEvent`].
    #[must_use]
    pub fn new(player: &Arc<Player>, window_type: WindowType, block_pos: Option<BlockPos>) -> Self {
        Self {
            player: Arc::clone(player),
            window_type,
            block_pos,
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

impl PlayerEvent for InventoryOpenEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
