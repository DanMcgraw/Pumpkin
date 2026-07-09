use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};

use crate::entity::{EntityBase, player::Player};

/// Fired when a player shoots a bow or crossbow.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityShootBowEvent {
    /// The player shooting the bow.
    pub player: Arc<Player>,

    /// The projectile being fired.
    pub projectile: Arc<dyn EntityBase>,

    /// The bow or crossbow item used.
    pub bow: ItemStack,

    /// The consumable item being shot (e.g. an arrow).
    pub consumable: Option<ItemStack>,

    /// The charge / force of the shot (0.0 - 1.0).
    pub force: f32,
}

impl EntityShootBowEvent {
    /// Creates a new [`EntityShootBowEvent`].
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        player: Arc<Player>,
        projectile: Arc<dyn EntityBase>,
        bow: ItemStack,
        consumable: Option<ItemStack>,
        force: f32,
    ) -> Self {
        Self {
            player,
            projectile,
            bow,
            consumable,
            force,
            cancelled: false,
        }
    }
}
