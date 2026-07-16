use pumpkin_data::{
    Block, BlockDirection, BlockStateId, HorizontalFacingExt,
    block_properties::{AttachFace, BlockProperties, GrindstoneLikeProperties},
};
use pumpkin_inventory::{
    grindstone::GrindstoneScreenHandler,
    player::player_inventory::PlayerInventory,
    screen_handler::{BoxFuture, InventoryPlayer, ScreenHandlerFactory, SharedScreenHandler},
};
use pumpkin_macros::pumpkin_block;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::text::TextComponent;
use pumpkin_world::inventory::SimpleInventory;
use pumpkin_world::world::BlockAccessor;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::block::CanPlaceAtArgs;
use crate::block::registry::BlockActionResult;
use crate::block::{BlockBehaviour, BlockFuture, NormalUseArgs};
use crate::block::{GetStateForNeighborUpdateArgs, OnPlaceArgs};

use super::abstract_wall_mounting::WallMountedBlock;

#[pumpkin_block("minecraft:grindstone")]
pub struct GrindstoneBlock;

impl BlockBehaviour for GrindstoneBlock {
    fn normal_use<'a>(&'a self, args: NormalUseArgs<'a>) -> BlockFuture<'a, BlockActionResult> {
        Box::pin(async move {
            args.player
                .open_handled_screen(&GrindstoneScreenFactory, Some(*args.position))
                .await;
            BlockActionResult::Success
        })
    }

    fn on_place<'a>(&'a self, args: OnPlaceArgs<'a>) -> BlockFuture<'a, BlockStateId> {
        Box::pin(async move {
            let mut props =
                GrindstoneLikeProperties::from_state_id(args.block.default_state.id, args.block);
            (props.face, props.facing) =
                WallMountedBlock::get_placement_face(self, args.player, args.direction);

            props.to_state_id(args.block)
        })
    }

    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        // Use the provided direction, or fallback to the current state's direction if missing
        let direction = args
            .direction
            .unwrap_or_else(|| self.get_direction(args.state.id, args.block));

        WallMountedBlock::can_place_at(self, args.block_accessor, args.position, direction)
    }

    fn get_state_for_neighbor_update<'a>(
        &'a self,
        args: GetStateForNeighborUpdateArgs<'a>,
    ) -> BlockFuture<'a, BlockStateId> {
        Box::pin(async move { WallMountedBlock::get_state_for_neighbor_update(self, args).await })
    }
}

struct GrindstoneScreenFactory;

impl ScreenHandlerFactory for GrindstoneScreenFactory {
    fn create_screen_handler<'a>(
        &'a self,
        sync_id: u8,
        player_inventory: &'a Arc<PlayerInventory>,
        _player: &'a dyn InventoryPlayer,
    ) -> BoxFuture<'a, Option<SharedScreenHandler>> {
        Box::pin(async move {
            let inventory = Arc::new(SimpleInventory::new(3));
            let handler = GrindstoneScreenHandler::new(sync_id, player_inventory, inventory);
            Some(Arc::new(Mutex::new(handler)) as SharedScreenHandler)
        })
    }

    fn get_display_name(&self) -> TextComponent {
        TextComponent::translate("container.grindstone", &[])
    }
}

impl WallMountedBlock for GrindstoneBlock {
    fn can_place_at<'a>(
        &'a self,
        _world: &'a dyn BlockAccessor,
        _pos: &'a BlockPos,
        _direction: BlockDirection,
    ) -> bool {
        true
    }

    fn get_direction(&self, state_id: BlockStateId, block: &Block) -> BlockDirection {
        let props = GrindstoneLikeProperties::from_state_id(state_id, block);
        match props.face {
            AttachFace::Floor => BlockDirection::Up,
            AttachFace::Ceiling => BlockDirection::Down,
            AttachFace::Wall => props.facing.to_block_direction(),
        }
    }
}
