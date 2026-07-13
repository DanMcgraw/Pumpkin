use std::io::{Error, ErrorKind, Read, Write};

use crate::serial::{PacketRead, PacketWrite};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayerRespawnState {
    SearchingForSpawn = 0,
    ReadyToSpawn = 1,
    ClientReadyToSpawn = 2,
}

impl PacketRead for PlayerRespawnState {
    fn read<R: Read>(reader: &mut R) -> Result<Self, Error> {
        match u8::read(reader)? {
            0 => Ok(Self::SearchingForSpawn),
            1 => Ok(Self::ReadyToSpawn),
            2 => Ok(Self::ClientReadyToSpawn),
            state => Err(Error::new(
                ErrorKind::InvalidData,
                format!("unknown Bedrock respawn state {state}"),
            )),
        }
    }
}

impl PacketWrite for PlayerRespawnState {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        (*self as u8).write(writer)
    }
}
