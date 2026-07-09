use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use std::sync::Arc;

use crate::world::World;

use super::BlockEvent;

/// Fired when a piston is about to extend.
///
/// The event fires after the piston has calculated which blocks will be pushed
/// and which blocks will be broken, but before any blocks are moved.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockPistonExtendEvent {
    /// The world where the piston resides.
    pub world: Arc<World>,

    /// The position of the piston base.
    pub piston_pos: BlockPos,

    /// The piston base block.
    pub piston_block: &'static Block,

    /// The direction in which the piston extends.
    pub direction: Vector3<i32>,

    /// The blocks that will be pushed by the piston.
    pub moved_blocks: Vec<BlockPos>,

    /// The blocks that will be broken by the piston extension.
    pub broken_blocks: Vec<BlockPos>,
}

impl BlockPistonExtendEvent {
    /// Creates a new [`BlockPistonExtendEvent`].
    ///
    /// # Arguments
    /// - `world`: The world where the piston resides.
    /// - `piston_pos`: The position of the piston base.
    /// - `piston_block`: The piston base block.
    /// - `direction`: The direction in which the piston extends.
    /// - `moved_blocks`: The blocks that will be pushed.
    /// - `broken_blocks`: The blocks that will be broken.
    ///
    /// # Returns
    /// A new `BlockPistonExtendEvent`.
    #[must_use]
    pub fn new(
        world: Arc<World>,
        piston_pos: BlockPos,
        piston_block: &'static Block,
        direction: Vector3<i32>,
        moved_blocks: Vec<BlockPos>,
        broken_blocks: Vec<BlockPos>,
    ) -> Self {
        Self {
            world,
            piston_pos,
            piston_block,
            direction,
            moved_blocks,
            broken_blocks,
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockPistonExtendEvent {
    fn get_block(&self) -> &Block {
        self.piston_block
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
        Arc::new(World::load(
            level,
            level_info,
            Dimension::OVERWORLD,
            Arc::new(BlockRegistry::default()),
            Weak::new(),
        ))
    }

    #[tokio::test]
    async fn block_piston_extend_event_can_be_cancelled() {
        let world = test_world();
        let mut event = BlockPistonExtendEvent::new(
            world,
            BlockPos::new(0, 64, 0),
            &Block::PISTON,
            pumpkin_data::BlockDirection::East.to_offset(),
            vec![BlockPos::new(1, 64, 0)],
            vec![],
        );
        assert!(!event.cancelled());
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.get_block(), &Block::PISTON);
    }
}
