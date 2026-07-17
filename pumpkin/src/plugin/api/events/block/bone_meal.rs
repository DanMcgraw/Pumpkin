use std::sync::Arc;

use pumpkin_data::BlockStateId;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::{Hand, math::position::BlockPos};

use crate::{
    entity::player::Player, plugin::api::transaction::TransactionContext, world::World,
};

#[cancellable]
#[derive(Event, Clone)]
pub struct BoneMealApplyPrepareEvent {
    pub transaction: TransactionContext,
    pub player: Arc<Player>,
    pub world: Arc<World>,
    pub position: BlockPos,
    pub hand: Hand,
    pub state_before: BlockStateId,
}

#[derive(Event, Clone)]
pub struct BoneMealApplyCompleteEvent {
    pub transaction: TransactionContext,
    pub player: Arc<Player>,
    pub world: Arc<World>,
    pub position: BlockPos,
    pub state_before: BlockStateId,
    pub state_after: BlockStateId,
    pub consumed_count: u8,
    pub growth_occurred: bool,
}
