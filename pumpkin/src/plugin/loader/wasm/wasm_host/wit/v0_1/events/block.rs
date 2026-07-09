use pumpkin_data::BlockStateId;

use crate::plugin::{
    block::{
        block_break::BlockBreakEvent, block_burn::BlockBurnEvent,
        block_can_build::BlockCanBuildEvent, block_form::BlockFormEvent,
        block_grow::BlockGrowEvent, block_multi_place::BlockMultiPlaceEvent,
        block_place::BlockPlaceEvent, block_redstone::BlockRedstoneEvent,
        structure_grow::StructureGrowEvent,
    },
    loader::wasm::wasm_host::{
        state::PluginHostState,
        wit::v0_1::{
            events::{
                ToFromWasmEvent, consume_player, consume_world, from_wasm_block_name,
                from_wasm_block_position, to_wasm_block_name, to_wasm_block_position,
            },
            pumpkin::plugin::event::{
                BlockBreakEventData, BlockBurnEventData, BlockCanBuildEventData,
                BlockFormEventData, BlockGrowEventData, BlockMultiPlaceEventData,
                BlockPlaceEventData, BlockRedstoneEventData, Event, StructureGrowEventData,
            },
        },
    },
};

impl ToFromWasmEvent for BlockRedstoneEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");

        Event::BlockRedstoneEvent(BlockRedstoneEventData {
            target_world,
            state_id: self.block_state_id.as_u16(),
            block_pos: to_wasm_block_position(self.block_pos),
            old_current: self.old_current,
            new_current: self.new_current,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockRedstoneEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                block_state_id: BlockStateId::new_or_air(data.state_id),
                block_pos: from_wasm_block_position(data.block_pos),
                old_current: data.old_current,
                new_current: data.new_current,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BlockBreakEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let player = self.player.as_ref().map(|player| {
            state
                .add_player(player.clone())
                .expect("failed to add player resource")
        });

        Event::BlockBreakEvent(BlockBreakEventData {
            player,
            block: to_wasm_block_name(self.block),
            block_pos: to_wasm_block_position(self.block_position),
            exp: self.exp,
            should_drop: self.drop,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockBreakEvent(data) => Self {
                player: data.player.map(|player| consume_player(state, &player)),
                block: from_wasm_block_name(&data.block),
                block_position: from_wasm_block_position(data.block_pos),
                exp: data.exp,
                drop: data.should_drop,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BlockBurnEvent {
    fn to_wasm_event(&self, _state: &mut PluginHostState) -> Event {
        Event::BlockBurnEvent(BlockBurnEventData {
            igniting_block: to_wasm_block_name(self.igniting_block),
            block: to_wasm_block_name(self.block),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, _state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockBurnEvent(data) => Self {
                igniting_block: from_wasm_block_name(&data.igniting_block),
                block: from_wasm_block_name(&data.block),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BlockCanBuildEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let player = state
            .add_player(self.player.clone())
            .expect("failed to add player resource");

        Event::BlockCanBuildEvent(BlockCanBuildEventData {
            block_to_build: to_wasm_block_name(self.block_to_build),
            buildable: self.buildable,
            player,
            block: to_wasm_block_name(self.block),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockCanBuildEvent(data) => Self {
                block_to_build: from_wasm_block_name(&data.block_to_build),
                buildable: data.buildable,
                player: consume_player(state, &data.player),
                block: from_wasm_block_name(&data.block),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BlockGrowEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");

        Event::BlockGrowEvent(BlockGrowEventData {
            target_world,
            old_block: to_wasm_block_name(self.old_block),
            old_state_id: self.old_state_id.as_u16(),
            new_block: to_wasm_block_name(self.new_block),
            new_state_id: self.new_state_id.as_u16(),
            block_pos: to_wasm_block_position(self.block_pos),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockGrowEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                old_block: from_wasm_block_name(&data.old_block),
                old_state_id: BlockStateId::new_or_air(data.old_state_id),
                new_block: from_wasm_block_name(&data.new_block),
                new_state_id: BlockStateId::new_or_air(data.new_state_id),
                block_pos: from_wasm_block_position(data.block_pos),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BlockPlaceEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let player = state
            .add_player(self.player.clone())
            .expect("failed to add player resource");

        Event::BlockPlaceEvent(BlockPlaceEventData {
            player,
            block_placed: to_wasm_block_name(self.block_placed),
            block_placed_against: to_wasm_block_name(self.block_placed_against),
            block_pos: to_wasm_block_position(self.block_position),
            can_build: self.can_build,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockPlaceEvent(data) => Self {
                player: consume_player(state, &data.player),
                block_placed: from_wasm_block_name(&data.block_placed),
                block_placed_against: from_wasm_block_name(&data.block_placed_against),
                block_position: from_wasm_block_position(data.block_pos),
                can_build: data.can_build,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BlockFormEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");

        Event::BlockFormEvent(BlockFormEventData {
            target_world,
            block: to_wasm_block_name(self.block),
            block_pos: to_wasm_block_position(self.block_pos),
            new_block: to_wasm_block_name(self.new_block),
            new_state_id: self.new_state_id.as_u16(),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockFormEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                block: from_wasm_block_name(&data.block),
                block_pos: from_wasm_block_position(data.block_pos),
                new_block: from_wasm_block_name(&data.new_block),
                new_state_id: BlockStateId::new_or_air(data.new_state_id),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BlockMultiPlaceEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let player = state
            .add_player(self.player.clone())
            .expect("failed to add player resource");

        Event::BlockMultiPlaceEvent(BlockMultiPlaceEventData {
            player,
            block_placed: to_wasm_block_name(self.block_placed),
            block_placed_against: to_wasm_block_name(self.block_placed_against),
            primary_pos: to_wasm_block_position(self.primary_pos),
            affected_positions: self
                .affected_positions
                .iter()
                .copied()
                .map(to_wasm_block_position)
                .collect(),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockMultiPlaceEvent(data) => Self {
                player: consume_player(state, &data.player),
                block_placed: from_wasm_block_name(&data.block_placed),
                block_placed_against: from_wasm_block_name(&data.block_placed_against),
                primary_pos: from_wasm_block_position(data.primary_pos),
                affected_positions: data
                    .affected_positions
                    .into_iter()
                    .map(from_wasm_block_position)
                    .collect(),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for StructureGrowEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");

        Event::StructureGrowEvent(StructureGrowEventData {
            target_world,
            block: to_wasm_block_name(self.block),
            block_pos: to_wasm_block_position(self.block_pos),
            placed_feature: self.placed_feature.to_name().to_string(),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::StructureGrowEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                block: from_wasm_block_name(&data.block),
                block_pos: from_wasm_block_position(data.block_pos),
                placed_feature: pumpkin_data::placed_feature::PlacedFeature::from_name(
                    &data.placed_feature,
                )
                .expect("invalid placed feature name"),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}
