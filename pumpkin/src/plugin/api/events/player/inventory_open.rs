use std::sync::Arc;

use crate::entity::player::Player;
use pumpkin_data::screen::WindowType;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;

use super::PlayerEvent;

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
}

impl InventoryOpenEvent {
    /// Creates a new [`InventoryOpenEvent`].
    #[must_use]
    pub fn new(
        player: &Arc<Player>,
        window_type: WindowType,
        block_pos: Option<BlockPos>,
    ) -> Self {
        Self {
            player: Arc::clone(player),
            window_type,
            block_pos,
            cancelled: false,
        }
    }
}

impl PlayerEvent for InventoryOpenEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
