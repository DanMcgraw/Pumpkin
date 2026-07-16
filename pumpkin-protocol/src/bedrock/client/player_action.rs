use pumpkin_macros::packet;
use pumpkin_util::math::position::BlockPos;

use crate::{
    codec::{var_int::VarInt, var_ulong::VarULong},
    serial::PacketWrite,
};

/// Sends a server-originated player action to a Bedrock client.
#[derive(Debug, PacketWrite)]
#[packet(36)]
pub struct CPlayerAction {
    pub runtime_id: VarULong,
    pub action: VarInt,
    pub block_pos: BlockPos,
    pub result_pos: BlockPos,
    pub face: VarInt,
}

impl CPlayerAction {
    const DIMENSION_CHANGE_SUCCESS: i32 = 14;

    #[must_use]
    pub const fn dimension_change_success(runtime_id: u64) -> Self {
        Self {
            runtime_id: VarULong(runtime_id),
            action: VarInt(Self::DIMENSION_CHANGE_SUCCESS),
            block_pos: BlockPos::ZERO,
            result_pos: BlockPos::ZERO,
            face: VarInt(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CPlayerAction;
    use crate::serial::PacketWrite;

    #[test]
    fn dimension_change_success_uses_unsigned_runtime_id() {
        let mut encoded = Vec::new();

        CPlayerAction::dimension_change_success(300)
            .write(&mut encoded)
            .unwrap();

        assert_eq!(encoded, [0xac, 0x02, 0x1c, 0, 0, 0, 0, 0, 0, 0]);
    }
}
