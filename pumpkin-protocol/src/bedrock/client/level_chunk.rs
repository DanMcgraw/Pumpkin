use std::io::{Error, Write};

use pumpkin_macros::packet;
use pumpkin_world::chunk::{ChunkData, palette::NetworkPalette};

use crate::{
    codec::{var_int::VarInt, var_uint::VarUInt},
    serial::PacketWrite,
};
const VERSION: u8 = 9;

#[packet(58)]
pub struct CLevelChunk<'a> {
    // https://mojang.github.io/bedrock-protocol-docs/html/LevelChunkPacket.html
    pub dimension: i32,
    pub cache_enabled: bool,

    // https://gist.github.com/Tomcc/a96af509e275b1af483b25c543cfbf37
    // https://github.com/Mojang/bedrock-protocol-docs/blob/main/additional_docs/SubChunk%20Request%20System%20v1.18.10.md
    pub chunk: &'a ChunkData,
}

/// Replaces a previously sent chunk with an empty column on the Bedrock client.
/// Bedrock has no dedicated chunk-unload packet; an empty `LevelChunk` is the
/// protocol-compatible invalidation used when a chunk leaves the publisher
/// radius.
#[packet(58)]
pub struct CEmptyLevelChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub dimension: i32,
    pub dimension_sub_chunk_count: u32,
}

impl PacketWrite for CLevelChunk<'_> {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        VarInt(self.chunk.x).write(writer)?;
        VarInt(self.chunk.z).write(writer)?;

        VarInt(self.dimension).write(writer)?;
        let sub_chunk_count = self.chunk.section.count as u32;
        VarUInt(sub_chunk_count).write(writer)?;
        self.cache_enabled.write(writer)?;

        let mut chunk_data = Vec::new();
        let data_write = &mut chunk_data;

        let block_sections = self
            .chunk
            .section
            .block_sections
            .read()
            .map_err(|_| Error::other("block_sections read lock poisoned"))?;
        let min_y_section = (self.chunk.section.min_y >> 4) as i8;

        for (i, block_palette) in block_sections.iter().enumerate() {
            // Version 9: [version:byte][num_storages:byte][sub_chunk_index:byte]
            let y = (i as i8) + min_y_section;
            let layer1 = block_palette.get_bedrock_layer1();
            let num_storages = if layer1.is_some() { 2 } else { 1 };
            data_write.write_all(&[VERSION, num_storages, y as u8])?;

            let network_repr = block_palette.convert_be_network();

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

            if let Some(layer1_repr) = layer1 {
                (layer1_repr.bits_per_entry << 1 | 1).write(data_write)?;

                for data in layer1_repr.packed_data {
                    data.write(data_write)?;
                }

                match layer1_repr.palette {
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
        }

        let biome_sections = self
            .chunk
            .section
            .biome_sections
            .read()
            .map_err(|_| Error::other("biome_sections read lock poisoned"))?;

        for (i, biome_palette) in biome_sections.iter().enumerate() {
            let num_storages = 1;
            let y = (i as i8) + min_y_section;
            data_write.write_all(&[VERSION, num_storages, y as u8])?;

            for _ in 0..num_storages {
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
        }

        data_write.write_all(&[0])?;

        VarUInt(chunk_data.len() as u32).write(writer)?;
        writer.write_all(&chunk_data)
    }
}

impl PacketWrite for CEmptyLevelChunk {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        VarInt(self.chunk_x).write(writer)?;
        VarInt(self.chunk_z).write(writer)?;
        VarInt(self.dimension).write(writer)?;
        VarUInt(0).write(writer)?; // no block sub-chunks
        false.write(writer)?; // cache disabled

        // A LevelChunk with zero block sub-chunks still carries one empty biome
        // palette, continuation markers for the remaining vertical sections,
        // and the Education Edition border-block terminator.
        let sub_chunk_count = self.dimension_sub_chunk_count.max(1);
        let mut chunk_data = Vec::with_capacity(sub_chunk_count as usize + 2);
        chunk_data.extend_from_slice(&[1, 0]); // singleton runtime biome palette: biome 0
        chunk_data.extend(std::iter::repeat_n(
            0xff,
            sub_chunk_count.saturating_sub(1) as usize,
        ));
        chunk_data.push(0); // border blocks

        VarUInt(chunk_data.len() as u32).write(writer)?;
        writer.write_all(&chunk_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::serial::PacketWrite;

    use super::CEmptyLevelChunk;

    #[test]
    fn empty_chunk_contains_dimension_and_unload_payload() {
        let packet = CEmptyLevelChunk {
            chunk_x: -2,
            chunk_z: 3,
            dimension: 1,
            dimension_sub_chunk_count: 2,
        };
        let mut bytes = Vec::new();
        packet.write(&mut bytes).unwrap();

        assert_eq!(bytes, [3, 6, 2, 0, 0, 4, 1, 0, 0xff, 0]);
    }
}
