use std::sync::{Arc, Weak};

use pumpkin_data::{entity::EntityType, item::Item, particle::Particle};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::vector3::Vector3;
use uuid::Uuid;

use crate::entity::{
    Entity, EntityBaseFuture, NBTStorage, NbtFuture,
    ai::goal::{
        breed::BreedGoal, escape_danger::EscapeDangerGoal, follow_owner::FollowOwnerGoal,
        follow_parent::FollowParentGoal, look_around::RandomLookAroundGoal,
        look_at_entity::LookAtEntityGoal, swim::SwimGoal, tempt::TemptGoal,
        wander_around::WanderAroundGoal,
    },
    mob::{Mob, MobEntity},
    passive::tameable::Tameable,
};
use crate::plugin::api::events::entity::entity_tame::EntityTameEvent;

const TEMPT_ITEMS: &[&Item] = &[&Item::COD, &Item::SALMON];

/// Represents a Cat, a passive mob that can be tamed and scares away creepers.
///
/// Wiki: <https://minecraft.wiki/w/Cat>
pub struct CatEntity {
    pub mob_entity: MobEntity,
    pub tameable: Tameable,
}

impl CatEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let cat = Self {
            mob_entity,
            tameable: Tameable::new(),
        };
        let mob_arc = Arc::new(cat);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        {
            let mut goal_selector = mob_arc.mob_entity.goals_selector.lock().unwrap();

            goal_selector.add_goal(1, Box::new(SwimGoal::default()));
            goal_selector.add_goal(1, EscapeDangerGoal::new(1.5));
            // goal_selector.add_goal(2, SitGoal::new(mob_arc.clone()));
            goal_selector.add_goal(4, Box::new(TemptGoal::new(0.6, TEMPT_ITEMS)));
            goal_selector.add_goal(5, BreedGoal::new(0.8));
            goal_selector.add_goal(7, FollowOwnerGoal::new(1.0, 10.0, 5.0));
            goal_selector.add_goal(9, Box::new(FollowParentGoal::new(0.8)));
            goal_selector.add_goal(11, Box::new(WanderAroundGoal::new(0.8)));
            goal_selector.add_goal(
                12,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 10.0),
            );
            goal_selector.add_goal(12, Box::new(RandomLookAroundGoal::default()));
        };

        mob_arc
    }
}

impl NBTStorage for CatEntity {
    fn write_nbt<'a>(&'a self, nbt: &'a mut NbtCompound) -> NbtFuture<'a, ()> {
        Box::pin(async move {
            self.mob_entity.living_entity.write_nbt(nbt).await;
            self.tameable.write_nbt(nbt);
        })
    }

    fn read_nbt_non_mut<'a>(&'a self, nbt: &'a NbtCompound) -> NbtFuture<'a, ()> {
        Box::pin(async move {
            self.mob_entity.living_entity.read_nbt_non_mut(nbt).await;
            self.tameable.read_nbt(nbt);
        })
    }
}

impl Mob for CatEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn get_owner_uuid(&self) -> Option<Uuid> {
        self.tameable.owner_uuid()
    }
    fn is_sitting(&self) -> bool {
        self.tameable.is_sitting()
    }
    fn is_tamed(&self) -> bool {
        self.tameable.is_tamed()
    }
    fn set_sitting(&self, sitting: bool) {
        self.tameable.set_sitting(sitting);
    }

    fn mob_interact<'a>(
        &'a self,
        player: &'a Arc<crate::entity::player::Player>,
        item_stack: &'a mut pumpkin_data::item_stack::ItemStack,
    ) -> EntityBaseFuture<'a, bool> {
        Box::pin(async move {
            if self.tameable.owner_uuid() == Some(player.gameprofile.id) && item_stack.is_empty() {
                self.tameable.set_sitting(!self.tameable.is_sitting());
                return true;
            }
            if self.tameable.is_tamed() || !TEMPT_ITEMS.contains(&item_stack.item) {
                return false;
            }
            item_stack.decrement_unless_creative(player.gamemode.load(), 1);
            if !rand::random_bool(1.0 / 3.0) {
                return true;
            }
            let entity = &self.mob_entity.living_entity.entity;
            let world = entity.world.load();
            let Some(server) = world.server.upgrade() else {
                return true;
            };
            let Some(animal) = world.get_entity_by_id(entity.entity_id) else {
                return true;
            };
            let event = server
                .plugin_manager
                .fire(EntityTameEvent::new(animal, player.clone()))
                .await;
            if event.cancelled {
                return true;
            }
            self.tameable.set_owner(Some(player.gameprofile.id));
            world.spawn_particle(
                entity.pos.load() + Vector3::new(0.0, f64::from(entity.height()), 0.0),
                Vector3::new(0.5, 0.5, 0.5),
                1.0,
                7,
                Particle::Heart,
            );
            true
        })
    }
}
