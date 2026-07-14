use std::io::{Error, Write};

use pumpkin_macros::packet;

use crate::{codec::var_uint::VarUInt, serial::PacketWrite};

/// Supplies the localized death cause shown on Bedrock's death screen.
#[packet(189)]
pub struct CDeathInfo {
    pub cause: String,
    pub messages: Vec<String>,
}

impl PacketWrite for CDeathInfo {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.cause.write(writer)?;
        VarUInt(self.messages.len() as u32).write(writer)?;
        for message in &self.messages {
            message.write(writer)?;
        }
        Ok(())
    }
}
