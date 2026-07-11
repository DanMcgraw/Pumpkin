use std::io::{Error, Read};

use pumpkin_macros::packet;
use pumpkin_util::math::vector3::Vector3;

use crate::{codec::var_int::VarInt, codec::var_uint::VarUInt, serial::PacketRead};

#[derive(Debug)]
pub struct SubChunkOffset {
    pub dx: i8,
    pub dy: i8,
    pub dz: i8,
}

impl PacketRead for SubChunkOffset {
    fn read<R: Read>(reader: &mut R) -> Result<Self, Error> {
        Ok(Self {
            dx: i8::read(reader)?,
            dy: i8::read(reader)?,
            dz: i8::read(reader)?,
        })
    }
}

#[derive(Debug)]
#[packet(175)]
pub struct SSubChunkRequest {
    // https://mojang.github.io/bedrock-protocol-docs/html/SubChunkRequestPacket.html
    pub dimension: VarInt,
    pub offsets: Vec<SubChunkOffset>,
    pub position: Vector3<i32>,
}

impl PacketRead for SSubChunkRequest {
    fn read<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let dimension = VarInt::read(reader)?;
        let len = VarUInt::read(reader)?.0 as usize;
        let mut offsets = Vec::with_capacity(len);
        for _ in 0..len {
            offsets.push(SubChunkOffset::read(reader)?);
        }
        let position = Vector3::<i32>::read(reader)?;
        Ok(Self {
            dimension,
            offsets,
            position,
        })
    }
}
