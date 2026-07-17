use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::Event;

use crate::entity::{EntityBase, player::Player};

use super::damage_attribution::DamageAttribution;

/// Immutable, exactly-once notification after a player-credited kill commits.
#[derive(Event, Clone)]
pub struct PlayerKillEntityEvent {
    pub player: Arc<Player>,
    pub victim: Arc<dyn EntityBase>,
    pub attribution: DamageAttribution,
    pub drops: Vec<ItemStack>,
    pub dropped_exp: i32,
}
