use std::io::{Error, ErrorKind, Read};

use pumpkin_macros::packet;
use pumpkin_util::math::position::BlockPos;

use crate::bedrock::network_item::NetworkItemDescriptor;
use crate::{
    codec::{var_int::VarInt, var_uint::VarUInt, var_ulong::VarULong},
    serial::PacketRead,
};
use pumpkin_util::math::vector3::Vector3;

pub const WINDOW_ID_INVENTORY: i32 = 0;
pub const WINDOW_ID_OFF_HAND: i32 = 119;
pub const WINDOW_ID_ARMOUR: i32 = 120;
pub const WINDOW_ID_UI: i32 = 124;

#[derive(Debug, PartialEq, Eq)]
pub enum InventoryActionSource {
    Container,
    World,
    Creative,
    Todo,
    Unknown(u32),
}

impl From<u32> for InventoryActionSource {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Container,
            2 => Self::World,
            3 => Self::Creative,
            99999 => Self::Todo,
            _ => Self::Unknown(value),
        }
    }
}

#[derive(Debug)]
pub enum TransactionData {
    Normal(NormalTransactionData),
    Mismatch(MismatchTransactionData),
    UseItem(UseItemTransactionData),
    UseItemOnEntity(UseItemOnEntityTransactionData),
    ReleaseItem(ReleaseItemTransactionData),
}

#[derive(Debug, PacketRead)]
pub struct LegacySetItemSlot {
    pub container_id: u8,
    pub slots: Vec<u8>,
}

#[derive(Debug)]
pub struct InventoryAction {
    pub source_type: u32,
    pub window_id: Option<i32>,
    pub source_flags: Option<u32>,
    pub inventory_slot: u32,
    pub old_item: NetworkItemDescriptor,
    pub new_item: NetworkItemDescriptor,
}

impl PacketRead for InventoryAction {
    fn read<R: Read>(buf: &mut R) -> Result<Self, Error> {
        let source_type = VarUInt::read(buf)?.0;

        let _container_presence = bool::read(buf)?;
        let window_id = bool::read(buf)?
            .then(|| i8::read(buf).map(i32::from))
            .transpose()?;
        let _flag_presence = bool::read(buf)?;
        let source_flags = bool::read(buf)?
            .then(|| VarUInt::read(buf).map(|flags| flags.0))
            .transpose()?;

        let inventory_slot = VarUInt::read(buf)?.0;

        let old_item = NetworkItemDescriptor::read_cereal(buf)?;
        let new_item = NetworkItemDescriptor::read_cereal(buf)?;

        Ok(Self {
            source_type,
            window_id,
            source_flags,
            inventory_slot,
            old_item,
            new_item,
        })
    }
}

#[derive(Debug, PacketRead)]
pub struct NormalTransactionData;

#[derive(Debug, PacketRead)]
pub struct MismatchTransactionData;

#[derive(Debug)]
pub struct UseItemTransactionData {
    pub action_type: VarInt,
    pub trigger_type: u8,
    pub block_position: BlockPos,
    pub block_face: i32,
    pub hot_bar_slot: VarInt,
    pub item_in_hand: NetworkItemDescriptor,
    pub player_position: Vector3<f32>,
    pub click_position: Vector3<f32>,
    pub block_runtime_id: VarUInt,
    pub client_prediction: u8,
    pub client_cooldown_state: u8,
}

impl PacketRead for UseItemTransactionData {
    fn read<R: Read>(buf: &mut R) -> Result<Self, Error> {
        Ok(Self {
            action_type: VarInt::read(buf)?,
            trigger_type: u8::read(buf)?,
            block_position: BlockPos::read(buf)?,
            block_face: i32::from(u8::read(buf)?),
            hot_bar_slot: VarInt::read(buf)?,
            item_in_hand: NetworkItemDescriptor::read_cereal(buf)?,
            player_position: Vector3::read(buf)?,
            click_position: Vector3::read(buf)?,
            block_runtime_id: VarUInt::read(buf)?,
            client_prediction: u8::read(buf)?,
            client_cooldown_state: u8::read(buf)?,
        })
    }
}

#[derive(Debug)]
pub struct UseItemOnEntityTransactionData {
    pub target_entity_runtime_id: VarULong,
    pub action_type: VarInt,
    pub hot_bar_slot: VarInt,
    pub item_in_hand: NetworkItemDescriptor,
    pub player_position: Vector3<f32>,
    pub click_position: Vector3<f32>,
}

impl PacketRead for UseItemOnEntityTransactionData {
    fn read<R: Read>(buf: &mut R) -> Result<Self, Error> {
        Ok(Self {
            target_entity_runtime_id: VarULong::read(buf)?,
            action_type: VarInt::read(buf)?,
            hot_bar_slot: VarInt::read(buf)?,
            item_in_hand: NetworkItemDescriptor::read_cereal(buf)?,
            player_position: Vector3::read(buf)?,
            click_position: Vector3::read(buf)?,
        })
    }
}

