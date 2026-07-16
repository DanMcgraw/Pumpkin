use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};

use crate::entity::player::Player;

use super::PlayerEvent;

/// Fired before an enchanting-table transaction consumes levels or lapis.
#[cancellable]
#[derive(Event, Clone)]
pub struct EnchantItemEvent {
    pub player: Arc<Player>,
    pub item: ItemStack,
    pub slot: usize,
    pub level_cost: i32,
    pub lapis_cost: u8,
    pub enchantments: Vec<(i32, i32)>,
}

impl PlayerEvent for EnchantItemEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
