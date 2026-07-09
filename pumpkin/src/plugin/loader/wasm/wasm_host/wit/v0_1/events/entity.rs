use crate::plugin::{
    Cancellable,
    entity::{
        chunk_entity_load::ChunkEntityLoadEvent, chunk_entity_unload::ChunkEntityUnloadEvent,
        entity_block_form::EntityBlockFormEvent, entity_change_block::EntityChangeBlockEvent,
        entity_remove::EntityRemoveEvent, entity_spawn::EntitySpawnEvent,
    },
    loader::wasm::wasm_host::{
        state::PluginHostState,
        wit::v0_1::{
            events::{
                ToFromWasmEvent, consume_entity, consume_world, from_wasm_block_position,
                to_wasm_block_position,
            },
            pumpkin::plugin::event::{
                ChunkEntityLoadEventData, ChunkEntityUnloadEventData, EntityBlockFormEventData,
                EntityChangeBlockEventData, EntityRemoveEventData, EntitySpawnEventData, Event,
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
