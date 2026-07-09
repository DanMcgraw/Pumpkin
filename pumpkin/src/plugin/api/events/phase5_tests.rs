#[cfg(test)]
mod tests {
    use std::sync::{Arc, Weak};

    use pumpkin_config::world::LevelConfig;
    use pumpkin_data::{entity::EntityType, item_stack::ItemStack};
    use pumpkin_util::{
        math::{position::BlockPos, vector3::Vector3},
        world_seed::Seed,
    };
    use pumpkin_world::level::Level;
    use tempfile::tempdir;

    use crate::{
        entity::{Entity, EntityBase},
        plugin::{
            Cancellable,
            api::events::entity::{
                entity_breed::EntityBreedEvent,
                entity_combust_by_entity::EntityCombustByEntityEvent,
                entity_explode::EntityExplodeEvent, entity_pickup_item::EntityPickupItemEvent,
                entity_target::EntityTargetEvent,
                entity_target_living_entity::EntityTargetLivingEntityEvent,
                entity_transform::EntityTransformEvent, explosion_prime::ExplosionPrimeEvent,
                potion_splash::PotionSplashEvent,
            },
        },
        world::{LevelData, World},
    };

    fn test_world() -> Arc<World> {
        let temp_dir = tempdir().unwrap();
        let level = Level::from_root_folder(
            &LevelConfig::default(),
            temp_dir.path().to_path_buf(),
            0,
            pumpkin_data::dimension::Dimension::OVERWORLD,
            None,
        );
        let level_info = Arc::new(arc_swap::ArcSwap::new(Arc::new(LevelData::default(Seed(
            0,
        )))));
        World::load(
            &level,
            level_info,
            pumpkin_data::dimension::Dimension::OVERWORLD,
            Arc::new(crate::block::registry::BlockRegistry::default()),
            Weak::new(),
        )
    }

    fn test_entity(world: Arc<World>) -> Arc<dyn EntityBase> {
        Arc::new(Entity::new(
            world,
            Vector3::new(0.0, 64.0, 0.0),
            &EntityType::PIG,
        ))
    }

    #[tokio::test]
    async fn entity_breed_event_is_cancellable_and_mutable() {
        let world = test_world();
        let mother = test_entity(world.clone());
        let father = test_entity(world);
        let mut event = EntityBreedEvent::new(
            mother,
            father,
            None,
            &EntityType::PIG,
            Vector3::new(0.0, 64.0, 0.0),
            0,
        );
        assert!(!event.cancelled());
        event.experience = 10;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.experience, 10);
    }

    #[tokio::test]
    async fn entity_target_event_is_cancellable_and_mutable() {
        let world = test_world();
        let mob = test_entity(world.clone());
        let target = test_entity(world);
        let mut event = EntityTargetEvent::new(mob, Some(target), Some("test"));
        assert!(!event.cancelled());
        event.reason = Some("changed");
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.reason, Some("changed"));
    }

    #[tokio::test]
    async fn entity_target_living_entity_event_is_cancellable() {
        let world = test_world();
        let mob = test_entity(world.clone());
        let target = test_entity(world);
        let mut event = EntityTargetLivingEntityEvent::new(mob, target);
        assert!(!event.cancelled());
        event.set_cancelled(true);
        assert!(event.cancelled());
    }

    #[tokio::test]
    async fn entity_pickup_item_event_is_cancellable_and_mutable() {
        let world = test_world();
        let entity = test_entity(world.clone());
        let item_entity = Arc::new(crate::entity::item::ItemEntity::new(
            Entity::new(world, Vector3::new(0.0, 64.0, 0.0), &EntityType::ITEM),
            ItemStack::new(1, &pumpkin_data::item::Item::STONE),
        ));
        let mut event = EntityPickupItemEvent::new(
            entity,
            item_entity,
            ItemStack::new(2, &pumpkin_data::item::Item::STONE),
            2,
        );
        assert!(!event.cancelled());
        event.amount = 5;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.amount, 5);
    }

    #[tokio::test]
    async fn entity_combust_by_entity_event_is_cancellable_and_mutable() {
        let world = test_world();
        let entity = test_entity(world.clone());
        let combuster = test_entity(world);
        let mut event = EntityCombustByEntityEvent::new(entity, combuster, 5.0);
        assert!(!event.cancelled());
        event.duration = 10.0;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert!((event.duration - 10.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn entity_explode_event_is_cancellable_and_mutable() {
        let world = test_world();
        let entity = test_entity(world);
        let mut event = EntityExplodeEvent::new(
            Some(entity),
            Vector3::new(0.0, 64.0, 0.0),
            vec![BlockPos(Vector3::new(0, 64, 0))],
            4.0,
        );
        assert!(!event.cancelled());
        event.affected_blocks.clear();
        event.yield_ = 2.0;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert!(event.affected_blocks.is_empty());
        assert!((event.yield_ - 2.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn explosion_prime_event_is_cancellable_and_mutable() {
        let world = test_world();
        let entity = test_entity(world);
        let mut event =
            ExplosionPrimeEvent::new(Some(entity), Vector3::new(0.0, 64.0, 0.0), 3.0, false);
        assert!(!event.cancelled());
        event.radius = 5.0;
        event.fire = true;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert!((event.radius - 5.0).abs() < f32::EPSILON);
        assert!(event.fire);
    }

    #[tokio::test]
    async fn entity_transform_event_is_cancellable_and_mutable() {
        let world = test_world();
        let entity = test_entity(world);
        let mut event = EntityTransformEvent::new(entity, &EntityType::ZOMBIE, Some("test"));
        assert!(!event.cancelled());
        event.reason = Some("changed");
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.reason, Some("changed"));
    }

    #[tokio::test]
    async fn potion_splash_event_is_cancellable_and_mutable() {
        let world = test_world();
        let entity = test_entity(world.clone());
        let affected = test_entity(world);
        let mut event = PotionSplashEvent::new(
            entity,
            Vector3::new(0.0, 64.0, 0.0),
            Some(BlockPos(Vector3::new(0, 64, 0))),
            None,
            vec![affected],
            ItemStack::new(1, &pumpkin_data::item::Item::SPLASH_POTION),
        );
        assert!(!event.cancelled());
        event.affected_entities.clear();
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert!(event.affected_entities.is_empty());
    }
}
