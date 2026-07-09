use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

use crate::entity::player::Player;

use super::BlockEvent;

/// Fired when a player starts damaging (mining) a block.
///
/// This event is fired during `Status::StartedDigging` handling, after the
/// server has validated that the player can reach the block. Plugins can use
/// it to prepare abilities (e.g. Green Terra, Tree Feller) or toggle
/// instant-break behavior.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockDamageEvent {
    /// The player who started damaging the block.
    pub player: Arc<Player>,

    /// The block being damaged.
    pub block: &'static Block,

    /// The position of the block being damaged.
    pub block_position: BlockPos,

    /// Whether the block would break instantly from this damage.
    pub insta_break: bool,
}

impl BlockDamageEvent {
    /// Creates a new [`BlockDamageEvent`].
    ///
    /// # Arguments
    /// - `player`: The player damaging the block.
    /// - `block`: The block being damaged.
    /// - `block_position`: The position of the block being damaged.
    /// - `insta_break`: Whether the block would break instantly.
    ///
    /// # Returns
    /// A new `BlockDamageEvent`.
    #[must_use]
    pub const fn new(
        player: Arc<Player>,
        block: &'static Block,
        block_position: BlockPos,
        insta_break: bool,
    ) -> Self {
        Self {
            player,
            block,
            block_position,
            insta_break,
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockDamageEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}
