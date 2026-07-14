use pumpkin_macros::packet;
use pumpkin_util::math::vector3::Vector3;

use crate::{
    bedrock::respawn::PlayerRespawnState, codec::var_ulong::VarULong, serial::PacketWrite,
};

#[derive(PacketWrite)]
#[packet(45)]
pub struct CRespawn {
    pub position: Vector3<f32>,
    pub state: PlayerRespawnState,
    pub player_runtime_id: VarULong,
}

impl CRespawn {
    #[must_use]
    pub const fn new(
        position: Vector3<f32>,
        state: PlayerRespawnState,
        player_runtime_id: VarULong,
    ) -> Self {
        Self {
            position,
            state,
            player_runtime_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        bedrock::respawn::PlayerRespawnState, codec::var_ulong::VarULong, serial::PacketWrite,
    };
    use pumpkin_util::math::vector3::Vector3;

    use super::CRespawn;

    #[test]
    fn server_ready_uses_zero_local_player_runtime_id() {
        let packet = CRespawn::new(
            Vector3::new(1.0, 2.0, 3.0),
            PlayerRespawnState::ReadyToSpawn,
            VarULong(0),
        );
        let mut bytes = Vec::new();
        packet.write(&mut bytes).unwrap();

        assert_eq!(bytes[12], PlayerRespawnState::ReadyToSpawn as u8);
        assert_eq!(bytes[13], 0);
    }
}
