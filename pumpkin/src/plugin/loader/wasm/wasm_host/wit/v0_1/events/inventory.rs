use crate::plugin::{
    inventory::inventory_move_item::InventoryMoveItemEvent,
    loader::wasm::wasm_host::{
        state::PluginHostState,
        wit::v0_1::{
            events::{
                ToFromWasmEvent, consume_item_stack, from_wasm_block_position,
                to_wasm_block_position, to_wasm_item_stack,
            },
            pumpkin::plugin::event::{Event, InventoryMoveItemEventData},
        },
    },
};

impl ToFromWasmEvent for InventoryMoveItemEvent {
    fn to_wasm_event(&self, state: &mut PluginHostState) -> Event {
        Event::InventoryMoveItemEvent(InventoryMoveItemEventData {
            item: to_wasm_item_stack(state, &self.item),
            source_pos: self.source_pos.map(to_wasm_block_position),
            destination_pos: self.destination_pos.map(to_wasm_block_position),
            cancelled: self.cancelled,
        })
    }

    fn from_wasm_event(event: Event, state: &mut PluginHostState) -> Self {
        match event {
            Event::InventoryMoveItemEvent(data) => Self {
                item: consume_item_stack(state, &data.item),
                source: None,
                destination: None,
                source_pos: data.source_pos.map(from_wasm_block_position),
                destination_pos: data.destination_pos.map(from_wasm_block_position),
                cancelled: data.cancelled,
            },
            _ => panic!("unexpected event type"),
        }
    }
}
