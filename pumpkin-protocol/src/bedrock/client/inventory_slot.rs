use crate::{
    bedrock::network_item::{FullContainerName, NetworkItemStackDescriptor},
    codec::var_uint::VarUInt,
    serial::PacketWrite,
};
use pumpkin_macros::packet;
use std::io::{Error, Write};

#[packet(50)]
pub struct CInventorySlot {
    pub window_id: VarUInt,
    pub inventory_slot: VarUInt,
    pub container_name: Option<FullContainerName>,
    pub storage: Option<NetworkItemStackDescriptor>,
    pub item: NetworkItemStackDescriptor,
}

impl PacketWrite for CInventorySlot {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.window_id.write(writer)?;
        self.inventory_slot.write(writer)?;

        self.container_name.is_some().write(writer)?;
        if let Some(container_name) = &self.container_name {
            container_name.write(writer)?;
        }

        self.storage.is_some().write(writer)?;
        if let Some(storage) = &self.storage {
            storage.write(writer)?;
        }

        self.item.write(writer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        bedrock::network_item::NetworkItemStackDescriptor, codec::var_uint::VarUInt,
        serial::PacketWrite,
    };

    use super::CInventorySlot;

    #[test]
    fn cursor_snapshot_uses_ui_window_without_stack_request_container_name() {
        let mut bytes = Vec::new();
        CInventorySlot {
            window_id: VarUInt(124),
            inventory_slot: VarUInt(0),
            container_name: None,
            storage: None,
            item: NetworkItemStackDescriptor::default(),
        }
        .write(&mut bytes)
        .unwrap();

        assert!(bytes.starts_with(&[124, 0, 0, 0]));
    }
}
