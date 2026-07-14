use std::io::{Error, Write};

use pumpkin_macros::packet;

use crate::{
    codec::{var_int::VarInt, var_uint::VarUInt},
    serial::PacketWrite,
};

#[derive(PacketWrite, Debug, Clone, Default)]
#[packet(0x48)]
pub struct CGamerulesChanged {
    pub rule_data: GameRules,
}

impl CGamerulesChanged {
    #[must_use]
    pub const fn new(rules: Vec<GameRule>) -> Self {
        Self {
            rule_data: GameRules::new(rules),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GameRules {
    pub rules: Vec<GameRule>,
}

impl GameRules {
    #[must_use]
    pub const fn new(rules: Vec<GameRule>) -> Self {
        Self { rules }
    }
}

impl PacketWrite for GameRules {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        VarUInt(self.rules.len() as u32).write(writer)?;
        self.rules.write(writer)
    }
}

/// Gamerules embedded in `StartGame` use a signed VarInt for integer values,
/// unlike the fixed-width integer used by `GameRulesChanged`.
#[derive(Debug, Clone, Default)]
pub struct StartGameRules {
    pub rules: Vec<GameRule>,
}

impl StartGameRules {
    #[must_use]
    pub const fn new(rules: Vec<GameRule>) -> Self {
        Self { rules }
    }
}

impl PacketWrite for StartGameRules {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        VarUInt(self.rules.len() as u32).write(writer)?;
        for rule in &self.rules {
            rule.write_start_game(writer)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameRule {
    pub name: String,
    pub can_be_modified: bool,
    pub value: GameRuleValue,
}

impl GameRule {
    #[must_use]
    pub fn new(name: impl Into<String>, value: GameRuleValue) -> Self {
        Self {
            name: name.into(),
            can_be_modified: false,
            value,
        }
    }
}

impl PacketWrite for GameRule {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.name.write(writer)?;
        self.can_be_modified.write(writer)?;
        self.value.write(writer)
    }
}

impl GameRule {
    fn write_start_game<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.name.write(writer)?;
        self.can_be_modified.write(writer)?;
        self.value.write_start_game(writer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameRuleValue {
    Bool(bool),
    Int(i32),
    Float(f32),
}

impl PacketWrite for GameRuleValue {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            Self::Bool(value) => {
                VarUInt(1).write(writer)?;
                value.write(writer)
            }
            Self::Int(value) => {
                VarUInt(2).write(writer)?;
                value.write(writer)
            }
            Self::Float(value) => {
                VarUInt(3).write(writer)?;
                value.write(writer)
            }
        }
    }
}

impl GameRuleValue {
    fn write_start_game<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            Self::Bool(value) => {
                VarUInt(1).write(writer)?;
                value.write(writer)
            }
            Self::Int(value) => {
                VarUInt(2).write(writer)?;
                VarInt(*value).write(writer)
            }
            Self::Float(value) => {
                VarUInt(3).write(writer)?;
                value.write(writer)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::serial::PacketWrite;

    use super::{GameRule, GameRuleValue, GameRules, StartGameRules};

    #[test]
    fn writes_changed_game_rules_with_fixed_width_integers() {
        let rules = GameRules::new(vec![
            GameRule::new("enabled", GameRuleValue::Bool(true)),
            GameRule::new("radius", GameRuleValue::Int(-2)),
            GameRule::new("rate", GameRuleValue::Float(1.5)),
        ]);
        let mut bytes = Vec::new();
        rules.write(&mut bytes).unwrap();

        assert_eq!(
            bytes,
            [
                3, // rule count
                7, b'e', b'n', b'a', b'b', b'l', b'e', b'd', 0, // name + editable
                1, 1, // bool discriminator + value
                6, b'r', b'a', b'd', b'i', b'u', b's', 0, // name + editable
                2, 0xfe, 0xff, 0xff, 0xff, // int discriminator + fixed-width value
                4, b'r', b'a', b't', b'e', 0, // name + editable
                3, 0, 0, 0xc0, 0x3f, // float discriminator + value
            ]
        );
    }

    #[test]
    fn writes_start_game_rules_with_varint_integers() {
        let rules = StartGameRules::new(vec![
            GameRule::new("enabled", GameRuleValue::Bool(true)),
            GameRule::new("radius", GameRuleValue::Int(-2)),
            GameRule::new("rate", GameRuleValue::Float(1.5)),
        ]);
        let mut bytes = Vec::new();
        rules.write(&mut bytes).unwrap();

        assert_eq!(
            bytes,
            [
                3, // rule count
                7, b'e', b'n', b'a', b'b', b'l', b'e', b'd', 0, // name + editable
                1, 1, // bool discriminator + value
                6, b'r', b'a', b'd', b'i', b'u', b's', 0, // name + editable
                2, 3, // int discriminator + signed VarInt(-2)
                4, b'r', b'a', b't', b'e', 0, // name + editable
                3, 0, 0, 0xc0, 0x3f, // float discriminator + value
            ]
        );
    }
}
