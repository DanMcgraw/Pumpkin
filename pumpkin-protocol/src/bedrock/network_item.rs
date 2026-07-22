use std::io::{Error, Read, Write};
use std::num::NonZeroI32;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::{
    data_component::DataComponent,
    item::{BedrockItem, JavaToBedrockItemMapping},
};
use pumpkin_nbt::{Nbt, compound::NbtCompound, tag::NbtTag};
use pumpkin_util::text::TextComponent;
use pumpkin_util::translation::Locale;

use crate::{
    codec::{var_int::VarInt, var_uint::VarUInt},
    serial::{PacketRead, PacketWrite},
};

/// Translates the Java item components Bedrock reads from the legacy item NBT
/// payload. Without this `display` compound, Bedrock renders only the vanilla
/// item name even though Java clients receive the custom name and lore.
fn component_data(stack: &ItemStack, id: DataComponent) -> Option<NbtTag> {
    if let Some((_, component)) = stack.patch.iter().find(|(patch_id, _)| *patch_id == id) {
        return component.as_ref().map(|component| component.write_data());
    }

    stack
        .item
        .components
        .iter()
        .find(|(component_id, _)| *component_id == id)
        .map(|(_, component)| component.write_data())
}

fn component_text(tag: &NbtTag) -> Option<TextComponent> {
    pumpkin_data::data_component_impl::text_component_from_nbt(tag)
}

fn bedrock_item_nbt(stack: &ItemStack) -> Nbt {
    // Native plugins are separate dynamic libraries, so their concrete component
    // types have different Rust TypeIds from Pumpkin's copies. Read the stable NBT
    // representation instead of downcasting the plugin-owned trait object.
    let custom_name = component_data(stack, DataComponent::CustomName)
        .as_ref()
        .and_then(component_text)
        .map(|component| component.0.to_bedrock_legacy(Locale::EnUs));
    let lore = component_data(stack, DataComponent::Lore).and_then(|tag| {
        tag.extract_list().map(|lines| {
            lines
                .iter()
                .filter_map(component_text)
                .map(|line| line.0.to_bedrock_legacy(Locale::EnUs))
                .collect::<Vec<_>>()
        })
    });

    if custom_name.is_none() && lore.as_ref().is_none_or(Vec::is_empty) {
        return Nbt::default();
    }

    let mut display = NbtCompound::new();
    if let Some(custom_name) = custom_name {
        display.put_string("Name", custom_name);
    }
    if let Some(lore) = lore.filter(|lines| !lines.is_empty()) {
        display.put_list(
            "Lore",
            lore.into_iter()
                .map(|line| NbtTag::String(line.into()))
                .collect(),
        );
    }

    let mut root = NbtCompound::new();
    root.put_compound("display", display);
    Nbt::new(String::new(), root)
}

fn item_v4_extra_data(stack: &ItemStack, is_shield: bool) -> Vec<u8> {
    let nbt = bedrock_item_nbt(stack);
    let mut extra_data = Vec::new();
    if nbt.is_empty() {
        extra_data.extend_from_slice(&0i16.to_le_bytes());
    } else {
        extra_data.extend_from_slice(&(-1i16).to_le_bytes());
        extra_data.push(1);
        extra_data.extend_from_slice(&nbt.write_little_endian());
    }
    extra_data.extend_from_slice(&0u32.to_le_bytes()); // can place on
    extra_data.extend_from_slice(&0u32.to_le_bytes()); // can destroy
    if is_shield {
        extra_data.extend_from_slice(&0i64.to_le_bytes());
    }
    extra_data
}

#[derive(Default, Clone, Debug)]
pub struct NetworkItemDescriptor {
    // I hate mojang
    // https://mojang.github.io/bedrock-protocol-docs/html/NetworkItemInstanceDescriptor.html
    pub id: VarInt,
    pub stack_size: u16,
    pub aux_value: VarUInt,
    pub block_runtime_id: VarInt,

    // remainder is expansion of `User Data Buffer` (ItemInstanceUserData)
    pub nbt_data: Nbt,
    pub place_on_blocks: Vec<String>,
    pub destroy_blocks: Vec<String>,

    pub shield_blocking_tick: i64,
}

