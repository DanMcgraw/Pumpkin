use std::sync::{Arc, Weak};

use pumpkin_data::entity::EntityType;
use pumpkin_data::{item::Item, particle::Particle};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::vector3::Vector3;
use uuid::Uuid;

use crate::entity::{
    Entity, EntityBaseFuture, NBTStorage, NbtFuture,
    ai::goal::{
        beg::BegGoal, breed::BreedGoal, escape_danger::EscapeDangerGoal,
        follow_owner::FollowOwnerGoal, follow_parent::FollowParentGoal,
        look_around::RandomLookAroundGoal, look_at_entity::LookAtEntityGoal, swim::SwimGoal,
        wander_around::WanderAroundGoal,
    },
    mob::{Mob, MobEntity},
    passive::tameable::Tameable,
};
use crate::plugin::api::events::entity::entity_tame::EntityTameEvent;
use crate::plugin::api::events::entity::entity_feed::{
    FeedOutcome, FeedPurpose, complete_feed, prepare_feed,
};

pub struct WolfEntity {
    pub mob_entity: MobEntity,
    pub tameable: Tameable,
}

impl WolfEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let wolf = Self {
            mob_entity,
            tameable: Tameable::new(),
        };
        let mob_arc = Arc::new(wolf);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        {
            let mut goal_selector = mob_arc.mob_entity.goals_selector.lock().unwrap();

            goal_selector.add_goal(1, Box::new(SwimGoal::default()));
            // goal_selector.add_goal(2, SitGoal::new(mob_arc.clone()));
            goal_selector.add_goal(4, EscapeDangerGoal::new(1.5));
            goal_selector.add_goal(5, BreedGoal::new(1.0));
            goal_selector.add_goal(6, FollowOwnerGoal::new(1.0, 10.0, 2.0));
            goal_selector.add_goal(8, Box::new(FollowParentGoal::new(1.1)));
            goal_selector.add_goal(9, BegGoal::new(8.0, &[&Item::BONE]));
            goal_selector.add_goal(
                10,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 8.0),
            );
            goal_selector.add_goal(10, Box::new(RandomLookAroundGoal::default()));
            goal_selector.add_goal(12, Box::new(WanderAroundGoal::new(1.0)));
        };

        mob_arc
    }
}

impl NBTStorage for WolfEntity {
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

impl Mob for WolfEntity {
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
            if self.tameable.is_tamed() || item_stack.item != &Item::BONE {
                return false;
            }
            let entity = &self.mob_entity.living_entity.entity;
            let Some(feed) = prepare_feed(entity, player, item_stack, FeedPurpose::TameAttempt).await
            else {
                return true;
            };
            item_stack.decrement_unless_creative(player.gamemode.load(), feed.consume_count);
            if !rand::random_bool(1.0 / 3.0) {
                complete_feed(feed, FeedOutcome::TameFailed).await;
                return true;
            }
            let world = entity.world.load();
            let Some(server) = world.server.upgrade() else {
                complete_feed(feed, FeedOutcome::TameFailed).await;
                return true;
            };
            let Some(animal) = world.get_entity_by_id(entity.entity_id) else {
                complete_feed(feed, FeedOutcome::TameFailed).await;
                return true;
            };
            let event = server
                .plugin_manager
                .fire(EntityTameEvent::new(animal, player.clone()))
                .await;
            if event.cancelled {
                complete_feed(feed, FeedOutcome::TameFailed).await;
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
            complete_feed(feed, FeedOutcome::TameSucceeded).await;
            true
        })
    }
}
