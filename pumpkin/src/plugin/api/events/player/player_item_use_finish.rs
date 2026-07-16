use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::Hand;

use crate::entity::player::Player;

use super::PlayerEvent;

/// Fired after a consumable finishes its use animation and before effects or inventory changes.
#[cancellable]
#[derive(Event, Clone)]
pub struct PlayerItemUseFinishEvent {
    pub player: Arc<Player>,
    pub item: ItemStack,
    pub hand: Hand,
    /// Nutrition applied by food; zero for non-food items.
    pub nutrition: u8,
    /// Saturation modifier applied by food; zero for non-food items.
    pub saturation: f32,
    /// Container/remainder placed into an emptied hand slot.
    pub result_item: ItemStack,
}

impl PlayerItemUseFinishEvent {
    #[must_use]
    pub fn new(
        player: Arc<Player>,
        item: ItemStack,
        hand: Hand,
        nutrition: u8,
        saturation: f32,
        result_item: ItemStack,
    ) -> Self {
        Self {
            player,
            item,
            hand,
            nutrition,
            saturation,
            result_item,
            cancelled: false,
        }
    }
}

impl PlayerEvent for PlayerItemUseFinishEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
