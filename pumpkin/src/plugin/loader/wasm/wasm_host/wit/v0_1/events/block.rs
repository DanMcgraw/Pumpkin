use pumpkin_data::BlockStateId;
use pumpkin_util::math::vector3::Vector3;

use crate::plugin::loader::wasm::wasm_host::wit::v0_1::world::{
    from_wasm_block_direction, to_wasm_block_direction,
};
use crate::plugin::{
    block::{
        block_break::BlockBreakEvent, block_broken::BlockBrokenEvent, block_burn::BlockBurnEvent,
        block_can_build::BlockCanBuildEvent, block_damage::BlockDamageEvent,
        block_drop_item::BlockDropItemEvent, block_form::BlockFormEvent,
        block_grow::BlockGrowEvent, block_multi_place::BlockMultiPlaceEvent,
        block_piston_extend::BlockPistonExtendEvent, block_piston_retract::BlockPistonRetractEvent,
        block_place::BlockPlaceEvent, block_redstone::BlockRedstoneEvent, brew::BrewEvent,
        furnace_burn::FurnaceBurnEvent, furnace_smelt::FurnaceSmeltEvent,
        structure_grow::StructureGrowEvent,
    },
    loader::wasm::wasm_host::{
        state::PluginHostState,
        wit::v0_1::{
            events::{
                ToFromWasmEvent, consume_item_stack, consume_player, consume_world,
                from_wasm_block_name, from_wasm_block_position, to_wasm_block_name,
                to_wasm_block_position, to_wasm_item_stack,
            },
            pumpkin::plugin::event::{
                BlockBreakEventData, BlockBrokenEventData, BlockBurnEventData,
                BlockCanBuildEventData, BlockDamageEventData, BlockDropItemEventData,
                BlockFormEventData, BlockGrowEventData, BlockMultiPlaceEventData,
                BlockPistonExtendEventData, BlockPistonRetractEventData, BlockPlaceEventData,
                BlockRedstoneEventData, BrewEventData, Event, FurnaceBurnEventData,
                FurnaceSmeltEventData, StructureGrowEventData,
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
            face: self.face.map(to_wasm_block_direction),
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
                face: data.face.map(from_wasm_block_direction),
                exp: data.exp,
                drop: data.should_drop,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BlockDamageEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let player = state
            .add_player(self.player.clone())
            .expect("failed to add player resource");

        Event::BlockDamageEvent(BlockDamageEventData {
            player,
            block: to_wasm_block_name(self.block),
            block_pos: to_wasm_block_position(self.block_position),
            insta_break: self.insta_break,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockDamageEvent(data) => Self {
                player: consume_player(state, &data.player),
                block: from_wasm_block_name(&data.block),
                block_position: from_wasm_block_position(data.block_pos),
                insta_break: data.insta_break,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BlockBrokenEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");
        let player = self.player.as_ref().map(|player| {
            state
                .add_player(player.clone())
                .expect("failed to add player resource")
        });

        Event::BlockBrokenEvent(BlockBrokenEventData {
            target_world,
            player,
            block: to_wasm_block_name(self.block),
            block_state_id: self.block_state_id.as_u16(),
            replacement_state_id: self.replacement_state_id.as_u16(),
            block_pos: to_wasm_block_position(self.block_position),
            face: self.face.map(to_wasm_block_direction),
            dropped_items: self.dropped_items,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockBrokenEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                player: data.player.map(|player| consume_player(state, &player)),
                block: from_wasm_block_name(&data.block),
                block_state_id: BlockStateId::new_or_air(data.block_state_id),
                replacement_state_id: BlockStateId::new_or_air(data.replacement_state_id),
                block_position: from_wasm_block_position(data.block_pos),
                face: data.face.map(from_wasm_block_direction),
                dropped_items: data.dropped_items,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BlockDropItemEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let player = state
            .add_player(self.player.clone())
            .expect("failed to add player resource");
        let items = self
            .items
            .iter()
            .map(|item| to_wasm_item_stack(state, item))
            .collect();

        Event::BlockDropItemEvent(BlockDropItemEventData {
            player,
            block: to_wasm_block_name(self.block),
            block_pos: to_wasm_block_position(self.block_position),
            items,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockDropItemEvent(data) => Self {
                player: consume_player(state, &data.player),
                block: from_wasm_block_name(&data.block),
                block_position: from_wasm_block_position(data.block_pos),
                items: data
                    .items
                    .iter()
                    .map(|item| consume_item_stack(state, item))
                    .collect(),
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

impl ToFromWasmEvent for BlockPistonExtendEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");

        Event::BlockPistonExtendEvent(BlockPistonExtendEventData {
            target_world,
            piston_pos: to_wasm_block_position(self.piston_pos),
            piston_block: to_wasm_block_name(self.piston_block),
            direction_x: self.direction.x,
            direction_y: self.direction.y,
            direction_z: self.direction.z,
            moved_blocks: self
                .moved_blocks
                .iter()
                .copied()
                .map(to_wasm_block_position)
                .collect(),
            broken_blocks: self
                .broken_blocks
                .iter()
                .copied()
                .map(to_wasm_block_position)
                .collect(),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockPistonExtendEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                piston_pos: from_wasm_block_position(data.piston_pos),
                piston_block: from_wasm_block_name(&data.piston_block),
                direction: Vector3::new(data.direction_x, data.direction_y, data.direction_z),
                moved_blocks: data
                    .moved_blocks
                    .into_iter()
                    .map(from_wasm_block_position)
                    .collect(),
                broken_blocks: data
                    .broken_blocks
                    .into_iter()
                    .map(from_wasm_block_position)
                    .collect(),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BlockPistonRetractEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");

        Event::BlockPistonRetractEvent(BlockPistonRetractEventData {
            target_world,
            piston_pos: to_wasm_block_position(self.piston_pos),
            piston_block: to_wasm_block_name(self.piston_block),
            direction_x: self.direction.x,
            direction_y: self.direction.y,
            direction_z: self.direction.z,
            moved_blocks: self
                .moved_blocks
                .iter()
                .copied()
                .map(to_wasm_block_position)
                .collect(),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BlockPistonRetractEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                piston_pos: from_wasm_block_position(data.piston_pos),
                piston_block: from_wasm_block_name(&data.piston_block),
                direction: Vector3::new(data.direction_x, data.direction_y, data.direction_z),
                moved_blocks: data
                    .moved_blocks
                    .into_iter()
                    .map(from_wasm_block_position)
                    .collect(),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for BrewEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");
        let potions = self
            .potions
            .iter()
            .map(|potion| to_wasm_item_stack(state, potion))
            .collect();

        Event::BrewEvent(BrewEventData {
            target_world,
            block: to_wasm_block_name(self.block),
            block_pos: to_wasm_block_position(self.block_position),
            ingredient: to_wasm_item_stack(state, &self.ingredient),
            potions,
            fuel: self.fuel,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::BrewEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                block: from_wasm_block_name(&data.block),
                block_position: from_wasm_block_position(data.block_pos),
                ingredient: consume_item_stack(state, &data.ingredient),
                potions: data
                    .potions
                    .iter()
                    .map(|potion| consume_item_stack(state, potion))
                    .collect(),
                fuel: data.fuel,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for FurnaceBurnEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");

        Event::FurnaceBurnEvent(FurnaceBurnEventData {
            target_world,
            block: to_wasm_block_name(self.block),
            block_pos: to_wasm_block_position(self.block_position),
            fuel: to_wasm_item_stack(state, &self.fuel),
            burn_time: self.burn_time,
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::FurnaceBurnEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                block: from_wasm_block_name(&data.block),
                block_position: from_wasm_block_position(data.block_pos),
                fuel: consume_item_stack(state, &data.fuel),
                burn_time: data.burn_time,
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}

impl ToFromWasmEvent for FurnaceSmeltEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        let target_world = state
            .add_world(self.world.clone())
            .expect("failed to add world resource");

        Event::FurnaceSmeltEvent(FurnaceSmeltEventData {
            target_world,
            block: to_wasm_block_name(self.block),
            block_pos: to_wasm_block_position(self.block_position),
            input: to_wasm_item_stack(state, &self.input),
            fuel: to_wasm_item_stack(state, &self.fuel),
            output: to_wasm_item_stack(state, &self.output),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::FurnaceSmeltEvent(data) => Self {
                world: consume_world(state, &data.target_world),
                block: from_wasm_block_name(&data.block),
                block_position: from_wasm_block_position(data.block_pos),
                input: consume_item_stack(state, &data.input),
                fuel: consume_item_stack(state, &data.fuel),
                output: consume_item_stack(state, &data.output),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}
