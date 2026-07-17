use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::Hand;

use crate::{
    entity::{Entity, EntityBase, player::Player},
    plugin::api::transaction::TransactionContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedPurpose {
    Heal,
    EnterLoveMode,
    TameAttempt,
    AgeUp,
    Trust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedOutcome {
    Healed,
    EnteredLoveMode,
    TameSucceeded,
    TameFailed,
    AgedUp,
    TrustIncreased,
}

#[cancellable]
#[derive(Event, Clone)]
pub struct EntityFeedPrepareEvent {
    pub transaction: TransactionContext,
    pub player: Arc<Player>,
    pub target: Arc<dyn EntityBase>,
    pub hand: Hand,
    pub item: ItemStack,
    pub purpose: FeedPurpose,
    pub consume_count: u8,
}

#[derive(Event, Clone)]
pub struct EntityFeedCompleteEvent {
    pub transaction: TransactionContext,
    pub player: Arc<Player>,
    pub target: Arc<dyn EntityBase>,
    pub item_before: ItemStack,
    pub consumed_count: u8,
    pub outcome: FeedOutcome,
}

pub(crate) struct FeedTransaction {
    transaction: TransactionContext,
    player: Arc<Player>,
    target: Arc<dyn EntityBase>,
    item_before: ItemStack,
    pub consume_count: u8,
}

pub(crate) async fn prepare_feed(
    entity: &Entity,
    player: &Arc<Player>,
    item: &ItemStack,
    purpose: FeedPurpose,
) -> Option<FeedTransaction> {
    let world = entity.world.load();
    let server = world.server.upgrade()?;
    let target = world.get_entity_by_id(entity.entity_id)?;
    let transaction = TransactionContext::new(
        player
            .tick_counter
            .load(std::sync::atomic::Ordering::Relaxed),
    );
    let event = server
        .plugin_manager
        .fire(EntityFeedPrepareEvent {
            transaction,
            player: Arc::clone(player),
            target: Arc::clone(&target),
            hand: Hand::Right,
            item: item.clone(),
            purpose,
            consume_count: 1,
            cancelled: false,
        })
        .await;
    if event.cancelled || event.consume_count > item.item_count {
        return None;
    }
    Some(FeedTransaction {
        transaction,
        player: Arc::clone(player),
        target,
        item_before: item.clone(),
        consume_count: event.consume_count,
    })
}

pub(crate) async fn complete_feed(transaction: FeedTransaction, outcome: FeedOutcome) {
    if let Some(server) = transaction
        .target
        .get_entity()
        .world
        .load()
        .server
        .upgrade()
    {
        server
            .plugin_manager
            .fire(EntityFeedCompleteEvent {
                transaction: transaction.transaction,
                player: transaction.player,
                target: transaction.target,
                item_before: transaction.item_before,
                consumed_count: transaction.consume_count,
                outcome,
            })
            .await;
    }
}
