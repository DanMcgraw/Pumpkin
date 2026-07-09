use crate::entity::EntityBase;
use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

/// Common spawn reasons for [`EntitySpawnEvent`].
///
/// These are exposed as constants so that plugins can compare against the
/// `spawn_reason` string without hard-coding values.
pub mod spawn_reason {
    pub const NATURAL: &str = "natural";
    pub const SPAWN_EGG: &str = "spawn_egg";
    pub const SPAWNER: &str = "spawner";
    pub const NETHER_PORTAL: &str = "nether_portal";
    pub const BREEDING: &str = "breeding";
    pub const COMMAND: &str = "command";
    pub const PROJECTILE: &str = "projectile";
    pub const DISPENSE: &str = "dispense";
    pub const ITEM_USE: &str = "item_use";
    pub const EXPLOSION: &str = "explosion";
    pub const FISHING: &str = "fishing";
}

/// An event that occurs when an entity is spawned into a world.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntitySpawnEvent {
    /// The world the entity was spawned in.
    pub world: Arc<World>,
    /// The entity that was spawned.
    pub entity: Arc<dyn EntityBase>,
    /// Why the entity was spawned (e.g. `"natural"`, `"spawn_egg"`).
    pub spawn_reason: String,
}

impl EntitySpawnEvent {
    /// Creates a new `EntitySpawnEvent`.
    #[must_use]
    pub fn new(
        world: Arc<World>,
        entity: Arc<dyn EntityBase>,
        spawn_reason: impl Into<String>,
    ) -> Self {
        Self {
            world,
            entity,
            spawn_reason: spawn_reason.into(),
            cancelled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{block::registry::BlockRegistry, entity::Entity, plugin::Cancellable};
    use pumpkin_config::world::LevelConfig;
    use pumpkin_data::{dimension::Dimension, entity::EntityType};
    use pumpkin_util::{math::vector3::Vector3, world_seed::Seed};
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
    async fn entity_spawn_event_carries_spawn_reason() {
        let world = test_world();
        let entity = Arc::new(Entity::new(
            world.clone(),
            Vector3::new(0.0, 64.0, 0.0),
            &EntityType::PIG,
        ));
        let mut event = EntitySpawnEvent::new(world, entity, "spawn_egg");
        assert_eq!(event.spawn_reason, "spawn_egg");
        assert!(!event.cancelled());
        event.set_cancelled(true);
        assert!(event.cancelled());
    }

    #[test]
    fn spawn_reason_constants_are_stable() {
        assert_eq!(spawn_reason::NATURAL, "natural");
        assert_eq!(spawn_reason::SPAWN_EGG, "spawn_egg");
        assert_eq!(spawn_reason::SPAWNER, "spawner");
    }
}
