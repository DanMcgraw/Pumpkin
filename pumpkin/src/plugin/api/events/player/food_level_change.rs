use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

use crate::entity::player::Player;

use super::PlayerEvent;

/// Fired when a player's food level changes.
///
/// This event fires both when hunger decreases from exhaustion and when the
/// player eats food. Plugins can cancel the change or mutate the resulting
/// food level.
#[cancellable]
#[derive(Event, Clone)]
pub struct FoodLevelChangeEvent {
    /// The player whose food level is changing.
    pub player: Arc<Player>,

    /// The new food level that will be applied.
    pub food_level: u8,
}

impl FoodLevelChangeEvent {
    /// Creates a new [`FoodLevelChangeEvent`].
    #[must_use]
    pub const fn new(player: Arc<Player>, food_level: u8) -> Self {
        Self {
            player,
            food_level,
            cancelled: false,
        }
    }
}

impl PlayerEvent for FoodLevelChangeEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
