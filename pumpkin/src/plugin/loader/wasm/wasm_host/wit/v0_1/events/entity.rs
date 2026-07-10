use crate::plugin::{
    Cancellable,
    entity::{
        chunk_entity_load::ChunkEntityLoadEvent, chunk_entity_unload::ChunkEntityUnloadEvent,
        entity_block_form::EntityBlockFormEvent, entity_change_block::EntityChangeBlockEvent,
        entity_damage::EntityDamageEvent, entity_damage_by_entity::EntityDamageByEntityEvent,
        entity_death::EntityDeathEvent, entity_remove::EntityRemoveEvent,
        entity_spawn::EntitySpawnEvent,
    },
    loader::wasm::wasm_host::{
        state::PluginHostState,
        wit::v0_1::{
            events::{
                ToFromWasmEvent, consume_entity, consume_item_stack, consume_world,
                from_wasm_block_position, from_wasm_damage_type, to_wasm_block_position,
                to_wasm_damage_type, to_wasm_item_stack,
            },
            pumpkin::plugin::event::{
                ChunkEntityLoadEventData, ChunkEntityUnloadEventData, EntityBlockFormEventData,
                EntityChangeBlockEventData, EntityDamageByEntityEventData, EntityDamageEventData,
                EntityDeathEventData, EntityRemoveEventData, EntitySpawnEventData, Event,
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
