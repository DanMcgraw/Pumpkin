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
