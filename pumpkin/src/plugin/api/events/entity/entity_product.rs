use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::Event;

use crate::{
    entity::{EntityBase, player::Player},
    plugin::api::transaction::TransactionContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimalProductKind {
    Milk,
    GoatMilk,
    Shear,
    MushroomStew,
    SuspiciousStew,
    Other,
}

#[derive(Event, Clone)]
pub struct AnimalProductCollectCompleteEvent {
    pub transaction: TransactionContext,
    pub player: Arc<Player>,
    pub target: Arc<dyn EntityBase>,
    pub kind: AnimalProductKind,
    pub tool_before: ItemStack,
    pub tool_after: ItemStack,
    pub outputs: Vec<ItemStack>,
}

impl AnimalProductCollectCompleteEvent {
    pub(crate) async fn fire(
        player: Arc<Player>,
        target: Arc<dyn EntityBase>,
        kind: AnimalProductKind,
        tool_before: ItemStack,
        tool_after: ItemStack,
        outputs: Vec<ItemStack>,
    ) {
        if let Some(server) = target.get_entity().world.load().server.upgrade() {
            server
                .plugin_manager
                .fire(Self {
                    transaction: TransactionContext::new(
                        player
                            .tick_counter
                            .load(std::sync::atomic::Ordering::Relaxed),
                    ),
                    player,
                    target,
                    kind,
                    tool_before,
                    tool_after,
                    outputs,
                })
                .await;
        }
    }
}

pub(crate) async fn replace_collected_container(
    player: &Arc<Player>,
    held: &mut ItemStack,
    output: ItemStack,
) {
    if player.gamemode.load() == pumpkin_util::GameMode::Creative {
        player
            .inventory
            .offer_or_drop_stack(output, player.as_ref())
            .await;
    } else if held.item_count == 1 {
        *held = output;
    } else {
        held.decrement(1);
        player
            .inventory
            .offer_or_drop_stack(output, player.as_ref())
            .await;
    }
}
