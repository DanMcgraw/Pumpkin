use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};

use crate::entity::{EntityBase, player::Player};

use super::PlayerEvent;

/// Fired before the server accepts an entity target for a melee attack.
#[cancellable]
#[derive(Event, Clone)]
pub struct PlayerAttackValidateEvent {
    pub player: Arc<Player>,
    pub target: Arc<dyn EntityBase>,
    pub weapon: ItemStack,
    pub maximum_reach: f64,
}

impl PlayerEvent for PlayerAttackValidateEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}

/// Fired after vanilla melee damage is calculated and before it is committed.
#[cancellable]
#[derive(Event, Clone)]
pub struct PlayerAttackDamageEvent {
    pub player: Arc<Player>,
    pub target: Arc<dyn EntityBase>,
    pub weapon: ItemStack,
    pub base_damage: f32,
    pub final_damage: f32,
    pub sweeping: bool,
    pub knockback_multiplier: f32,
}

impl PlayerEvent for PlayerAttackDamageEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
