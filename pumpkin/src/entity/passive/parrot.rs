use std::sync::{Arc, Weak};

use pumpkin_data::{entity::EntityType, item::Item, particle::Particle};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::vector3::Vector3;
use uuid::Uuid;

use crate::entity::{
    Entity, EntityBaseFuture, NBTStorage, NbtFuture,
    ai::goal::{
        follow_owner::FollowOwnerGoal, look_around::RandomLookAroundGoal,
        look_at_entity::LookAtEntityGoal, swim::SwimGoal, wander_around::WanderAroundGoal,
    },
    mob::{Mob, MobEntity},
    passive::tameable::Tameable,
};
use crate::plugin::api::events::entity::entity_feed::{
    FeedOutcome, FeedPurpose, complete_feed, prepare_feed,
};
use crate::plugin::api::events::entity::entity_tame::EntityTameEvent;

const TAME_ITEMS: &[&Item] = &[
    &Item::WHEAT_SEEDS,
    &Item::MELON_SEEDS,
    &Item::PUMPKIN_SEEDS,
    &Item::BEETROOT_SEEDS,
];

/// Represents a Parrot, a passive flying mob that can mimic nearby mob sounds.
///
/// Wiki: <https://minecraft.wiki/w/Parrot>
pub struct ParrotEntity {
    pub mob_entity: MobEntity,
    pub tameable: Tameable,
}

impl ParrotEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let parrot = Self {
            mob_entity,
            tameable: Tameable::new(),
        };
        let mob_arc = Arc::new(parrot);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        {
            let mut goal_selector = mob_arc.mob_entity.goals_selector.lock().unwrap();

            goal_selector.add_goal(0, Box::new(SwimGoal::default()));
            goal_selector.add_goal(1, FollowOwnerGoal::new(1.0, 10.0, 2.0));
            goal_selector.add_goal(1, Box::new(WanderAroundGoal::new(1.0)));
            goal_selector.add_goal(
                2,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 6.0),
            );
            goal_selector.add_goal(3, Box::new(RandomLookAroundGoal::default()));
        };

        mob_arc
    }
}

impl NBTStorage for ParrotEntity {
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

impl Mob for ParrotEntity {
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
            if self.tameable.is_tamed() || !TAME_ITEMS.contains(&item_stack.item) {
                return false;
            }
            let entity = &self.mob_entity.living_entity.entity;
            let Some(feed) =
                prepare_feed(entity, player, item_stack, FeedPurpose::TameAttempt).await
            else {
                return true;
            };
            item_stack.decrement_unless_creative(player.gamemode.load(), feed.consume_count);
            if !rand::random_bool(1.0 / 10.0) {
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
