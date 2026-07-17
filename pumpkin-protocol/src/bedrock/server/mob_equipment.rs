use crate::{
    bedrock::network_item::NetworkItemStackDescriptor, codec::var_ulong::VarULong,
    serial::PacketRead,
};
use pumpkin_macros::packet;
use std::io::{Error, Read};

#[derive(Debug)]
#[packet(31)]
pub struct SMobEquipment {
    pub entity_runtime_id: VarULong,
    pub item: NetworkItemStackDescriptor,
    pub inventory_slot: u8,
    pub hotbar_slot: u8,
    pub window_id: i8,
}

impl PacketRead for SMobEquipment {
    fn read<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let entity_runtime_id = VarULong::read(reader)?;
        let item = NetworkItemStackDescriptor::read(reader)?;
        let inventory_slot = u8::read(reader)?;
        let hotbar_slot = u8::read(reader)?;
        let window_id = i8::read(reader)?;

        Ok(Self {
            entity_runtime_id,
            item,
            inventory_slot,
            hotbar_slot,
            window_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::serial::PacketRead;

    use super::SMobEquipment;

    #[test]
    fn reads_protocol_1001_empty_item_layout() {
        let packet = SMobEquipment::read(&mut Cursor::new([
            1, // runtime entity id
            0, 0, // fixed-width item network id
            0, 0, // fixed-width count
            0, // metadata
            0, // no stack id
            0, // block runtime id
            0, // empty user-data buffer
            4, // inventory slot
            4, // selected hotbar slot
            0, // inventory window
        ]))
        .unwrap();

        assert_eq!(packet.entity_runtime_id.0, 1);
        assert_eq!(packet.item.id, 0);
        assert_eq!(packet.inventory_slot, 4);
        assert_eq!(packet.hotbar_slot, 4);
        assert_eq!(packet.window_id, 0);
    }
}
