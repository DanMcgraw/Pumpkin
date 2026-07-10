use crate::plugin::{
    Cancellable,
    entity::{
        chunk_entity_load::ChunkEntityLoadEvent, chunk_entity_unload::ChunkEntityUnloadEvent,
        entity_block_form::EntityBlockFormEvent, entity_change_block::EntityChangeBlockEvent,
        entity_damage::EntityDamageEvent, entity_damage_by_entity::EntityDamageByEntityEvent,
        entity_death::EntityDeathEvent, entity_explode::EntityExplodeEvent,
        entity_remove::EntityRemoveEvent, entity_shoot_bow::EntityShootBowEvent,
        entity_spawn::EntitySpawnEvent, explosion_prime::ExplosionPrimeEvent,
        potion_splash::PotionSplashEvent, projectile_hit::ProjectileHitEvent,
        projectile_launch::ProjectileLaunchEvent,
    },
    loader::wasm::wasm_host::{
        state::PluginHostState,
        wit::v0_1::{
            events::{
                ToFromWasmEvent, consume_entity, consume_item_stack, consume_player, consume_world,
                from_wasm_block_name, from_wasm_block_position, from_wasm_damage_type,
                from_wasm_position, to_wasm_block_name, to_wasm_block_position,
                to_wasm_damage_type, to_wasm_item_stack, to_wasm_position,
            },
            pumpkin::plugin::event::{
                ChunkEntityLoadEventData, ChunkEntityUnloadEventData, EntityBlockFormEventData,
                EntityChangeBlockEventData, EntityDamageByEntityEventData, EntityDamageEventData,
                EntityDeathEventData, EntityExplodeEventData, EntityRemoveEventData,
                EntityShootBowEventData, EntitySpawnEventData, Event, ExplosionPrimeEventData,
                PotionSplashEventData, ProjectileHitEventData, ProjectileLaunchEventData,
            },
        },
    },
};
use pumpkin_data::BlockStateId;
use pumpkin_util::math::vector2::Vector2;

