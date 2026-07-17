use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::Event;
use pumpkin_util::Hand;

use crate::{entity::player::Player, plugin::api::transaction::TransactionContext};

/// Immutable notification after consumption, remainders, and effects commit.
#[derive(Event, Clone)]
pub struct PlayerItemUseCompleteEvent {
    pub transaction: TransactionContext,
    pub player: Arc<Player>,
    pub item_before: ItemStack,
    pub item_after: ItemStack,
    pub consumed_count: u8,
    pub hand: Hand,
    pub nutrition: u8,
    pub saturation: f32,
    pub applied_potion_effects: bool,
}
