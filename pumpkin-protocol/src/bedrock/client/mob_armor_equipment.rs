use pumpkin_macros::packet;

use crate::{
    bedrock::network_item::NetworkItemStackDescriptor, codec::var_ulong::VarULong,
    serial::PacketWrite,
};

/// Replaces the complete armor state rendered on an actor by Bedrock clients.
#[derive(PacketWrite)]
#[packet(32)]
pub struct CMobArmorEquipment {
    pub entity_runtime_id: VarULong,
    pub helmet: NetworkItemStackDescriptor,
    pub chestplate: NetworkItemStackDescriptor,
    pub leggings: NetworkItemStackDescriptor,
    pub boots: NetworkItemStackDescriptor,
    pub body: NetworkItemStackDescriptor,
}

#[cfg(test)]
mod tests {
    use crate::{
        bedrock::network_item::NetworkItemStackDescriptor, codec::var_ulong::VarULong,
        serial::PacketWrite,
    };

    use super::CMobArmorEquipment;

    #[test]
    fn writes_protocol_1001_complete_armor_layout() {
        let empty = NetworkItemStackDescriptor::default();
        let packet = CMobArmorEquipment {
            entity_runtime_id: VarULong(1),
            helmet: empty.clone(),
            chestplate: empty.clone(),
            leggings: empty.clone(),
            boots: empty.clone(),
            body: empty,
        };

        let mut encoded = Vec::new();
        packet.write(&mut encoded).unwrap();

        assert_eq!(encoded.len(), 41);
        assert_eq!(encoded[0], 1);
        assert!(encoded[1..].iter().all(|byte| *byte == 0));
    }
}
