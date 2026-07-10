use pumpkin_data::{Block, BlockDirection};
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

use crate::entity::player::Player;

use super::BlockEvent;

/// An event that occurs when a block is broken.
///
/// This event contains information about the player breaking the block, the block itself,
/// the experience gained, and whether the block should drop items.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockBreakEvent {
    /// The player who is breaking the block, if applicable.
    pub player: Option<Arc<Player>>,

    /// The block that is being broken.
    pub block: &'static Block,

    /// The position of the block that is being broken.
    pub block_position: BlockPos,

    /// The outward-facing side of the block selected by the player, if known.
    pub face: Option<BlockDirection>,

    /// The amount of experience gained from breaking the block.
    pub exp: u32,

    /// A boolean indicating whether the block should drop items.
    pub drop: bool,
}

impl BlockBreakEvent {
    /// Creates a new instance of `BlockBreakEvent`.
    ///
    /// # Arguments
    /// - `player`: An optional reference to the player breaking the block.
    /// - `block`: The block that is being broken.
    /// - `block_position`: The position of the block that is being broken.
    /// - `face`: The outward-facing side selected by the player, if known.
    /// - `exp`: The amount of experience gained from breaking the block.
    /// - `drop`: A boolean indicating whether the block should drop items.
    ///
    /// # Returns
    /// A new instance of `BlockBreakEvent`.
    #[must_use]
    pub const fn new(
        player: Option<Arc<Player>>,
        block: &'static Block,
        block_position: BlockPos,
        face: Option<BlockDirection>,
        exp: u32,
        drop: bool,
    ) -> Self {
        Self {
            player,
            block,
            block_position,
            face,
            exp,
            drop,
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockBreakEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_optional_face() {
        let position = BlockPos::ZERO;
        let with_face = BlockBreakEvent::new(
            None,
            &Block::STONE,
            position,
            Some(BlockDirection::North),
            0,
            true,
        );
        assert_eq!(with_face.face, Some(BlockDirection::North));

        let without_face = BlockBreakEvent::new(None, &Block::STONE, position, None, 0, true);
        assert_eq!(without_face.face, None);
    }
}
