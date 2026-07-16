use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};

use crate::entity::player::Player;

use super::PlayerEvent;

/// Fired before an anvil output is taken and its costs are consumed.
#[cancellable]
#[derive(Event, Clone)]
pub struct AnvilRepairEvent {
    pub player: Arc<Player>,
    pub input_first: ItemStack,
    pub input_second: ItemStack,
    pub output: ItemStack,
    pub level_cost: i16,
    pub material_cost: u8,
}

impl PlayerEvent for AnvilRepairEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