impl ToFromWasmEvent for EntitySpawnEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");
        let entity = state
            .add_entity(self.entity.clone())
            .expect("failed to add entity resource");

        Event::EntitySpawnEvent(EntitySpawnEventData {
            target_world,
            entity,
            cancelled: self.cancelled(),
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::EntitySpawnEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                entity: consume_entity(state, &data.entity),
                spawn_reason: String::from("natural"),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for EntityRemoveEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");
        let entity = state
            .add_entity(self.entity.clone())
            .expect("failed to add entity resource");

        Event::EntityRemoveEvent(EntityRemoveEventData {
            target_world,
            entity,
            cancelled: self.cancelled(),
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::EntityRemoveEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                entity: consume_entity(state, &data.entity),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for ChunkEntityLoadEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");
        let entity = state
            .add_entity(self.entity.clone())
            .expect("failed to add entity resource");

        Event::ChunkEntityLoadEvent(ChunkEntityLoadEventData {
            target_world,
            entity,
            chunk_x: self.chunk_pos.x,
            chunk_z: self.chunk_pos.y,
            cancelled: self.cancelled(),
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::ChunkEntityLoadEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                entity: consume_entity(state, &data.entity),
                chunk_pos: Vector2::new(data.chunk_x, data.chunk_z),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for ChunkEntityUnloadEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");
        let entity = state
            .add_entity(self.entity.clone())
            .expect("failed to add entity resource");

        Event::ChunkEntityUnloadEvent(ChunkEntityUnloadEventData {
            target_world,
            entity,
            chunk_x: self.chunk_pos.x,
            chunk_z: self.chunk_pos.y,
            cancelled: self.cancelled(),
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::ChunkEntityUnloadEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                entity: consume_entity(state, &data.entity),
                chunk_pos: Vector2::new(data.chunk_x, data.chunk_z),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for EntityBlockFormEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let entity = state
            .add_entity(self.entity.clone())
            .expect("failed to add entity resource");

        Event::EntityBlockFormEvent(EntityBlockFormEventData {
            entity,
            block_pos: to_wasm_block_position(self.block_pos),
            new_state_id: self.new_state_id.as_u16(),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::EntityBlockFormEvent(data) => Self {
                entity: consume_entity(state, &data.entity),
                block_pos: from_wasm_block_position(data.block_pos),
                new_state_id: BlockStateId::new_or_air(data.new_state_id),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for EntityChangeBlockEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let entity = state
            .add_entity(self.entity.clone())
            .expect("failed to add entity resource");

        Event::EntityChangeBlockEvent(EntityChangeBlockEventData {
            entity,
            block_pos: to_wasm_block_position(self.block_pos),
            old_state_id: self.old_state_id.as_u16(),
            new_state_id: self.new_state_id.as_u16(),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::EntityChangeBlockEvent(data) => Self {
                entity: consume_entity(state, &data.entity),
                block_pos: from_wasm_block_position(data.block_pos),
                old_state_id: BlockStateId::new_or_air(data.old_state_id),
                new_state_id: BlockStateId::new_or_air(data.new_state_id),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for EntityDamageEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let entity = state
            .add_entity(self.entity.clone())
            .expect("failed to add entity resource");

        Event::EntityDamageEvent(EntityDamageEventData {
            entity,
            damage_type: to_wasm_damage_type(self.damage_type),
            damage: self.damage,
            final_damage: self.final_damage,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::EntityDamageEvent(data) => Self {
                entity: consume_entity(state, &data.entity),
                damage_type: from_wasm_damage_type(&data.damage_type),
                damage: data.damage,
                final_damage: data.final_damage,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for EntityDamageByEntityEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let entity = state
            .add_entity(self.entity.clone())
            .expect("failed to add entity resource");
        let damager = state
            .add_entity(self.damager.clone())
            .expect("failed to add entity resource");
        let attacker = self.attacker.as_ref().map(|attacker| {
            state
                .add_entity(attacker.clone())
                .expect("failed to add entity resource")
        });

        Event::EntityDamageByEntityEvent(EntityDamageByEntityEventData {
            entity,
            damager,
            attacker,
            damage_type: to_wasm_damage_type(self.damage_type),
            damage: self.damage,
            final_damage: self.final_damage,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::EntityDamageByEntityEvent(data) => Self {
                entity: consume_entity(state, &data.entity),
                damager: consume_entity(state, &data.damager),
                attacker: data
                    .attacker
                    .as_ref()
                    .map(|attacker| consume_entity(state, attacker)),
                damage_type: from_wasm_damage_type(&data.damage_type),
                damage: data.damage,
                final_damage: data.final_damage,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for EntityDeathEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let entity = state
            .add_entity(self.entity.clone())
            .expect("failed to add entity resource");
        let killer = self.killer.as_ref().map(|killer| {
            state
                .add_entity(killer.clone())
                .expect("failed to add entity resource")
        });
        let drops = self
            .drops
            .iter()
            .map(|drop| to_wasm_item_stack(state, drop))
            .collect();

        Event::EntityDeathEvent(EntityDeathEventData {
            entity,
            damage_type: to_wasm_damage_type(self.damage_type),
            killer,
            drops,
            dropped_exp: self.dropped_exp,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::EntityDeathEvent(data) => Self {
                entity: consume_entity(state, &data.entity),
                damage_type: from_wasm_damage_type(&data.damage_type),
                killer: data
                    .killer
                    .as_ref()
                    .map(|killer| consume_entity(state, killer)),
                drops: data
                    .drops
                    .iter()
                    .map(|drop| consume_item_stack(state, drop))
                    .collect(),
                dropped_exp: data.dropped_exp,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for ProjectileLaunchEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let projectile = state
            .add_entity(self.projectile.clone())
            .expect("failed to add entity resource");
        let shooter = self.shooter.as_ref().map(|shooter| {
            state
                .add_entity(shooter.clone())
                .expect("failed to add entity resource")
        });

        Event::ProjectileLaunchEvent(ProjectileLaunchEventData {
            projectile,
            shooter,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::ProjectileLaunchEvent(data) => Self {
                projectile: consume_entity(state, &data.projectile),
                shooter: data
                    .shooter
                    .as_ref()
                    .map(|shooter| consume_entity(state, shooter)),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for ProjectileHitEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let projectile = state
            .add_entity(self.projectile.clone())
            .expect("failed to add entity resource");
        let hit_entity = self.hit_entity.as_ref().map(|hit_entity| {
            state
                .add_entity(hit_entity.clone())
                .expect("failed to add entity resource")
        });

        Event::ProjectileHitEvent(ProjectileHitEventData {
            projectile,
            hit_entity,
            hit_block: self.hit_block.map(to_wasm_block_name),
            hit_block_pos: self.hit_block_pos.map(to_wasm_block_position),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::ProjectileHitEvent(data) => Self {
                projectile: consume_entity(state, &data.projectile),
                hit_entity: data
                    .hit_entity
                    .as_ref()
                    .map(|hit_entity| consume_entity(state, hit_entity)),
                hit_block: data
                    .hit_block
                    .as_ref()
                    .map(|hit_block| from_wasm_block_name(hit_block)),
                hit_block_pos: data.hit_block_pos.map(from_wasm_block_position),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for PotionSplashEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let entity = state
            .add_entity(self.entity.clone())
            .expect("failed to add entity resource");
        let hit_entity = self.hit_entity.as_ref().map(|hit_entity| {
            state
                .add_entity(hit_entity.clone())
                .expect("failed to add entity resource")
        });
        let affected_entities = self
            .affected_entities
            .iter()
            .map(|entity| {
                state
                    .add_entity(entity.clone())
                    .expect("failed to add entity resource")
            })
            .collect();

        Event::PotionSplashEvent(PotionSplashEventData {
            entity,
            hit_pos: to_wasm_position(self.hit_pos),
            hit_block: self.hit_block.map(to_wasm_block_position),
            hit_entity,
            affected_entities,
            potion: to_wasm_item_stack(state, &self.potion),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::PotionSplashEvent(data) => Self {
                entity: consume_entity(state, &data.entity),
                hit_pos: from_wasm_position(data.hit_pos),
                hit_block: data.hit_block.map(from_wasm_block_position),
                hit_entity: data
                    .hit_entity
                    .as_ref()
                    .map(|hit_entity| consume_entity(state, hit_entity)),
                affected_entities: data
                    .affected_entities
                    .iter()
                    .map(|entity| consume_entity(state, entity))
                    .collect(),
                potion: consume_item_stack(state, &data.potion),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for EntityShootBowEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let player = state
            .add_player(self.player.clone())
            .expect("failed to add player resource");
        let projectile = state
            .add_entity(self.projectile.clone())
            .expect("failed to add entity resource");

        Event::EntityShootBowEvent(EntityShootBowEventData {
            player,
            projectile,
            bow: to_wasm_item_stack(state, &self.bow),
            consumable: self
                .consumable
                .as_ref()
                .map(|consumable| to_wasm_item_stack(state, consumable)),
            force: self.force,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::EntityShootBowEvent(data) => Self {
                player: consume_player(state, &data.player),
                projectile: consume_entity(state, &data.projectile),
                bow: consume_item_stack(state, &data.bow),
                consumable: data
                    .consumable
                    .as_ref()
                    .map(|consumable| consume_item_stack(state, consumable)),
                force: data.force,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for EntityExplodeEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let entity = self.entity.as_ref().map(|entity| {
            state
                .add_entity(entity.clone())
                .expect("failed to add entity resource")
        });

        Event::EntityExplodeEvent(EntityExplodeEventData {
            entity,
            location: to_wasm_position(self.location),
            affected_blocks: self
                .affected_blocks
                .iter()
                .copied()
                .map(to_wasm_block_position)
                .collect(),
            yield_value: self.yield_,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::EntityExplodeEvent(data) => Self {
                entity: data
                    .entity
                    .as_ref()
                    .map(|entity| consume_entity(state, entity)),
                location: from_wasm_position(data.location),
                affected_blocks: data
                    .affected_blocks
                    .into_iter()
                    .map(from_wasm_block_position)
                    .collect(),
                yield_: data.yield_value,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for ExplosionPrimeEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let entity = self.entity.as_ref().map(|entity| {
            state
                .add_entity(entity.clone())
                .expect("failed to add entity resource")
        });

        Event::ExplosionPrimeEvent(ExplosionPrimeEventData {
            entity,
            location: to_wasm_position(self.location),
            radius: self.radius,
            fire: self.fire,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::ExplosionPrimeEvent(data) => Self {
                entity: data
                    .entity
                    .as_ref()
                    .map(|entity| consume_entity(state, entity)),
                location: from_wasm_position(data.location),
                radius: data.radius,
                fire: data.fire,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}