impl PacketWrite for NetworkItemDescriptor {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.write_with_net_id(writer, None)
    }
}

impl PacketRead for NetworkItemDescriptor {
    fn read<R: Read>(buf: &mut R) -> Result<Self, Error> {
        let id = VarInt::read(buf)?;
        if id.0 == 0 {
            return Ok(Self::default());
        }
        let stack_size = u16::read(buf)?;
        let aux_value = VarUInt::read(buf)?;

        let has_net_id = bool::read(buf)?;
        if has_net_id {
            let _variant = VarUInt::read(buf)?;
            let _net_id = VarInt::read(buf)?;
        }

        let block_runtime_id = VarInt::read(buf)?;

        let user_data_len = VarUInt::read(buf)?.0;
        let mut user_data = vec![0u8; user_data_len as usize];
        buf.read_exact(&mut user_data)?;

        Ok(Self {
            id,
            stack_size,
            aux_value,
            block_runtime_id,
            ..Default::default()
        })
    }
}

impl NetworkItemDescriptor {
    /// Reads the Cereal representation used by server-bound inventory
    /// transactions since Bedrock protocol 985.
    ///
    /// Unlike the legacy representation, the item id and stack size are fixed
    /// width and every field is present even when the item id is zero.
    pub fn read_cereal<R: Read>(buf: &mut R) -> Result<Self, Error> {
        let id = i16::read(buf)?;
        let stack_size = u16::read(buf)?;
        let aux_value = VarUInt::read(buf)?;

        let has_net_id = bool::read(buf)?;
        if has_net_id {
            let _variant = VarUInt::read(buf)?;
            let _net_id = VarInt::read(buf)?;
        }

        let block_runtime_id = VarUInt::read(buf)?;
        let user_data_len = VarUInt::read(buf)?.0;
        let mut user_data = vec![0u8; user_data_len as usize];
        buf.read_exact(&mut user_data)?;

        Ok(Self {
            id: VarInt(i32::from(id)),
            stack_size,
            aux_value,
            block_runtime_id: VarInt(block_runtime_id.0 as i32),
            ..Default::default()
        })
    }

    #[allow(clippy::option_option)]
    fn write_with_net_id<W: Write>(
        &self,
        writer: &mut W,
        net_id: Option<Option<VarInt>>,
    ) -> Result<(), Error> {
        self.id.write(writer)?;
        if self.id.0 != 0 {
            self.stack_size.write(writer)?;
            self.aux_value.write(writer)?;

            if let Some(id) = net_id {
                id.write(writer)?;
            }

            self.block_runtime_id.write(writer)?;

            let mut buf = Vec::new();

            if self.nbt_data.is_empty() {
                (0i16).write(&mut buf)?;
            } else {
                (-1i16).write(&mut buf)?;
                (1i8).write(&mut buf)?;

                self.nbt_data
                    .clone()
                    .write_to_writer_little_endian(&mut buf)?;
            }

            (self.place_on_blocks.len() as u32).write(&mut buf)?;
            self.place_on_blocks.write(&mut buf)?;

            (self.destroy_blocks.len() as u32).write(&mut buf)?;
            self.destroy_blocks.write(&mut buf)?;

            if self.id.0 == (BedrockItem::SHIELD.id as i32) {
                self.shield_blocking_tick.write(&mut buf)?;
            }

            VarUInt(buf.len() as u32).write(writer)?;
            writer.write_all(&buf)?;
        }
        Ok(())
    }
}

