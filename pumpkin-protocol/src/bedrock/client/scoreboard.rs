use std::io::{Error, Write};

use crate::{
    codec::{var_int::VarInt, var_long::VarLong, var_uint::VarUInt},
    serial::PacketWrite,
};
use pumpkin_macros::packet;

#[derive(PacketWrite)]
#[packet(107)]
pub struct CSetDisplayObjective {
    pub display_slot: String,
    pub objective_name: String,
    pub display_name: String,
    pub criteria_name: String,
    pub sort_order: VarInt,
}

#[packet(108)]
pub struct CSetScore {
    pub action: u8,
    pub entries: Vec<ScoreEntry>,
}

pub struct ScoreEntry {
    pub scoreboard_id: VarLong,
    pub objective_name: String,
    pub score: i32,
    pub entry_type: u8,
    pub entity_unique_id: VarLong,
    pub custom_name: String,
}

impl CSetScore {
    pub const ACTION_CHANGE: u8 = 0;
    pub const ACTION_REMOVE: u8 = 1;
}

impl ScoreEntry {
    pub const TYPE_PLAYER: u8 = 1;
    pub const TYPE_ENTITY: u8 = 2;
    pub const TYPE_FAKE_PLAYER: u8 = 3;

    fn write<W: Write>(&self, writer: &mut W, include_identity: bool) -> Result<(), Error> {
        self.scoreboard_id.write(writer)?;
        self.objective_name.write(writer)?;
        self.score.write(writer)?;
        if !include_identity {
            return Ok(());
        }

        self.entry_type.write(writer)?;
        match self.entry_type {
            Self::TYPE_PLAYER | Self::TYPE_ENTITY => self.entity_unique_id.write(writer)?,
            Self::TYPE_FAKE_PLAYER => self.custom_name.write(writer)?,
            _ => return Err(Error::other("Invalid scoreboard entry type")),
        }
        Ok(())
    }
}

impl PacketWrite for CSetScore {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.action.write(writer)?;
        VarUInt(self.entries.len() as u32).write(writer)?;
        let include_identity = match self.action {
            Self::ACTION_CHANGE => true,
            Self::ACTION_REMOVE => false,
            _ => return Err(Error::other("Invalid SetScore action")),
        };
        for entry in &self.entries {
            entry.write(writer, include_identity)?;
        }
        Ok(())
    }
}

#[derive(PacketWrite)]
#[packet(106)]
pub struct CRemoveObjective {
    pub objective_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_score_entry() -> ScoreEntry {
        ScoreEntry {
            scoreboard_id: VarLong(1),
            objective_name: "0".to_string(),
            score: 1,
            entry_type: ScoreEntry::TYPE_FAKE_PLAYER,
            entity_unique_id: VarLong(0),
            custom_name: "owner".to_string(),
        }
    }

    #[test]
    fn set_score_uses_bedrock_wire_types() {
        let packet = CSetScore {
            action: CSetScore::ACTION_CHANGE,
            entries: vec![fake_score_entry()],
        };
        let mut bytes = Vec::new();
        packet.write(&mut bytes).unwrap();

        assert_eq!(
            bytes,
            vec![
                0, 1, // action and entry count
                2, // signed VarLong scoreboard ID 1
                1, b'0', // objective ID
                1, 0, 0, 0, // little-endian i32 score
                3, // fake-player scorer type
                5, b'o', b'w', b'n', b'e', b'r', // custom name
            ]
        );
    }

    #[test]
    fn remove_score_omits_scorer_identity() {
        let packet = CSetScore {
            action: CSetScore::ACTION_REMOVE,
            entries: vec![fake_score_entry()],
        };
        let mut bytes = Vec::new();
        packet.write(&mut bytes).unwrap();

        assert_eq!(
            bytes,
            vec![
                1, 1, // action and entry count
                2, // signed VarLong scoreboard ID 1
                1, b'0', // objective ID
                1, 0, 0, 0, // little-endian i32 score
            ]
        );
    }
}
