use pumpkin_macros::packet;

use crate::{codec::var_uint::VarUInt, serial::PacketWrite};

#[derive(PacketWrite)]
#[packet(48)]
pub struct CPlayerHotbar {
    pub selected_slot: VarUInt,
    pub container_id: u8,
    pub should_select_block: bool,
}

#[cfg(test)]
mod tests {
    use crate::{codec::var_uint::VarUInt, serial::PacketWrite};

    use super::CPlayerHotbar;

    #[test]
    fn selected_hotbar_slot_has_protocol_1001_wire_layout() {
        let mut bytes = Vec::new();
        CPlayerHotbar {
            selected_slot: VarUInt(7),
            container_id: 0,
            should_select_block: false,
        }
        .write(&mut bytes)
        .unwrap();

        assert_eq!(bytes, [7, 0, 0]);
    }
}
