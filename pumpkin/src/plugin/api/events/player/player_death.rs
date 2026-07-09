use pumpkin_data::damage::DamageType;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::Event;
use std::sync::Arc;

use crate::entity::{EntityBase, player::Player};

use super::PlayerEvent;

/// Fired when a player dies.
///
/// This event is non-cancellable. Plugins can mutate drops, dropped XP,
/// `keep_inventory`, and `keep_level`.
#[derive(Event, Clone)]
pub struct PlayerDeathEvent {
    /// The player who died.
    pub player: Arc<Player>,

    /// The type of damage that caused the death.
    pub damage_type: DamageType,

    /// The entity credited with the kill, if any.
    pub killer: Option<Arc<dyn EntityBase>>,

    /// The item drops that will be spawned.
    pub drops: Vec<ItemStack>,

    /// The amount of experience that will be dropped.
    pub dropped_exp: i32,

    /// Whether the player should keep their inventory on death.
    pub keep_inventory: bool,

    /// Whether the player should keep their experience level on death.
    pub keep_level: bool,
}

impl PlayerDeathEvent {
    /// Creates a new [`PlayerDeathEvent`].
    #[must_use]
    pub fn new(
        player: Arc<Player>,
        damage_type: DamageType,
        killer: Option<Arc<dyn EntityBase>>,
        drops: Vec<ItemStack>,
        dropped_exp: i32,
    ) -> Self {
        Self {
            player,
            damage_type,
            killer,
            drops,
            dropped_exp,
            keep_inventory: false,
            keep_level: false,
        }
    }
}

impl PlayerEvent for PlayerDeathEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
