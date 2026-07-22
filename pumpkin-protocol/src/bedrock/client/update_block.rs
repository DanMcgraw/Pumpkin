use std::io::{Error, Write};

use pumpkin_macros::packet;
use pumpkin_util::math::position::BlockPos;

use crate::{codec::var_uint::VarUInt, serial::PacketWrite};

#[packet(21)]
pub struct CUpdateBlock {
    pub position: BlockPos,
    pub block_runtime_id: VarUInt,
    pub flags: VarUInt,
    pub layer: VarUInt,
}

impl CUpdateBlock {
    #[must_use]
    pub const fn new(position: BlockPos, block_runtime_id: u32) -> Self {
        Self {
            position,
            block_runtime_id: VarUInt(block_runtime_id),
            flags: VarUInt(0x3), // neighbors | network
            layer: VarUInt(0),
        }
    }

    /// Creates the high-priority block update used to prepare a client-only
    /// inventory holder before its block-actor data is sent.
    #[must_use]
    pub const fn new_priority(position: BlockPos, block_runtime_id: u32) -> Self {
        Self {
            position,
            block_runtime_id: VarUInt(block_runtime_id),
            // NEIGHBORS | NETWORK | PRIORITY, matching Bedrock's
            // FLAG_ALL_PRIORITY and Geyser's virtual inventory holders.
            flags: VarUInt(0x13),
            layer: VarUInt(0),
        }
    }

    #[must_use]
    pub const fn new_with_layer(position: BlockPos, block_runtime_id: u32, layer: u32) -> Self {
        Self {
            position,
            block_runtime_id: VarUInt(block_runtime_id),
            flags: VarUInt(0x3), // neighbors | network
            layer: VarUInt(layer),
        }
    }
}

#[cfg(test)]
mod tests {
    use pumpkin_util::math::position::BlockPos;

    use super::CUpdateBlock;

    #[test]
    fn priority_update_uses_all_priority_flags() {
        let packet = CUpdateBlock::new_priority(BlockPos::ZERO, 42);
        assert_eq!(packet.flags.0, 0x13);
    }
}

impl PacketWrite for CUpdateBlock {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.position.write(writer)?;
        self.block_runtime_id.write(writer)?;
        self.flags.write(writer)?;
        self.layer.write(writer)?;
        Ok(())
    }
}
