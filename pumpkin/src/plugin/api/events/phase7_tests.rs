#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, Weak, atomic::AtomicBool};

    use pumpkin_config::world::LevelConfig;
    use pumpkin_data::{chunk::ChunkStatus, dimension::Dimension};
    use pumpkin_util::{math::vector2::Vector2, world_seed::Seed};
    use pumpkin_world::{
        chunk::{ChunkData, ChunkLight, ChunkSections},
        level::Level,
        tick::scheduler::ChunkTickScheduler,
    };
    use tempfile::tempdir;

    use crate::{
        block::registry::BlockRegistry,
        plugin::{
            Cancellable,
            api::events::world::{ChunkUnloadEvent, WorldLoadEvent, WorldUnloadEvent},
        },
        world::{LevelData, World},
    };

    fn test_world() -> Arc<World> {
        let temp_dir = tempdir().unwrap();
        let level = Level::from_root_folder(
            &LevelConfig::default(),
            temp_dir.path().to_path_buf(),
            0,
            Dimension::OVERWORLD,
            None,
        );
        let level_info = Arc::new(arc_swap::ArcSwap::new(Arc::new(LevelData::default(Seed(
            0,
        )))));
        World::load(
            &level,
            level_info,
            Dimension::OVERWORLD,
            Arc::new(BlockRegistry::default()),
            Weak::new(),
        )
    }

    fn test_chunk() -> Arc<ChunkData> {
        Arc::new(ChunkData {
            section: ChunkSections::new(24, -64),
            heightmap: Mutex::default(),
            x: 0,
            z: 0,
            block_ticks: ChunkTickScheduler::default(),
            fluid_ticks: ChunkTickScheduler::default(),
            pending_block_entities: Mutex::default(),
            light_engine: Mutex::new(ChunkLight::default()),
            light_populated: AtomicBool::new(false),
            status: ChunkStatus::Empty,
            blending_data: None,
            dirty: AtomicBool::new(false),
        })
    }

    #[tokio::test]
    async fn world_load_event_carries_world() {
        let world = test_world();
        let event = WorldLoadEvent::new(world.clone());
        assert!(Arc::ptr_eq(&event.world, &world));
    }

    #[tokio::test]
    async fn world_unload_event_is_cancellable() {
        let world = test_world();
        let mut event = WorldUnloadEvent::new(world);
        assert!(!event.cancelled());
        event.set_cancelled(true);
        assert!(event.cancelled());
    }

    #[tokio::test]
    async fn chunk_unload_event_is_cancellable_and_mutable() {
        let world = test_world();
        let chunk = test_chunk();
        let pos = Vector2::new(1, 2);
        let mut event = ChunkUnloadEvent::new(world, chunk.clone(), pos);
        assert!(!event.cancelled());
        assert_eq!(event.pos, pos);
        assert!(Arc::ptr_eq(&event.chunk, &chunk));
        event.pos = Vector2::new(3, 4);
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.pos, Vector2::new(3, 4));
    }
}
