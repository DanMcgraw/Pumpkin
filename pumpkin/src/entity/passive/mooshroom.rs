use std::sync::{Arc, Weak};

use pumpkin_data::{entity::EntityType, item::Item, item_stack::ItemStack};

use crate::entity::{
    Entity, EntityBaseFuture, NBTStorage,
    ai::goal::{
        look_around::RandomLookAroundGoal, look_at_entity::LookAtEntityGoal, swim::SwimGoal,
        wander_around::WanderAroundGoal,
    },
    mob::{Mob, MobEntity},
    player::Player,
};
use crate::plugin::api::events::entity::entity_product::{
    AnimalProductCollectCompleteEvent, AnimalProductKind, replace_collected_container,
};

/// Represents a Mooshroom, a fungal variant of cows that can be milked for mushroom stew.
///
/// Wiki: <https://minecraft.wiki/w/Mooshroom>
pub struct MooshroomEntity {
    pub mob_entity: MobEntity,
}

impl MooshroomEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let mooshroom = Self { mob_entity };
        let mob_arc = Arc::new(mooshroom);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        {
            let mut goal_selector = mob_arc.mob_entity.goals_selector.lock().unwrap();

            goal_selector.add_goal(0, Box::new(SwimGoal::default()));
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

impl NBTStorage for MooshroomEntity {}

impl Mob for MooshroomEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn mob_interact<'a>(
        &'a self,
        player: &'a Arc<Player>,
        item_stack: &'a mut ItemStack,
    ) -> EntityBaseFuture<'a, bool> {
        Box::pin(async move {
            let entity = &self.mob_entity.living_entity.entity;
            if item_stack.item != &Item::BOWL
                || entity.age.load(std::sync::atomic::Ordering::Relaxed) < 0
            {
                return false;
            }
            let Some(target) = entity.world.load().get_entity_by_id(entity.entity_id) else {
                return false;
            };
            let tool_before = item_stack.clone();
            let output = ItemStack::new(1, &Item::MUSHROOM_STEW);
            replace_collected_container(player, item_stack, output.clone()).await;
            AnimalProductCollectCompleteEvent::fire(
                Arc::clone(player),
                target,
                AnimalProductKind::MushroomStew,
                tool_before,
                item_stack.clone(),
                vec![output],
            )
            .await;
            true
        })
    }
}
