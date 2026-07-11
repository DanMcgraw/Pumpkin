use std::io::{Error, Write};

use pumpkin_macros::packet;
use pumpkin_util::math::vector3::Vector3;

use crate::{
    codec::{var_int::VarInt, var_uint::VarUInt},
    serial::PacketWrite,
};

pub const SUB_CHUNK_RESULT_SUCCESS: u8 = 1;
pub const SUB_CHUNK_RESULT_CHUNK_NOT_FOUND: u8 = 2;
pub const SUB_CHUNK_RESULT_WRONG_DIMENSION: u8 = 3;
pub const SUB_CHUNK_RESULT_PLAYER_NOT_FOUND: u8 = 4;
pub const SUB_CHUNK_RESULT_INDEX_OUT_OF_BOUNDS: u8 = 5;
pub const SUB_CHUNK_RESULT_SUCCESS_ALL_AIR: u8 = 6;

pub const HEIGHTMAP_DATA_NONE: u8 = 0;
pub const HEIGHTMAP_DATA_HAS_DATA: u8 = 1;
pub const HEIGHTMAP_DATA_TOO_HIGH: u8 = 2;
pub const HEIGHTMAP_DATA_TOO_LOW: u8 = 3;
pub const HEIGHTMAP_DATA_ALL_COPIED: u8 = 4;

#[derive(Clone, Debug)]
pub struct SubChunkOffset {
    pub dx: i8,
    pub dy: i8,
    pub dz: i8,
}

impl PacketWrite for SubChunkOffset {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.dx.write(writer)?;
        self.dy.write(writer)?;
        self.dz.write(writer)
    }
}

#[derive(Clone, Debug)]
pub struct SubChunkEntry {
    pub offset: SubChunkOffset,
    pub result: u8,
    pub payload: Vec<u8>,
    pub heightmap_type: u8,
    pub heightmap: Vec<i8>,
    pub render_heightmap_type: u8,
    pub render_heightmap: Vec<i8>,
    pub blob_id: u64,
}

impl PacketWrite for SubChunkEntry {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.offset.write(writer)?;
        self.result.write(writer)?;

        VarUInt(self.payload.len() as u32).write(writer)?;
        writer.write_all(&self.payload)?;

        self.heightmap_type.write(writer)?;
        if self.heightmap_type == HEIGHTMAP_DATA_HAS_DATA {
            debug_assert_eq!(self.heightmap.len(), 256);
            for &v in &self.heightmap {
                v.write(writer)?;
            }
        }

        self.render_heightmap_type.write(writer)?;
        if self.render_heightmap_type == HEIGHTMAP_DATA_HAS_DATA {
            debug_assert_eq!(self.render_heightmap.len(), 256);
            for &v in &self.render_heightmap {
                v.write(writer)?;
            }
        }

        self.blob_id.write(writer)
    }
}

#[packet(174)]
pub struct CSubChunkPacket {
    // https://mojang.github.io/bedrock-protocol-docs/html/SubChunkPacket.html
    pub cache_enabled: bool,
    pub dimension: VarInt,
    pub center: Vector3<i32>,
    pub entries: Vec<SubChunkEntry>,
}

impl PacketWrite for CSubChunkPacket {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.cache_enabled.write(writer)?;
        self.dimension.write(writer)?;
        self.center.write(writer)?;

        // SubChunk entry count is a fixed uint32, not a varint.
        (self.entries.len() as u32).write(writer)?;
        for entry in &self.entries {
            entry.write(writer)?;
        }
        Ok(())
    }
}
