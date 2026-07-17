use std::sync::Arc;

use uuid::Uuid;

use crate::entity::{EntityBase, ai::pathfinder::NavigatorGoal, mob::Mob, r#type::from_type};
use crate::{
    entity::experience_orb::ExperienceOrbEntity,
    plugin::api::{
        events::entity::entity_breed::{EntityBreedCompleteEvent, EntityBreedEvent},
        transaction::TransactionContext,
    },
};

use super::{Controls, Goal, GoalFuture};

pub struct BreedGoal {
    speed: f64,
    mate: Option<Arc<dyn EntityBase>>,
    timer: i32,
}

impl BreedGoal {
    #[must_use]
    pub fn new(speed: f64) -> Box<Self> {
        Box::new(Self {
            speed,
            mate: None,
            timer: 0,
        })
    }

    fn find_mate(mob: &dyn Mob) -> Option<Arc<dyn EntityBase>> {
        let mob_entity = mob.get_mob_entity();
        if !mob_entity.is_in_love() {
            return None;
        }

        let entity = mob.get_entity();
        let pos = entity.pos.load();
        let world = entity.world.load();
        let my_type = entity.entity_type;
        let my_uuid = entity.entity_uuid;

        let nearby = world.get_nearby_entities(pos, 8.0);
        let mut closest: Option<(f64, Arc<dyn EntityBase>)> = None;

        for candidate in nearby.values() {
            let c_entity = candidate.get_entity();
            if c_entity.entity_uuid == my_uuid {
                continue;
            }
            if c_entity.entity_type != my_type {
                continue;
            }
            if !candidate.is_in_love() || !candidate.is_breeding_ready() || candidate.is_panicking()
            {
                continue;
            }

            let dist = pos.squared_distance_to_vec(&c_entity.pos.load());
            match &closest {
                Some((best_dist, _)) if dist >= *best_dist => {}
                _ => closest = Some((dist, candidate.clone())),
            }
        }

        closest.map(|(_, e)| e)
    }

    async fn breed(mob: &dyn Mob, mate: &dyn EntityBase) {
        let mob_entity = mob.get_mob_entity();
        let entity = mob.get_entity();
        let world = entity.world.load();

        let breeder = mob_entity
            .breeder
            .load()
            .and_then(|uuid| world.get_player_by_uuid(uuid));

        let parent_pos = entity.pos.load();
        let mother = world
            .get_entity_by_uuid(entity.entity_uuid)
            .expect("breeding parent should exist");
        let father = world
            .get_entity_by_uuid(mate.get_entity().entity_uuid)
            .expect("breeding mate should exist");
        let initiated_tick = world.get_world_age().await.clamp(0, i64::from(i32::MAX)) as i32;
        let transaction = TransactionContext::new(initiated_tick);
        let experience = if let Some(server) = world.server.upgrade() {
            let event = server
                .plugin_manager
                .fire(EntityBreedEvent::new_with_transaction(
                    transaction,
                    mother.clone(),
                    father.clone(),
                    breeder.clone(),
                    entity.entity_type,
                    parent_pos,
                    0,
                ))
                .await;
            if event.cancelled {
                return;
            }
            event.experience.max(0)
        } else {
            0
        };

        mob_entity.reset_love_ticks();
        mob_entity
            .breeding_cooldown
            .store(6000, std::sync::atomic::Ordering::Relaxed);
        mate.reset_love();
        mate.set_breeding_cooldown(6000);

        if let Some(player) = breeder.clone() {
            player
                .increment_stat(
                    pumpkin_data::statistic::StatisticCategory::Custom,
                    pumpkin_data::statistic::CustomStatistic::AnimalsBred as i32,
                    1,
                )
                .await;
        }

        let baby = from_type(entity.entity_type, parent_pos, &world, Uuid::new_v4());
        baby.get_entity().set_age(-24000);
        world
            .spawn_entity_with_reason(baby.clone(), "breeding")
            .await;
        if experience > 0 {
            ExperienceOrbEntity::spawn(&world, parent_pos, experience as u32).await;
        }
        if let Some(server) = world.server.upgrade() {
            server
                .plugin_manager
                .fire(EntityBreedCompleteEvent {
                    transaction,
                    mother,
                    father,
                    breeder,
                    baby,
                    consumed_items: Vec::new(),
                    experience,
                })
                .await;
        }
    }
}

impl Goal for BreedGoal {
    fn can_start<'a>(&'a mut self, mob: &'a dyn Mob) -> GoalFuture<'a, bool> {
        Box::pin(async {
            let mob_entity = mob.get_mob_entity();
            if !mob_entity.is_breeding_ready() || !mob_entity.is_in_love() {
                return false;
            }

            self.mate = Self::find_mate(mob);
            self.mate.is_some()
        })
    }

    fn should_continue<'a>(&'a self, _mob: &'a dyn Mob) -> GoalFuture<'a, bool> {
        Box::pin(async {
            let Some(mate) = &self.mate else {
                return false;
            };

            if !mate.get_entity().is_alive() || mate.is_panicking() {
                return false;
            }

            mate.is_in_love() && self.timer < 60
        })
    }

    fn start<'a>(&'a mut self, _mob: &'a dyn Mob) -> GoalFuture<'a, ()> {
        Box::pin(async {
            self.timer = 0;
        })
    }

    fn stop<'a>(&'a mut self, mob: &'a dyn Mob) -> GoalFuture<'a, ()> {
        Box::pin(async {
            self.mate = None;
            self.timer = 0;
            let mut navigator = mob.get_mob_entity().navigator.lock().unwrap();
            navigator.stop();
        })
    }

    fn tick<'a>(&'a mut self, mob: &'a dyn Mob) -> GoalFuture<'a, ()> {
        Box::pin(async {
            let Some(mate) = &self.mate else {
                return;
            };

            let mob_entity = mob.get_mob_entity();
            let mate_pos = mate.get_entity().pos.load();

            {
                let mut look_control = mob_entity.look_control.lock().unwrap();
                look_control.look_at_entity(mob, mate);
            };

            let my_pos = mob.get_entity().pos.load();
            let dist_sq = my_pos.squared_distance_to_vec(&mate_pos);

            {
                let mut navigator = mob_entity.navigator.lock().unwrap();
                navigator.set_progress(NavigatorGoal::new(my_pos, mate_pos, self.speed));
            };

            self.timer += 1;

            if self.timer >= 60 && dist_sq < 9.0 {
                Self::breed(mob, mate.as_ref()).await;
            }
        })
    }

    fn should_run_every_tick(&self) -> bool {
        true
    }

    fn controls(&self) -> Controls {
        Controls::MOVE | Controls::LOOK
    }
}
