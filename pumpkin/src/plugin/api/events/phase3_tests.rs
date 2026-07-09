#[cfg(test)]
mod tests {
    use std::sync::{Arc, Weak};

    use pumpkin_config::world::LevelConfig;
    use pumpkin_data::{
        Block, damage::DamageType, dimension::Dimension, entity::EntityType, item_stack::ItemStack,
    };
    use pumpkin_util::{
        math::{position::BlockPos, vector3::Vector3},
        world_seed::Seed,
    };
    use pumpkin_world::level::Level;
    use tempfile::tempdir;

    use crate::{
        block::registry::BlockRegistry,
        entity::{Entity, EntityBase},
        plugin::{
            Cancellable,
            api::events::entity::{
                entity_damage::EntityDamageEvent,
                entity_damage_by_entity::EntityDamageByEntityEvent, entity_death::EntityDeathEvent,
                projectile_hit::ProjectileHitEvent, projectile_launch::ProjectileLaunchEvent,
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

    fn test_entity(world: Arc<World>) -> Arc<dyn EntityBase> {
        Arc::new(Entity::new(
            world,
            Vector3::new(0.0, 64.0, 0.0),
            &EntityType::PIG,
        ))
    }

    #[tokio::test]
    async fn entity_damage_event_is_cancellable_and_mutable() {
        let world = test_world();
        let entity = test_entity(world);
        let mut event = EntityDamageEvent::new(entity, DamageType::ARROW, 5.0, 5.0);
        assert!(!event.cancelled());
        event.damage = 3.0;
        event.final_damage = 2.5;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.damage, 3.0);
        assert_eq!(event.final_damage, 2.5);
    }

    #[tokio::test]
    async fn entity_damage_by_entity_event_is_cancellable_and_mutable() {
        let world = test_world();
        let victim = test_entity(world.clone());
        let damager = test_entity(world.clone());
        let attacker = test_entity(world);
        let mut event = EntityDamageByEntityEvent::new(
            victim,
            damager,
            Some(attacker),
            DamageType::ARROW,
            5.0,
            5.0,
        );
        assert!(!event.cancelled());
        event.damage = 1.0;
        event.final_damage = 0.0;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.damage, 1.0);
        assert_eq!(event.final_damage, 0.0);
    }

    #[tokio::test]
    async fn entity_death_event_drops_and_experience_are_mutable() {
        let world = test_world();
        let entity = test_entity(world.clone());
        let killer = test_entity(world);
        let mut event = EntityDeathEvent::new(
            entity,
            DamageType::ARROW,
            Some(killer),
            vec![ItemStack::EMPTY.clone()],
            5,
        );
        event.drops.clear();
        event.dropped_exp = 10;
        assert!(event.drops.is_empty());
        assert_eq!(event.dropped_exp, 10);
    }

    #[tokio::test]
    async fn projectile_launch_event_is_cancellable() {
        let world = test_world();
        let projectile = test_entity(world.clone());
        let shooter = test_entity(world);
        let mut event = ProjectileLaunchEvent::new(projectile, Some(shooter));
        assert!(!event.cancelled());
        event.set_cancelled(true);
        assert!(event.cancelled());
    }

    #[tokio::test]
    async fn projectile_hit_event_is_cancellable() {
        let world = test_world();
        let projectile = test_entity(world.clone());
        let hit_entity = test_entity(world);
        let mut event = ProjectileHitEvent::new(
            projectile,
            Some(hit_entity),
            Some(&Block::STONE),
            Some(BlockPos(Vector3::new(1, 64, 2))),
        );
        assert!(!event.cancelled());
        event.set_cancelled(true);
        assert!(event.cancelled());
    }
}
