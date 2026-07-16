use pumpkin_macros::packet;

use crate::serial::PacketWrite;

#[derive(PacketWrite)]
#[packet(162)]
pub struct CItemRegistry {
    // https://mojang.github.io/bedrock-protocol-docs/docs/ItemRegistryPacket.html
    pub items: Vec<ItemDefinition>,
}

#[derive(PacketWrite)]
pub struct ItemDefinition {
    pub name: String,

    // Normally would be `Nbt`, but for simplicity elsewhere, this is preserialized (via `Nbt::write_bedrock`)
    #[serial(no_prefix)]
    pub component_data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::{CItemRegistry, ItemDefinition};
    use crate::serial::PacketWrite;

    #[test]
    fn item_components_use_protocol_1001_wire_layout() {
        let packet = CItemRegistry {
            items: vec![ItemDefinition {
                name: "minecraft:apple".to_string(),
                component_data: vec![10, 0, 0],
            }],
        };
        let mut encoded = Vec::new();

        packet.write(&mut encoded).unwrap();

        assert_eq!(
            encoded,
            [
                1, // item count
                15, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':', b'a', b'p', b'p',
                b'l', b'e', // identifier
                10, 0, 0, // little-endian NBT compound
            ]
        );
    }
}
