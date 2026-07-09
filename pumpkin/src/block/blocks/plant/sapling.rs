use pumpkin_data::BlockStateId;
use pumpkin_data::block_properties::BlockProperties;
use pumpkin_data::placed_feature::PlacedFeature;
use pumpkin_macros::pumpkin_block_from_tag;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::world::BlockFlags;
use std::sync::Arc;

use crate::block::blocks::plant::PlantBlockBase;
use crate::block::{
    BlockBehaviour, BlockFuture, CanPlaceAtArgs, GetStateForNeighborUpdateArgs, RandomTickArgs,
};
use crate::plugin::api::events::block::{
    block_grow::fire_block_grow, structure_grow::StructureGrowEvent,
};
use crate::world::World;

type SaplingProperties = pumpkin_data::block_properties::OakSaplingLikeProperties;

#[pumpkin_block_from_tag("minecraft:saplings")]
pub struct SaplingBlock;

impl SaplingBlock {
    const fn select_tree_feature(block: &'static pumpkin_data::Block) -> Option<PlacedFeature> {
        // TODO: detect 2x2 arrangements for spruce, jungle, and dark oak.
        Some(match block.id {
            pumpkin_data::BlockId::OAK_SAPLING => PlacedFeature::OakChecked,
            pumpkin_data::BlockId::SPRUCE_SAPLING => PlacedFeature::SpruceChecked,
            pumpkin_data::BlockId::BIRCH_SAPLING => PlacedFeature::BirchChecked,
            pumpkin_data::BlockId::JUNGLE_SAPLING => PlacedFeature::JungleTree,
            pumpkin_data::BlockId::ACACIA_SAPLING => PlacedFeature::AcaciaChecked,
            pumpkin_data::BlockId::DARK_OAK_SAPLING => PlacedFeature::DarkOakChecked,
            pumpkin_data::BlockId::CHERRY_SAPLING => PlacedFeature::CherryChecked,
            pumpkin_data::BlockId::PALE_OAK_SAPLING => PlacedFeature::PaleOakChecked,
            pumpkin_data::BlockId::MANGROVE_PROPAGULE => PlacedFeature::MangroveChecked,
            _ => return None,
        })
    }

    async fn generate(&self, world: &Arc<World>, pos: &BlockPos) {
        let (block, state) = world.get_block_and_state_id(pos);
        let mut props = SaplingProperties::from_state_id(state, block);
        if props.stage == 0 {
            props.stage = 1;
            let new_state_id = props.to_state_id(block);
            let Some(new_state_id) = fire_block_grow(world, *pos, new_state_id).await else {
                return;
            };
            world
                .set_block_state(pos, new_state_id, BlockFlags::NOTIFY_ALL)
                .await;
        } else {
            let Some(server) = world.server.upgrade() else {
                return;
            };
            let Some(placed_feature) = Self::select_tree_feature(block) else {
                return;
            };

            let event = StructureGrowEvent::new(world.clone(), block, *pos, placed_feature);
            let event = server.plugin_manager.fire(event).await;
            if !event.cancelled {
                // TODO: generate the tree/feature in the live world
            }
        }
    }
}

impl BlockBehaviour for SaplingBlock {
    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        <Self as PlantBlockBase>::can_place_at(self, args.block_accessor, args.position)
    }

    fn get_state_for_neighbor_update<'a>(
        &'a self,
        args: GetStateForNeighborUpdateArgs<'a>,
    ) -> BlockFuture<'a, BlockStateId> {
        Box::pin(async move {
            <Self as PlantBlockBase>::get_state_for_neighbor_update(
                self,
                args.world,
                args.position,
                args.state_id,
            )
            .await
        })
    }

    fn random_tick<'a>(&'a self, args: RandomTickArgs<'a>) -> BlockFuture<'a, ()> {
        Box::pin(async move {
            self.generate(args.world, args.position).await;
        })
    }
}

impl PlantBlockBase for SaplingBlock {}