impl From<&ItemStack> for NetworkItemDescriptor {
    fn from(stack: &ItemStack) -> Self {
        if stack.is_empty() {
            Self::default()
        } else {
            JavaToBedrockItemMapping::from_java_item_id(stack.get_item().id).map_or(
                Self::default(),
                |mapping| Self {
                    id: VarInt::from(mapping.bedrock_item.id),
                    stack_size: stack.item_count as u16,
                    aux_value: VarUInt(mapping.bedrock_data),
                    block_runtime_id: VarInt::from(mapping.bedrock_block_state),
                    nbt_data: bedrock_item_nbt(stack),
                    place_on_blocks: Vec::default(),
                    destroy_blocks: Vec::default(),
                    shield_blocking_tick: 0,
                },
            )
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct ItemStackWrapper {
    pub id: i16,
    pub stack_size: u16,
    pub aux_value: VarUInt,
    pub block_runtime_id: VarInt,
    pub nbt_data: Nbt,
    pub place_on_blocks: Vec<String>,
    pub destroy_blocks: Vec<String>,
    pub shield_blocking_tick: i64,
    pub net_id: Option<NonZeroI32>,
}

impl PacketWrite for ItemStackWrapper {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        VarInt(self.id as i32).write(writer)?;

        if self.id != 0 {
            self.stack_size.write(writer)?;

            self.aux_value.write(writer)?;

            self.net_id.is_some().write(writer)?;

            if let Some(id) = self.net_id {
                VarInt(id.get()).write(writer)?;
            }

            self.block_runtime_id.write(writer)?;

            let mut buf = Vec::new();

            if self.nbt_data.is_empty() {
                (0i16).write(&mut buf)?;
            } else {
                (-1i16).write(&mut buf)?;
                (1i8).write(&mut buf)?;

                self.nbt_data
                    .clone()
                    .write_to_writer_little_endian(&mut buf)?;
            }

            (self.place_on_blocks.len() as u32).write(&mut buf)?;
            for block in &self.place_on_blocks {
                block.write(&mut buf)?;
            }

            (self.destroy_blocks.len() as u32).write(&mut buf)?;
            for block in &self.destroy_blocks {
                block.write(&mut buf)?;
            }

            if self.id == BedrockItem::SHIELD.id {
                self.shield_blocking_tick.write(&mut buf)?;
            }

            VarUInt(buf.len() as u32).write(writer)?;
            writer.write_all(&buf)?;
        }
        Ok(())
    }
}

impl PacketRead for ItemStackWrapper {
    fn read<R: Read>(buf: &mut R) -> Result<Self, Error> {
        let id = i16::read(buf)?;
        if id == 0 {
            return Ok(Self::default());
        }
        let stack_size = u16::read(buf)?;
        let aux_value = VarUInt::read(buf)?;

        let has_net_id = bool::read(buf)?;
        let net_id = if has_net_id {
            let stack_id = VarInt::read(buf)?;
            NonZeroI32::new(stack_id.0)
        } else {
            None
        };

        let block_runtime_id = VarInt::read(buf)?;

        let user_data_len = VarUInt::read(buf)?.0;
        let mut user_data = vec![0u8; user_data_len as usize];
        buf.read_exact(&mut user_data)?;

        Ok(Self {
            id,
            stack_size,
            aux_value,
            block_runtime_id,
            net_id,
            ..Default::default()
        })
    }
}

impl ItemStackWrapper {
    /// Builds the item carried by an `AddItemActor` packet.
    ///
    /// Network stack IDs belong to a session's inventory transaction state.
    /// Geyser omits them for item actors, so a dropped item must not inherit the
    /// server-side stack UID used by inventory snapshots.
    #[must_use]
    pub fn from_world_item(stack: &ItemStack) -> Self {
        let mut item = Self::from(stack);
        item.net_id = None;
        item
    }
}

impl From<&ItemStack> for ItemStackWrapper {
    fn from(stack: &ItemStack) -> Self {
        if stack.is_empty() {
            Self::default()
        } else {
            JavaToBedrockItemMapping::from_java_item_id(stack.get_item().id).map_or(
                Self::default(),
                |mapping| Self {
                    id: mapping.bedrock_item.id,
                    stack_size: stack.item_count as u16,
                    aux_value: VarUInt(mapping.bedrock_data),
                    block_runtime_id: VarInt::from(mapping.bedrock_block_state),
                    nbt_data: bedrock_item_nbt(stack),
                    place_on_blocks: Vec::default(),
                    destroy_blocks: Vec::default(),
                    shield_blocking_tick: 0,
                    net_id: Some(stack.uid),
                },
            )
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct NetworkItemStackDescriptor {
    pub id: i16,
    pub stack_size: u16,
    pub aux_value: VarUInt,
    pub block_runtime_id: VarUInt,
    pub extra_data: Vec<u8>,
    pub net_id_variant: Option<VarUInt>,
    pub net_id: Option<NonZeroI32>,
}

impl PacketWrite for NetworkItemStackDescriptor {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        self.id.write(writer)?;

        self.stack_size.write(writer)?;
        self.aux_value.write(writer)?;

        self.net_id.is_some().write(writer)?;
        if let Some(id) = self.net_id {
            let variant = self.net_id_variant.unwrap_or(VarUInt(0));
            variant.write(writer)?;
            VarInt(id.get()).write(writer)?;
        }

        self.block_runtime_id.write(writer)?;

        VarUInt(self.extra_data.len() as u32).write(writer)?;
        writer.write_all(&self.extra_data)?;

        Ok(())
    }
}

impl PacketRead for NetworkItemStackDescriptor {
    fn read<R: Read>(buf: &mut R) -> Result<Self, Error> {
        let id = i16::read(buf)?;

        let stack_size = u16::read(buf)?;
        let aux_value = VarUInt::read(buf)?;

        let has_net_id = bool::read(buf)?;
        let (net_id_variant, net_id) = if has_net_id {
            let variant = VarUInt::read(buf)?;
            let stack_id = VarInt::read(buf)?;
            (Some(variant), NonZeroI32::new(stack_id.0))
        } else {
            (None, None)
        };

        let block_runtime_id = VarUInt::read(buf)?;

        let extra_data_len = VarUInt::read(buf)?.0;
        let mut extra_data = vec![0u8; extra_data_len as usize];
        buf.read_exact(&mut extra_data)?;

        Ok(Self {
            id,
            stack_size,
            aux_value,
            block_runtime_id,
            extra_data,
            net_id_variant,
            net_id,
        })
    }
}

impl From<&ItemStack> for NetworkItemStackDescriptor {
    fn from(stack: &ItemStack) -> Self {
        if stack.is_empty() {
            Self::default()
        } else {
            JavaToBedrockItemMapping::from_java_item_id(stack.get_item().id).map_or(
                Self::default(),
                |mapping| {
                    // ItemV4's opaque network-user-data buffer still contains
                    // the legacy payload: NBT marker followed by fixed-width
                    // can-place and can-destroy list lengths.
                    let extra_data = item_v4_extra_data(
                        stack,
                        mapping.bedrock_item.id == BedrockItem::SHIELD.id,
                    );

                    Self {
                        id: mapping.bedrock_item.id,
                        stack_size: stack.item_count as u16,
                        aux_value: VarUInt(mapping.bedrock_data),
                        block_runtime_id: VarUInt(mapping.bedrock_block_state as u32),
                        extra_data,
                        net_id_variant: Some(VarUInt(0)),
                        net_id: Some(stack.uid),
                    }
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::io::{Cursor, Read};

    use pumpkin_data::{
        data_component::DataComponent,
        data_component_impl::{DataComponentImpl, text_component_to_nbt},
        item::{BedrockItem, Item, JavaToBedrockItemMapping},
        item_stack::ItemStack,
    };
    use pumpkin_nbt::tag::NbtTag;
    use pumpkin_util::text::{TextComponent, color::NamedColor};

    use crate::serial::PacketWrite;

    use super::{
        ContainerName, ItemStackWrapper, NetworkItemDescriptor, NetworkItemStackDescriptor,
        bedrock_item_nbt,
    };

    #[derive(Clone)]
    struct ForeignComponent {
        id: DataComponent,
        data: NbtTag,
    }

    impl DataComponentImpl for ForeignComponent {
        fn write_data(&self) -> NbtTag {
            self.data.clone()
        }

        fn equal(&self, other: &dyn DataComponentImpl) -> bool {
            self.id == other.get_self_enum() && self.data == other.write_data()
        }

        fn get_enum() -> DataComponent
        where
            Self: Sized,
        {
            DataComponent::CustomName
        }

        fn get_self_enum(&self) -> DataComponent {
            self.id
        }

        fn to_dyn(self) -> Box<dyn DataComponentImpl> {
            Box::new(self)
        }

        fn clone_dyn(&self) -> Box<dyn DataComponentImpl> {
            Box::new(self.clone())
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_mut_any(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn cereal_empty_item_consumes_the_complete_descriptor() {
        // Fixed i16 id, fixed u16 count, aux, net-id presence, block id,
        // user-data length, followed by a byte belonging to the next field.
        let mut cursor = Cursor::new([0, 0, 0, 0, 0, 0, 0, 0, 0x7f]);

        let item = NetworkItemDescriptor::read_cereal(&mut cursor).unwrap();
        assert_eq!(item.id.0, 0);
        assert_eq!(item.stack_size, 0);

        let mut next = [0];
        cursor.read_exact(&mut next).unwrap();
        assert_eq!(next[0], 0x7f);
    }

    #[test]
    fn item_v4_has_complete_empty_user_data() {
        let stack = ItemStack::new(1, &Item::STONE);
        let item = NetworkItemStackDescriptor::from(&stack);

        assert_eq!(item.extra_data, vec![0; 10]);
    }

    #[test]
    fn item_v4_translates_custom_name_and_lore_to_bedrock_display_nbt() {
        let mut stack = ItemStack::new(1, &Item::FEATHER);
        stack.set_custom_name("Acrobatics - Level 4".to_owned());
        stack.set_lore(vec![
            TextComponent::text("Progress: 20/100 XP (20%)").color_named(NamedColor::Yellow),
            TextComponent::text("Total XP: 320").color_named(NamedColor::Gray),
        ]);

        let nbt = bedrock_item_nbt(&stack);
        let display = nbt.get_compound("display").unwrap();
        assert_eq!(display.get_string("Name"), Some("Acrobatics - Level 4"));
        let lore = display.get_list("Lore").unwrap();
        assert_eq!(lore.len(), 2);
        assert_eq!(
            lore[0].extract_string(),
            Some("§eProgress: 20/100 XP (20%)")
        );
        assert_eq!(lore[1].extract_string(), Some("§7Total XP: 320"));

        let item = NetworkItemStackDescriptor::from(&stack);
        assert_eq!(&item.extra_data[..6], &[0xff, 0xff, 1, 0x0a, 0x00, 0x00]);
        assert!(item.extra_data.len() > 10);
    }

    #[test]
    fn item_v4_normalizes_native_plugin_name_and_lore_without_downcasting() {
        let mut stack = ItemStack::new(1, &Item::FEATHER);
        let name = TextComponent::text("Acrobatics - Level 4");
        let lore = TextComponent::text("Progress: 20/100 XP (20%)").color_named(NamedColor::Yellow);
        stack.patch = vec![
            (
                DataComponent::CustomName,
                Some(Box::new(ForeignComponent {
                    id: DataComponent::CustomName,
                    data: text_component_to_nbt(&name),
                })),
            ),
            (
                DataComponent::Lore,
                Some(Box::new(ForeignComponent {
                    id: DataComponent::Lore,
                    data: NbtTag::List(vec![text_component_to_nbt(&lore)]),
                })),
            ),
        ];

        let nbt = bedrock_item_nbt(&stack);
        let display = nbt.get_compound("display").unwrap();
        assert_eq!(display.get_string("Name"), Some("Acrobatics - Level 4"));
        let lore = display.get_list("Lore").unwrap();
        assert_eq!(
            lore[0].extract_string(),
            Some("§eProgress: 20/100 XP (20%)")
        );
    }

    #[test]
    fn dropped_apple_uses_runtime_mapping_without_inventory_network_id() {
        let stack = ItemStack::new(1, &Item::APPLE);
        let item = ItemStackWrapper::from_world_item(&stack);

        assert_eq!(item.id, 285);
        assert_eq!(item.stack_size, 1);
        assert!(item.net_id.is_none());

        let mut bytes = Vec::new();
        item.write(&mut bytes).unwrap();
        assert_eq!(
            bytes,
            [
                0xba, 0x04, // signed VarInt runtime ID 285
                0x01, 0x00, // stack size
                0x00, // aux value
                0x00, // no inventory network-stack ID
                0x00, // no block runtime ID
                0x0a, // ten-byte empty user-data payload
                0x00, 0x00, // no NBT
                0x00, 0x00, 0x00, 0x00, // empty can-place list
                0x00, 0x00, 0x00, 0x00, // empty can-destroy list
            ]
        );
    }

    #[test]
    fn apple_runtime_mapping_round_trips_through_the_negotiated_palette() {
        let to_bedrock = JavaToBedrockItemMapping::from_java_item_id(Item::APPLE.id).unwrap();
        assert_eq!(to_bedrock.bedrock_item.id, BedrockItem::APPLE.id);
        assert_eq!(to_bedrock.bedrock_item.registry_key, "minecraft:apple");
        assert!(to_bedrock.bedrock_item.component_based);
        assert!(!to_bedrock.bedrock_item.definition_components.is_empty());

        let to_java = JavaToBedrockItemMapping::from_bedrock(
            to_bedrock.bedrock_item.id,
            to_bedrock.bedrock_data,
        )
        .unwrap();
        assert_eq!(to_java.java_item.id, Item::APPLE.id);
    }

    #[test]
    fn protocol_1001_container_names_use_wire_order() {
        let expected = [
            (18, ContainerName::RecipeSearch),
            (28, ContainerName::HotBar),
            (29, ContainerName::Inventory),
            (34, ContainerName::Offhand),
            (59, ContainerName::Cursor),
            (63, ContainerName::Dynamic),
            (64, ContainerName::RecipeFood),
            (65, ContainerName::RecipeBlocks),
            (66, ContainerName::RecipeFurnaceItems),
        ];

        for (wire_id, container_name) in expected {
            assert_eq!(container_name as u8, wire_id);
            assert_eq!(ContainerName::try_from(wire_id).unwrap(), container_name);
        }
    }
}

#[derive(PacketWrite, PacketRead, Clone, Debug, PartialEq, Eq)]
pub struct FullContainerName {
    pub container_name: ContainerName,
    pub dynamic_id: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ContainerName {
    AnvilInput,
    AnvilMaterial,
    AnvilResultPreview,
    SmithingTableInput,
    SmithingTableMaterial,
    SmithingTableResultPreview,
    Armor,
    LevelEntity,
    BeaconPayment,
    BrewingStandInput,
    BrewingStandResult,
    BrewingStandFuel,
    CombinedHotBarAndInventory,
    CraftingInput,
    CraftingOutputPreview,
    RecipeConstruction,
    RecipeNature,
    RecipeItems,
    RecipeSearch,
    RecipeSearchBar,
    RecipeEquipment,
    RecipeBook,
    EnchantingInput,
    EnchantingMaterial,
    FurnaceFuel,
    FurnaceIngredient,
    FurnaceResult,
    HorseEquip,
    HotBar,
    Inventory,
    ShulkerBox,
    TradeIngredient1,
    TradeIngredient2,
    TradeResultPreview,
    Offhand,
    CompoundCreatorInput,
    CompoundCreatorOutputPreview,
    ElementConstructorOutputPreview,
    MaterialReducerInput,
    MaterialReducerOutput,
    LabTableInput,
    LoomInput,
    LoomDye,
    LoomMaterial,
    LoomResultPreview,
    BlastFurnaceIngredient,
    SmokerIngredient,
    Trade2Ingredient1,
    Trade2Ingredient2,
    Trade2ResultPreview,
    GrindstoneInput,
    GrindstoneAdditional,
    GrindstoneResultPreview,
    StonecutterInput,
    StonecutterResultPreview,
    CartographyInput,
    CartographyAdditional,
    CartographyResultPreview,
    Barrel,
    Cursor,
    CreatedOutput,
    SmithingTableTemplate,
    CrafterLevelEntity,
    Dynamic,
    RecipeFood,
    RecipeBlocks,
    RecipeFurnaceItems,
}

impl TryFrom<u8> for ContainerName {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::AnvilInput),
            1 => Ok(Self::AnvilMaterial),
            2 => Ok(Self::AnvilResultPreview),
            3 => Ok(Self::SmithingTableInput),
            4 => Ok(Self::SmithingTableMaterial),
            5 => Ok(Self::SmithingTableResultPreview),
            6 => Ok(Self::Armor),
            7 => Ok(Self::LevelEntity),
            8 => Ok(Self::BeaconPayment),
            9 => Ok(Self::BrewingStandInput),
            10 => Ok(Self::BrewingStandResult),
            11 => Ok(Self::BrewingStandFuel),
            12 => Ok(Self::CombinedHotBarAndInventory),
            13 => Ok(Self::CraftingInput),
            14 => Ok(Self::CraftingOutputPreview),
            15 => Ok(Self::RecipeConstruction),
            16 => Ok(Self::RecipeNature),
            17 => Ok(Self::RecipeItems),
            18 => Ok(Self::RecipeSearch),
            19 => Ok(Self::RecipeSearchBar),
            20 => Ok(Self::RecipeEquipment),
            21 => Ok(Self::RecipeBook),
            22 => Ok(Self::EnchantingInput),
            23 => Ok(Self::EnchantingMaterial),
            24 => Ok(Self::FurnaceFuel),
            25 => Ok(Self::FurnaceIngredient),
            26 => Ok(Self::FurnaceResult),
            27 => Ok(Self::HorseEquip),
            28 => Ok(Self::HotBar),
            29 => Ok(Self::Inventory),
            30 => Ok(Self::ShulkerBox),
            31 => Ok(Self::TradeIngredient1),
            32 => Ok(Self::TradeIngredient2),
            33 => Ok(Self::TradeResultPreview),
            34 => Ok(Self::Offhand),
            35 => Ok(Self::CompoundCreatorInput),
            36 => Ok(Self::CompoundCreatorOutputPreview),
            37 => Ok(Self::ElementConstructorOutputPreview),
            38 => Ok(Self::MaterialReducerInput),
            39 => Ok(Self::MaterialReducerOutput),
            40 => Ok(Self::LabTableInput),
            41 => Ok(Self::LoomInput),
            42 => Ok(Self::LoomDye),
            43 => Ok(Self::LoomMaterial),
            44 => Ok(Self::LoomResultPreview),
            45 => Ok(Self::BlastFurnaceIngredient),
            46 => Ok(Self::SmokerIngredient),
            47 => Ok(Self::Trade2Ingredient1),
            48 => Ok(Self::Trade2Ingredient2),
            49 => Ok(Self::Trade2ResultPreview),
            50 => Ok(Self::GrindstoneInput),
            51 => Ok(Self::GrindstoneAdditional),
            52 => Ok(Self::GrindstoneResultPreview),
            53 => Ok(Self::StonecutterInput),
            54 => Ok(Self::StonecutterResultPreview),
            55 => Ok(Self::CartographyInput),
            56 => Ok(Self::CartographyAdditional),
            57 => Ok(Self::CartographyResultPreview),
            58 => Ok(Self::Barrel),
            59 => Ok(Self::Cursor),
            60 => Ok(Self::CreatedOutput),
            61 => Ok(Self::SmithingTableTemplate),
            62 => Ok(Self::CrafterLevelEntity),
            63 => Ok(Self::Dynamic),
            64 => Ok(Self::RecipeFood),
            65 => Ok(Self::RecipeBlocks),
            66 => Ok(Self::RecipeFurnaceItems),
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid ContainerName ID: {value}"),
            )),
        }
    }
}

impl PacketWrite for ContainerName {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        (*self as u8).write(writer)?;
        Ok(())
    }
}

impl PacketRead for ContainerName {
    fn read<R: Read>(buf: &mut R) -> Result<Self, Error> {
        let value = u8::read(buf)?;
        Self::try_from(value)
    }
}

#[derive(Debug, Clone)]
pub struct NetworkItemStack {
    pub id: VarInt,
    pub count: u16,
    pub aux_value: VarUInt,
    pub block_runtime_id: VarInt,
    pub extra_data: Vec<u8>,
}

impl PacketRead for NetworkItemStack {
    fn read<R: Read>(buf: &mut R) -> Result<Self, Error> {
        let id = VarInt::read(buf)?;
        if id.0 == 0 {
            return Ok(Self {
                id,
                count: 0,
                aux_value: VarUInt(0),
                block_runtime_id: VarInt(0),
                extra_data: Vec::new(),
            });
        }
        let count = u16::read(buf)?;
        let aux_value = VarUInt::read(buf)?;
        let block_runtime_id = VarInt::read(buf)?;

        let extra_data_len = VarUInt::read(buf)?.0;
        let mut extra_data = vec![0u8; extra_data_len as usize];
        buf.read_exact(&mut extra_data)?;

        Ok(Self {
            id,
            count,
            aux_value,
            block_runtime_id,
            extra_data,
        })
    }
}
