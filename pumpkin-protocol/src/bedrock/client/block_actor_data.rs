use std::io::{Error, Write};

use pumpkin_macros::packet;
use pumpkin_nbt::Nbt;
use pumpkin_util::math::position::BlockPos;

use crate::serial::PacketWrite;

/// Updates the block actor attached to a block position.
///
/// Bedrock container screens use this packet to discover pairing metadata and
/// the custom title of virtual chest holders.
#[packet(56)]
pub struct CBlockActorData {
    pub position: BlockPos,
    pub data: Nbt,
}

impl PacketWrite for CBlockActorData {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.position.write(writer)?;
        writer.write_all(&self.data.clone().write_bedrock())
    }
}

#[cfg(test)]
mod tests {
    use pumpkin_nbt::{Nbt, compound::NbtCompound};
    use pumpkin_util::math::position::BlockPos;

    use crate::serial::PacketWrite;

    use super::CBlockActorData;

    #[test]
    fn block_actor_data_contains_position_then_little_endian_nbt() {
        let mut tag = NbtCompound::new();
        tag.put_int("x", 12);
        tag.put_string("id", "Chest".to_owned());
        let packet = CBlockActorData {
            position: BlockPos::new(12, 64, -3),
            data: Nbt::new(String::new(), tag),
        };

        let mut bytes = Vec::new();
        packet.write(&mut bytes).unwrap();

        assert!(bytes.len() > 4);
        assert!(bytes.windows(5).any(|window| window == b"Chest"));
    }
}
