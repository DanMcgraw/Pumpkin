use std::sync::{
    Arc, Weak,
    atomic::{AtomicU8, Ordering},
};

use pumpkin_data::{
    entity::EntityType, item::Item, meta_data_type::MetaDataType, tracked_data::TrackedData,
};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_protocol::java::client::play::Metadata;

use crate::entity::{
    Entity, EntityBase, EntityBaseFuture, NBTStorage, NbtFuture,
    ai::goal::{
        breed::BreedGoal, eat_grass::EatGrassGoal, escape_danger::EscapeDangerGoal,
        follow_parent::FollowParentGoal, look_around::RandomLookAroundGoal,
        look_at_entity::LookAtEntityGoal, swim::SwimGoal, tempt::TemptGoal,
        wander_around::WanderAroundGoal,
    },
    mob::{Mob, MobEntity},
    player::Player,
};

use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::particle::Particle;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;

use crate::plugin::api::events::entity::{
    entity_feed::{FeedOutcome, FeedPurpose, complete_feed, prepare_feed},
    entity_product::{AnimalProductCollectCompleteEvent, AnimalProductKind},
};

const TEMPT_ITEMS: &[&Item] = &[&Item::WHEAT];

pub struct SheepEntity {
    pub mob_entity: MobEntity,
    color_and_sheared: AtomicU8,
}

impl SheepEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let sheep = Self {
            mob_entity,
            color_and_sheared: AtomicU8::new(0),
        };
        let mob_arc = Arc::new(sheep);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        {
            let mut goal_selector = mob_arc.mob_entity.goals_selector.lock().unwrap();

            goal_selector.add_goal(0, Box::new(SwimGoal::default()));
            goal_selector.add_goal(1, EscapeDangerGoal::new(1.25));
            goal_selector.add_goal(2, BreedGoal::new(1.0));
            goal_selector.add_goal(3, Box::new(TemptGoal::new(1.1, TEMPT_ITEMS)));
            goal_selector.add_goal(4, Box::new(FollowParentGoal::new(1.1)));
            goal_selector.add_goal(5, Box::new(EatGrassGoal::default()));
            goal_selector.add_goal(6, Box::new(WanderAroundGoal::new(1.0)));
            goal_selector.add_goal(
                7,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 6.0),
            );
            goal_selector.add_goal(8, Box::new(RandomLookAroundGoal::default()));
        };

        mob_arc
    }

    fn get_packed_byte(&self) -> u8 {
        self.color_and_sheared.load(Ordering::Relaxed)
    }

    pub fn get_color(&self) -> u8 {
        self.get_packed_byte() & 0x0F
    }

    pub fn is_sheared(&self) -> bool {
        (self.get_packed_byte() & 0x10) != 0
    }

    fn set_packed_and_sync(&self, byte: u8) {
        self.color_and_sheared.store(byte, Ordering::Relaxed);
        self.mob_entity
            .living_entity
            .entity
            .send_meta_data(&[Metadata::new(
                TrackedData::WOOL_ID,
                MetaDataType::BYTE,
                byte as i8,
            )]);
    }

    pub fn set_color(&self, color: u8) {
        let byte = (self.get_packed_byte() & 0xF0) | (color & 0x0F);
        self.set_packed_and_sync(byte);
    }

    pub fn set_sheared(&self, sheared: bool) {
        let byte = if sheared {
            self.get_packed_byte() | 0x10
        } else {
            self.get_packed_byte() & !0x10
        };
        self.set_packed_and_sync(byte);
    }
}

