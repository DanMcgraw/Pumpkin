use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};

use crate::entity::player::Player;
use crate::plugin::api::transaction::TransactionContext;

use super::PlayerEvent;

/// Fired after Pumpkin computes an anvil result and before it is shown.
#[cancellable]
#[derive(Event, Clone)]
pub struct AnvilPrepareEvent {
    pub transaction: TransactionContext,
    pub screen_sync_id: u8,
    pub player: Arc<Player>,
    pub input_first: ItemStack,
    pub input_second: ItemStack,
    pub output: ItemStack,
    pub level_cost: i16,
    pub material_cost: u8,
}

impl PlayerEvent for AnvilPrepareEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
