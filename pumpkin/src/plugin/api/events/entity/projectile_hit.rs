use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

use crate::entity::EntityBase;

/// Fired when a projectile hits an entity or block.
///
/// Cancelling this event prevents the projectile's normal hit handling (such
/// as sticking into a block or damaging an entity) from running.
#[cancellable]
#[derive(Event, Clone)]
pub struct ProjectileHitEvent {
    /// The projectile that hit something.
    pub projectile: Arc<dyn EntityBase>,

    /// The entity that was hit, if any.
    pub hit_entity: Option<Arc<dyn EntityBase>>,

    /// The block that was hit, if any.
    pub hit_block: Option<&'static Block>,

    /// The position of the block that was hit, if any.
    pub hit_block_pos: Option<BlockPos>,
}

impl ProjectileHitEvent {
    /// Creates a new [`ProjectileHitEvent`].
    #[must_use]
    pub fn new(
        projectile: Arc<dyn EntityBase>,
        hit_entity: Option<Arc<dyn EntityBase>>,
        hit_block: Option<&'static Block>,
        hit_block_pos: Option<BlockPos>,
    ) -> Self {
        Self {
            projectile,
            hit_entity,
            hit_block,
            hit_block_pos,
            cancelled: false,
        }
    }
}
