use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};

use crate::entity::player::Player;
use crate::plugin::api::transaction::TransactionContext;

use super::PlayerEvent;

/// Fired before an enchanting-table transaction consumes levels or lapis.
#[cancellable]
#[derive(Event, Clone)]
pub struct EnchantItemEvent {
    pub transaction: TransactionContext,
    pub screen_sync_id: u8,
    pub player: Arc<Player>,
    pub item: ItemStack,
    pub slot: usize,
    pub level_cost: i32,
    pub lapis_cost: u8,
    pub enchantments: Vec<(i32, i32)>,
}

/// Fired after the enchanted item, lapis, levels, and seed are committed.
#[derive(Event, Clone)]
pub struct EnchantItemCompleteEvent {
    pub transaction: TransactionContext,
    pub screen_sync_id: u8,
    pub player: Arc<Player>,
    pub item: ItemStack,
    pub slot: usize,
    pub level_cost: i32,
    pub lapis_cost: u8,
    pub enchantments: Vec<(i32, i32)>,
}

impl PlayerEvent for EnchantItemCompleteEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}

impl PlayerEvent for EnchantItemEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