#[derive(Debug)]
pub struct ReleaseItemTransactionData {
    pub action_type: VarInt,
    pub hot_bar_slot: VarInt,
    pub item_in_hand: NetworkItemDescriptor,
    pub head_position: Vector3<f32>,
}

impl PacketRead for ReleaseItemTransactionData {
    fn read<R: Read>(buf: &mut R) -> Result<Self, Error> {
        Ok(Self {
            action_type: VarInt::read(buf)?,
            hot_bar_slot: VarInt::read(buf)?,
            item_in_hand: NetworkItemDescriptor::read_cereal(buf)?,
            head_position: Vector3::read(buf)?,
        })
    }
}

#[derive(Debug)]
#[packet(30)]
pub struct SInventoryTransaction {
    pub legacy_request_id: VarInt,
    pub legacy_set_item_slots: Vec<LegacySetItemSlot>,
    pub has_value: bool,
    pub actions: Vec<InventoryAction>,
    pub transaction_type: VarUInt,
    pub transaction_data: TransactionData,
}

impl PacketRead for SInventoryTransaction {
    fn read<R: Read>(buf: &mut R) -> Result<Self, Error> {
        let legacy_request_id = VarInt::read(buf)?;

        let has_legacy_slots = bool::read(buf)?;
        let mut legacy_set_item_slots = Vec::new();
        if has_legacy_slots {
            let len = VarUInt::read(buf)?.0;
            for _ in 0..len {
                legacy_set_item_slots.push(LegacySetItemSlot::read(buf)?);
            }
        }

        let transaction_type = if bool::read(buf)? {
            VarUInt::read(buf)?
        } else {
            VarUInt(0)
        };

        let has_value = bool::read(buf)?;
        let mut actions = Vec::new();
        if has_value {
            let actions_len = VarUInt::read(buf)?.0;
            for _ in 0..actions_len {
                actions.push(InventoryAction::read(buf)?);
            }
        }

        let transaction_data = match transaction_type.0 {
            0 => TransactionData::Normal(NormalTransactionData::read(buf)?),
            1 => TransactionData::Mismatch(MismatchTransactionData::read(buf)?),
            2 => TransactionData::UseItem(UseItemTransactionData::read(buf)?),
            3 => TransactionData::UseItemOnEntity(UseItemOnEntityTransactionData::read(buf)?),
            4 => TransactionData::ReleaseItem(ReleaseItemTransactionData::read(buf)?),
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Unknown inventory transaction type: {}", transaction_type.0),
                ));
            }
        };

        Ok(Self {
            legacy_request_id,
            legacy_set_item_slots,
            has_value,
            actions,
            transaction_type,
            transaction_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use crate::serial::PacketRead;

    use super::{SInventoryTransaction, TransactionData};

    #[test]
    fn reads_cereal_attack_transaction_without_losing_alignment() {
        let mut bytes = vec![
            0, // legacy request id
            0, // no legacy slots
            1, 3, // transaction type present: item use on actor
            1, 0, // actions present, zero actions
            1, // target runtime id
            2, // zig-zag encoded attack action (1)
            0, // hotbar slot
            // Empty ItemV4: i16 id, u16 count, metadata, optional net id,
            // block runtime id, extra-data length.
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        for value in [1.0_f32, 2.0, 3.0, 0.25, 0.5, 0.75] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        let packet = SInventoryTransaction::read(&mut Cursor::new(bytes)).unwrap();
        assert!(packet.has_value);
        assert!(packet.actions.is_empty());

        let TransactionData::UseItemOnEntity(data) = packet.transaction_data else {
            panic!("expected item-use-on-actor transaction");
        };
        assert_eq!(data.target_entity_runtime_id.0, 1);
        assert_eq!(data.action_type.0, 1);
        assert_eq!(data.player_position.x, 1.0);
        assert_eq!(data.player_position.y, 2.0);
        assert_eq!(data.player_position.z, 3.0);
        assert_eq!(data.click_position.x, 0.25);
        assert_eq!(data.click_position.y, 0.5);
        assert_eq!(data.click_position.z, 0.75);
    }

    #[test]
    fn reads_normal_action_with_nested_presence_markers() {
        let mut bytes = vec![
            0, // legacy request id
            0, // no legacy slots
            1, 0, // transaction type present: normal
            1, 1, // actions present, one action
            0, // container source
            1, // container presence
            1, 0, // window id option present: inventory
            0, // flags presence
            0, // flags option absent
            0, // inventory slot
        ];
        bytes.extend_from_slice(&[0; 8]); // old empty ItemV4
        bytes.extend_from_slice(&[0; 8]); // new empty ItemV4
        bytes.push(0x7f);

        let mut cursor = Cursor::new(bytes);
        let packet = SInventoryTransaction::read(&mut cursor).unwrap();
        assert!(packet.has_value);
        assert_eq!(packet.actions.len(), 1);
        assert_eq!(packet.actions[0].window_id, Some(0));
        assert_eq!(packet.actions[0].source_flags, None);

        let mut trailing = [0];
        cursor.read_exact(&mut trailing).unwrap();
        assert_eq!(trailing[0], 0x7f);
    }
}