fn wool_for_color(color: u8) -> &'static Item {
    match color {
        1 => &Item::ORANGE_WOOL,
        2 => &Item::MAGENTA_WOOL,
        3 => &Item::LIGHT_BLUE_WOOL,
        4 => &Item::YELLOW_WOOL,
        5 => &Item::LIME_WOOL,
        6 => &Item::PINK_WOOL,
        7 => &Item::GRAY_WOOL,
        8 => &Item::LIGHT_GRAY_WOOL,
        9 => &Item::CYAN_WOOL,
        10 => &Item::PURPLE_WOOL,
        11 => &Item::BLUE_WOOL,
        12 => &Item::BROWN_WOOL,
        13 => &Item::GREEN_WOOL,
        14 => &Item::RED_WOOL,
        15 => &Item::BLACK_WOOL,
        _ => &Item::WHITE_WOOL,
    }
}

impl NBTStorage for SheepEntity {
    fn write_nbt<'a>(&'a self, nbt: &'a mut NbtCompound) -> NbtFuture<'a, ()> {
        Box::pin(async {
            self.mob_entity.living_entity.entity.write_nbt(nbt).await;
            nbt.put_bool("Sheared", self.is_sheared());
            nbt.put_byte("Color", self.get_color() as i8);
        })
    }

    fn read_nbt_non_mut<'a>(&'a self, nbt: &'a NbtCompound) -> NbtFuture<'a, ()> {
        Box::pin(async {
            self.mob_entity
                .living_entity
                .entity
                .read_nbt_non_mut(nbt)
                .await;
            let sheared = nbt.get_bool("Sheared").unwrap_or(false);
            let color = nbt.get_byte("Color").unwrap_or(0) as u8;
            let byte = (color & 0x0F) | if sheared { 0x10 } else { 0 };
            self.color_and_sheared.store(byte, Ordering::Relaxed);
        })
    }
}

impl Mob for SheepEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn on_eating_grass(&self) -> EntityBaseFuture<'_, ()> {
        Box::pin(async {
            self.set_sheared(false);
        })
    }

    fn mob_interact<'a>(
        &'a self,
        player: &'a Arc<Player>,
        item_stack: &'a mut ItemStack,
    ) -> EntityBaseFuture<'a, bool> {
        Box::pin(async move {
            let entity = &self.mob_entity.living_entity.entity;
            if item_stack.item == &Item::SHEARS
                && !self.is_sheared()
                && entity.age.load(std::sync::atomic::Ordering::Relaxed) >= 0
                && let Some(target) = entity.world.load().get_entity_by_id(entity.entity_id)
            {
                let tool_before = item_stack.clone();
                self.set_sheared(true);
                let output = ItemStack::new(
                    rand::rng().random_range(1..=3),
                    wool_for_color(self.get_color()),
                );
                entity.world.load().drop_stack(&entity.block_pos.load(), output.clone()).await;
                if player.gamemode.load() != pumpkin_util::GameMode::Creative {
                    let _ = item_stack.damage_item(1);
                }
                AnimalProductCollectCompleteEvent::fire(
                    Arc::clone(player),
                    target,
                    AnimalProductKind::Shear,
                    tool_before,
                    item_stack.clone(),
                    vec![output],
                )
                .await;
                return true;
            }
            let is_food = TEMPT_ITEMS.iter().any(|i| i.id == item_stack.item.id);
            if is_food && self.is_breeding_ready() && !self.is_in_love() {
                let Some(feed) = prepare_feed(
                    entity,
                    player,
                    item_stack,
                    FeedPurpose::EnterLoveMode,
                )
                .await else {
                    return true;
                };
                item_stack.decrement_unless_creative(player.gamemode.load(), feed.consume_count);

                self.mob_entity
                    .set_love_ticks(600, Some(player.gameprofile.id));
                let world = entity.world.load();
                let pos = entity.pos.load();

                world.spawn_particle(
                    pos + Vector3::new(0.0, f64::from(entity.height()), 0.0),
                    Vector3::new(0.5, 0.5, 0.5),
                    1.0,
                    7,
                    Particle::Heart,
                );
                world.play_sound(
                    Sound::EntitySheepAmbient,
                    SoundCategory::Neutral,
                    &entity.pos.load(),
                );
                complete_feed(feed, FeedOutcome::EnteredLoveMode).await;
                return true;
            }
            false
        })
    }
}
