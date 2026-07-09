use std::sync::Arc;

use pumpkin_data::Block;
use pumpkin_data::BlockStateId;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;

use crate::world::World;

use super::BlockEvent;

/// An event that occurs when a block grows.
///
/// Fired during random-tick growth for crops, saplings, cactus, sugar cane, bamboo,
/// sweet berry bushes, nether wart, and melon/pumpkin stems. Bonemeal-based growth and
/// tree generation are not yet covered.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockGrowEvent {
    /// The world where growth is happening.
    pub world: Arc<World>,

    /// The original block before growth.
    pub old_block: &'static Block,

    /// The original block state id.
    pub old_state_id: BlockStateId,

    /// The new block targeted by growth.
    pub new_block: &'static Block,

    /// The new block state id to apply.
    pub new_state_id: BlockStateId,

    /// The position of the growing block.
    pub block_pos: BlockPos,
}

impl BlockGrowEvent {
    /// Creates a new `BlockGrowEvent`.
    ///
    /// # Arguments
    /// - `world`: The world where the growth is happening.
    /// - `old_block`: The original block before growth.
    /// - `old_state_id`: The original block state id.
    /// - `new_block`: The new block targeted by the growth.
    /// - `new_state_id`: The new block state id that will be applied if not cancelled.
    /// - `block_pos`: The block position where growth is happening.
    ///
    /// # Returns
    /// A new `BlockGrowEvent`.
    #[must_use]
    pub const fn new(
        world: Arc<World>,
        old_block: &'static Block,
        old_state_id: BlockStateId,
        new_block: &'static Block,
        new_state_id: BlockStateId,
        block_pos: BlockPos,
    ) -> Self {
        Self {
            world,
            old_block,
            old_state_id,
            new_block,
            new_state_id,
            block_pos,
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockGrowEvent {
    fn get_block(&self) -> &Block {
        self.old_block
    }
}

/// Fires a [`BlockGrowEvent`] for the state transition at `pos` and returns the
/// state id that should actually be applied.
///
/// Returns `None` if the event was cancelled, in which case callers should not
/// change the block state.
pub async fn fire_block_grow(
    world: &Arc<World>,
    pos: BlockPos,
    new_state_id: BlockStateId,
) -> Option<BlockStateId> {
    let Some(server) = world.server.upgrade() else {
        return Some(new_state_id);
    };

    let (old_block, old_state_id) = world.get_block_and_state_id(&pos);
    let new_block = Block::from_state_id(new_state_id);
    let event = BlockGrowEvent::new(
        world.clone(),
        old_block,
        old_state_id,
        new_block,
        new_state_id,
        pos,
    );
    let event = server.plugin_manager.fire(event).await;
    if event.cancelled {
        None
    } else {
        Some(event.new_state_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{block::registry::BlockRegistry, plugin::Cancellable};
    use pumpkin_config::world::LevelConfig;
    use pumpkin_data::dimension::Dimension;
    use pumpkin_util::world_seed::Seed;
    use pumpkin_world::level::Level;
    use std::sync::Weak;
    use tempfile::tempdir;

    fn test_world() -> Arc<World> {
        let temp_dir = tempdir().unwrap();
        let level = Level::from_root_folder(
            &LevelConfig::default(),
            temp_dir.path().to_path_buf(),
            0,
            Dimension::OVERWORLD,
            None,
        );
        let level_info = Arc::new(arc_swap::ArcSwap::new(Arc::new(
            crate::world::LevelData::default(Seed(0)),
        )));
        World::load(
            &level,
            level_info,
            Dimension::OVERWORLD,
            Arc::new(BlockRegistry::default()),
            Weak::new(),
        )
    }

    #[tokio::test]
    async fn fire_block_grow_passes_through_when_no_server() {
        let world = test_world();
        let pos = BlockPos::new(0, 64, 0);
        let new_state_id = Block::STONE.default_state.id;
        let result = fire_block_grow(&world, pos, new_state_id).await;
        assert_eq!(result, Some(new_state_id));
    }

    #[tokio::test]
    async fn block_grow_event_can_be_cancelled() {
        let world = test_world();
        let mut event = BlockGrowEvent::new(
            world,
            &Block::WHEAT,
            Block::WHEAT.default_state.id,
            &Block::WHEAT,
            Block::WHEAT.default_state.id,
            BlockPos::new(0, 64, 0),
        );
        assert!(!event.cancelled());
        event.set_cancelled(true);
        assert!(event.cancelled());
    }
}
