use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};

use crate::entity::player::Player;

use super::PlayerEvent;

/// Fired after Pumpkin computes a grindstone result and before it is shown.
#[cancellable]
#[derive(Event, Clone)]
pub struct GrindstoneEvent {
    pub player: Arc<Player>,
    pub input_top: ItemStack,
    pub input_bottom: ItemStack,
    pub output: ItemStack,
    pub experience: i32,
}

impl PlayerEvent for GrindstoneEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}

/// Fired before a grindstone output is taken and its costs are committed.
#[cancellable]
#[derive(Event, Clone)]
pub struct GrindstoneTakeEvent {
    pub player: Arc<Player>,
    pub input_top: ItemStack,
    pub input_bottom: ItemStack,
    pub output: ItemStack,
    pub experience: i32,
}

impl PlayerEvent for GrindstoneTakeEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
