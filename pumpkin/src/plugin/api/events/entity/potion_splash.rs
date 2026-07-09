use std::sync::Arc;

use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::{position::BlockPos, vector3::Vector3};

use crate::entity::EntityBase;

/// Fired when a splash potion impacts and affects entities.
#[cancellable]
#[derive(Event, Clone)]
pub struct PotionSplashEvent {
    /// The potion entity.
    pub entity: Arc<dyn EntityBase>,

    /// The position where the potion impacted.
    pub hit_pos: Vector3<f64>,

    /// The hit block position, if it hit a block.
    pub hit_block: Option<BlockPos>,

    /// The hit entity, if it hit an entity directly.
    pub hit_entity: Option<Arc<dyn EntityBase>>,

    /// The entities that will be affected by the potion.
    pub affected_entities: Vec<Arc<dyn EntityBase>>,

    /// The potion item stack.
    pub potion: ItemStack,
}

impl PotionSplashEvent {
    /// Creates a new [`PotionSplashEvent`].
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        entity: Arc<dyn EntityBase>,
        hit_pos: Vector3<f64>,
        hit_block: Option<BlockPos>,
        hit_entity: Option<Arc<dyn EntityBase>>,
        affected_entities: Vec<Arc<dyn EntityBase>>,
        potion: ItemStack,
    ) -> Self {
        Self {
            entity,
            hit_pos,
            hit_block,
            hit_entity,
            affected_entities,
            potion,
            cancelled: false,
        }
    }
}
