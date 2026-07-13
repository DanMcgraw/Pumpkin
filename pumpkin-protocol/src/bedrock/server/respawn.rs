use pumpkin_macros::packet;
use pumpkin_util::math::vector3::Vector3;

use crate::{bedrock::respawn::PlayerRespawnState, codec::var_ulong::VarULong, serial::PacketRead};

#[derive(Debug, PacketRead)]
#[packet(45)]
pub struct SRespawn {
    pub position: Vector3<f32>,
    pub state: PlayerRespawnState,
    pub player_runtime_id: VarULong,
}
