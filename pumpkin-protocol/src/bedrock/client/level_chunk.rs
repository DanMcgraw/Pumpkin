use std::io::{Error, Write};

use pumpkin_macros::packet;
use pumpkin_world::chunk::{ChunkData, palette::NetworkPalette};

use crate::{
    codec::{var_int::VarInt, var_uint::VarUInt},
    serial::PacketWrite,
};

const VERSION: u8 = 9;

/// Serialize a single Bedrock sub-chunk (version 9) for use in either a full
/// `LevelChunk` packet or a `SubChunk` response.
pub fn serialize_bedrock_block_subchunk(
    y: i8,
    palette: &pumpkin_world::chunk::palette::BlockPalette,
) -> Vec<u8> {
    let mut out = Vec::new();
    // Version 9: [version:byte][num_storages:byte][sub_chunk_index:byte]
    out.write_all(&[VERSION, 1, y as u8]).unwrap();

    let network_repr = palette.convert_be_network();
    (network_repr.bits_per_entry << 1 | 1).write(&mut out).unwrap();

    for data in network_repr.packed_data {
        data.write(&mut out).unwrap();
    }

    match network_repr.palette {
        NetworkPalette::Single(id) => {
            VarInt(i32::from(id)).write(&mut out).unwrap();
        }
        NetworkPalette::Indirect(palette) => {
            VarInt(palette.len() as i32).write(&mut out).unwrap();
            for id in palette {
                VarInt(i32::from(id)).write(&mut out).unwrap();
            }
        }
        NetworkPalette::Direct => (),
    }

    out
}

#[packet(58)]
pub struct CLevelChunk<'a> {
    // https://mojang.github.io/bedrock-protocol-docs/html/LevelChunkPacket.html
    pub dimension: i32,
    pub cache_enabled: bool,

    // https://gist.github.com/Tomcc/a96af509e275b1af483b25c543cfbf37
    // https://github.com/Mojang/bedrock-protocol-docs/blob/main/additional_docs/SubChunk%20Request%20System%20v1.18.10.md
    pub chunk: &'a ChunkData,
}

impl PacketWrite for CLevelChunk<'_> {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        VarInt(self.chunk.x).write(writer)?;
        VarInt(self.chunk.z).write(writer)?;

        VarInt(self.dimension).write(writer)?;
        // Use the SubChunk Request System: the client will request individual
        // sub-chunks via SubChunkRequestPacket after receiving the skeleton.
        VarUInt(u32::MAX).write(writer)?;
        self.cache_enabled.write(writer)?;

        let mut chunk_data = Vec::new();
        let data_write = &mut chunk_data;

        let biome_sections = self
            .chunk
            .section
            .biome_sections
            .read()
            .map_err(|_| Error::other("biome_sections read lock poisoned"))?;

        for biome_palette in biome_sections.iter() {
            let network_repr = biome_palette.convert_be_network();

            (network_repr.bits_per_entry << 1 | 1).write(data_write)?;

            for data in network_repr.packed_data {
                data.write(data_write)?;
            }

            match network_repr.palette {
                NetworkPalette::Single(id) => {
                    VarInt(i32::from(id)).write(data_write)?;
                }
                NetworkPalette::Indirect(palette) => {
                    VarInt(palette.len() as i32).write(data_write)?;
                    for id in palette {
                        VarInt(i32::from(id)).write(data_write)?;
                    }
                }
                NetworkPalette::Direct => (),
            }
        }

        // Border block array count (0).
        data_write.write_all(&[0])?;

        VarUInt(chunk_data.len() as u32).write(writer)?;
        writer.write_all(&chunk_data)
    }
}
