use pumpkin_macros::packet;

use crate::{
    bedrock::network_item::NetworkItemStackDescriptor, codec::var_ulong::VarULong,
    serial::PacketWrite,
};

/// Updates an actor's main-hand or offhand item for Bedrock observers.
#[derive(PacketWrite)]
#[packet(31)]
pub struct CMobEquipment {
    pub entity_runtime_id: VarULong,
    pub item: NetworkItemStackDescriptor,
    pub inventory_slot: u8,
    pub hotbar_slot: u8,
    pub window_id: i8,
}

impl CMobEquipment {
    pub const INVENTORY_CONTAINER_ID: i8 = 0;
    pub const OFFHAND_CONTAINER_ID: i8 = 119;
    pub const NO_HOTBAR_SLOT: u8 = u8::MAX;

    #[must_use]
    pub const fn main_hand(entity_runtime_id: VarULong, item: NetworkItemStackDescriptor) -> Self {
        Self {
            entity_runtime_id,
            item,
            inventory_slot: 0,
            hotbar_slot: Self::NO_HOTBAR_SLOT,
            window_id: Self::INVENTORY_CONTAINER_ID,
        }
    }

    #[must_use]
    pub const fn off_hand(entity_runtime_id: VarULong, item: NetworkItemStackDescriptor) -> Self {
        Self {
            entity_runtime_id,
            item,
            inventory_slot: 0,
            hotbar_slot: Self::NO_HOTBAR_SLOT,
            window_id: Self::OFFHAND_CONTAINER_ID,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        bedrock::network_item::NetworkItemStackDescriptor, codec::var_ulong::VarULong,
        serial::PacketWrite,
    };

    use super::CMobEquipment;

    #[test]
    fn writes_main_and_offhand_container_ids() {
        let mut main = Vec::new();
        CMobEquipment::main_hand(VarULong(1), NetworkItemStackDescriptor::default())
            .write(&mut main)
            .unwrap();
        assert_eq!(main, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0]);

        let mut offhand = Vec::new();
        CMobEquipment::off_hand(VarULong(1), NetworkItemStackDescriptor::default())
            .write(&mut offhand)
            .unwrap();
        assert_eq!(offhand, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 119]);
    }
}
