use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};

use crate::entity::player::Player;
use crate::plugin::api::transaction::TransactionContext;

use super::PlayerEvent;

/// Fired for each server-computed enchanting-table offer before it is sent to the client.
#[cancellable]
#[derive(Event, Clone)]
pub struct EnchantItemGenerateEvent {
    pub transaction: TransactionContext,
    pub screen_sync_id: u8,
    pub player: Arc<Player>,
    pub item: ItemStack,
    pub slot: usize,
    pub bookshelf_count: i32,
    pub level_requirement: i32,
    pub enchantment_id: i32,
    pub enchantment_level: i32,
}

impl PlayerEvent for EnchantItemGenerateEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
