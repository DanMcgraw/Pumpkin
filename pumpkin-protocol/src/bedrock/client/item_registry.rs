use pumpkin_macros::packet;

use crate::{codec::var_int::VarInt, serial::PacketWrite};

#[derive(PacketWrite)]
#[packet(162)]
pub struct CItemRegistry {
    // https://mojang.github.io/bedrock-protocol-docs/docs/ItemRegistryPacket.html
    pub items: Vec<ItemDefinition>,
}

#[derive(PacketWrite)]
pub struct ItemDefinition {
    pub name: String,
    pub id: i16,
    pub component_based: bool,
    pub item_version: VarInt,

    // Normally would be `Nbt`, but for simplicity elsewhere, this is preserialized (via `Nbt::write_bedrock`)
    #[serial(no_prefix)]
    pub component_data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::{CItemRegistry, ItemDefinition};
    use crate::{codec::var_int::VarInt, serial::PacketWrite};

    #[test]
    fn item_components_use_protocol_1001_wire_layout() {
        let packet = CItemRegistry {
            items: vec![ItemDefinition {
                name: "minecraft:apple".to_string(),
                id: 285,
                component_based: true,
                item_version: VarInt(1),
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
                0x1d, 0x01, // little-endian runtime ID 285
                1,    // component based
                2,    // data-driven item version 1, zig-zag encoded
                10, 0, 0, // little-endian NBT compound
            ]
        );
    }
}
