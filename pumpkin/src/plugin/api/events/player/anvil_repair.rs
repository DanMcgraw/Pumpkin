use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};

use crate::entity::player::Player;
use crate::plugin::api::transaction::TransactionContext;

use super::PlayerEvent;

/// Fired before an anvil output is taken and its costs are consumed.
#[cancellable]
#[derive(Event, Clone)]
pub struct AnvilRepairEvent {
    pub transaction: TransactionContext,
    pub screen_sync_id: u8,
    pub player: Arc<Player>,
    pub input_first: ItemStack,
    pub input_second: ItemStack,
    pub output: ItemStack,
    pub level_cost: i16,
    pub material_cost: u8,
}

/// Fired after an anvil output was delivered and all costs were consumed.
#[derive(Event, Clone)]
pub struct AnvilCompleteEvent {
    pub transaction: TransactionContext,
    pub screen_sync_id: u8,
    pub player: Arc<Player>,
    pub input_first: ItemStack,
    pub input_second: ItemStack,
    pub output: ItemStack,
    pub level_cost: i16,
    pub material_cost: u8,
}

impl PlayerEvent for AnvilCompleteEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}

impl PlayerEvent for AnvilRepairEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
