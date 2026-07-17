use std::sync::Arc;

use pumpkin_data::{
    Block, BlockStateId,
    block_properties::{
        BlockProperties, NetherWartLikeProperties, TorchflowerCropLikeProperties,
        WheatLikeProperties,
    },
};
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::world::BlockFlags;
use thiserror::Error;

use crate::{plugin::api::events::block::block_grow::fire_block_grow, world::World};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrowthCause {
    Plugin,
    BoneMeal,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrowthOutcome {
    pub position: BlockPos,
    pub cause: GrowthCause,
    pub state_before: BlockStateId,
    pub state_after: BlockStateId,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum GrowthError {
    #[error("the target block does not support bounded crop growth")]
    UnsupportedBlock,
    #[error("the target crop is already mature")]
    AlreadyMature,
    #[error("a plugin cancelled the growth transition")]
    Cancelled,
    #[error("the block changed while the growth request was being validated")]
    ConcurrentChange,
    #[error("a handler replaced the candidate with an invalid growth transition")]
    InvalidTransition,
}

pub async fn request_block_growth(
    world: Arc<World>,
    position: BlockPos,
    cause: GrowthCause,
) -> Result<GrowthOutcome, GrowthError> {
    let (block, state_before) = world.get_block_and_state_id(&position);
    let candidate = next_crop_state(block, state_before)?;
    let Some(state_after) = fire_block_grow(&world, position, candidate).await else {
        return Err(GrowthError::Cancelled);
    };
    if state_after != candidate {
        return Err(GrowthError::InvalidTransition);
    }
    if world.get_block_state_id(&position) != state_before {
        return Err(GrowthError::ConcurrentChange);
    }
    world
        .set_block_state(&position, state_after, BlockFlags::NOTIFY_NEIGHBORS)
        .await;
    Ok(GrowthOutcome {
        position,
        cause,
        state_before,
        state_after,
    })
}

fn next_crop_state(block: &Block, state: BlockStateId) -> Result<BlockStateId, GrowthError> {
    if block == &Block::WHEAT || block == &Block::CARROTS || block == &Block::POTATOES {
        let mut properties = WheatLikeProperties::from_state_id(state, block);
        if properties.age >= 7 {
            return Err(GrowthError::AlreadyMature);
        }
        properties.age += 1;
        return Ok(properties.to_state_id(block));
    }
    if block == &Block::BEETROOTS
        || block == &Block::NETHER_WART
        || block == &Block::SWEET_BERRY_BUSH
    {
        let mut properties = NetherWartLikeProperties::from_state_id(state, block);
        if properties.age >= 3 {
            return Err(GrowthError::AlreadyMature);
        }
        properties.age += 1;
        return Ok(properties.to_state_id(block));
    }
    if block == &Block::TORCHFLOWER_CROP {
        let mut properties = TorchflowerCropLikeProperties::from_state_id(state, block);
        return match properties.age {
            0 => {
                properties.age = 1;
                Ok(properties.to_state_id(block))
            }
            1 => Ok(Block::TORCHFLOWER.default_state.id),
            _ => Err(GrowthError::AlreadyMature),
        };
    }
    Err(GrowthError::UnsupportedBlock)
}

#[cfg(test)]
mod tests {
    use super::{GrowthError, next_crop_state};
    use pumpkin_data::{
        Block,
        block_properties::{BlockProperties, WheatLikeProperties},
    };

    #[test]
    fn wheat_request_advances_exactly_one_age() {
        let mut properties =
            WheatLikeProperties::from_state_id(Block::WHEAT.default_state.id, &Block::WHEAT);
        properties.age = 3;
        let before = properties.to_state_id(&Block::WHEAT);
        let after = next_crop_state(&Block::WHEAT, before).unwrap();
        assert_eq!(
            WheatLikeProperties::from_state_id(after, &Block::WHEAT).age,
            4
        );
    }

    #[test]
    fn mature_and_unsupported_blocks_are_rejected() {
        let mut mature =
            WheatLikeProperties::from_state_id(Block::WHEAT.default_state.id, &Block::WHEAT);
        mature.age = 7;
        assert_eq!(
            next_crop_state(&Block::WHEAT, mature.to_state_id(&Block::WHEAT)),
            Err(GrowthError::AlreadyMature)
        );
        assert_eq!(
            next_crop_state(&Block::STONE, Block::STONE.default_state.id),
            Err(GrowthError::UnsupportedBlock)
        );
    }
}
