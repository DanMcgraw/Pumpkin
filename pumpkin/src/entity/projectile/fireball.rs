use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::{
    entity::{
        Entity, EntityBase, EntityBaseFuture, NBTStorage,
        projectile::{ProjectileHit, ThrownItemEntity},
    },
    plugin::api::events::entity::entity_combust_by_entity::EntityCombustByEntityEvent,
    server::Server,
};

const EXPLOSION_POWER: f32 = 1.0;
const GRAVITY: f64 = 0.0;

pub struct FireballEntity {
    pub thrown: ThrownItemEntity,
    pub explosion_power: f32,
}

impl FireballEntity {
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        let thrown = ThrownItemEntity {
            entity,
            owner_id: None,
            owner_uuid: None,
            collides_with_projectiles: false,
            has_hit: AtomicBool::new(false),
            gravity: GRAVITY,
        };

        Self {
            thrown,
            explosion_power: EXPLOSION_POWER,
        }
    }

    #[must_use]
    pub fn new_shot(entity: Entity, shooter: &Entity) -> Self {
        let thrown = ThrownItemEntity::new(entity, shooter, GRAVITY);
        Self {
            thrown,
            explosion_power: EXPLOSION_POWER,
        }
    }
}

impl NBTStorage for FireballEntity {}

impl EntityBase for FireballEntity {
    fn projectile_owner_uuid(&self) -> Option<uuid::Uuid> {
        self.thrown.owner_uuid
    }

    fn tick<'a>(
        &'a self,
        caller: &'a Arc<dyn EntityBase>,
        server: &'a Server,
    ) -> EntityBaseFuture<'a, ()> {
        Box::pin(async move { self.thrown.process_tick(caller, server).await })
    }

    fn get_entity(&self) -> &Entity {
        self.thrown.get_entity()
    }

    fn get_living_entity(&self) -> Option<&crate::entity::living::LivingEntity> {
        None
    }

    fn as_nbt_storage(&self) -> &dyn NBTStorage {
        self
    }

    fn cast_any(&self) -> &dyn std::any::Any {
        self
    }

    fn on_hit(&self, hit: ProjectileHit) -> EntityBaseFuture<'_, ()> {
        Box::pin(async move {
            let world = self.get_entity().world.load();

            // Handle entity/block hit
            if let ProjectileHit::Entity { ref entity, .. } = hit {
                let entity_clone = entity.clone();
                let world = self.get_entity().world.load();
                let combuster = self
                    .thrown
                    .owner_id
                    .and_then(|id| world.get_entity_by_id(id))
                    .unwrap_or_else(|| {
                        world
                            .get_entity_by_id(self.get_entity().entity_id)
                            .expect("fireball should exist")
                    });

                tokio::spawn(async move {
                    let server = world.server.upgrade().expect("server is gone");
                    let event =
                        EntityCombustByEntityEvent::new(entity_clone.clone(), combuster, 5.0);
                    let event = server.plugin_manager.fire(event).await;
                    if !event.cancelled {
                        entity_clone.get_entity().set_on_fire_for(5.0);
                    }
                    // Fireball does 6.0 damage in vanilla
                    let _ = entity_clone
                        .damage(
                            entity_clone.as_ref(),
                            6.0,
                            pumpkin_data::damage::DamageType::FIREBALL,
                        )
                        .await;
                });
            }

            let hit_pos = hit.hit_pos();
            // Explosion sets fire if mob griefing is enabled (assuming true for now)
            let source = world.get_entity_by_id(self.get_entity().entity_id);
            world.explode(hit_pos, self.explosion_power, source).await;
        })
    }
}
