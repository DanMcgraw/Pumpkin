use std::sync::Arc;

use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::{position::BlockPos, vector3::Vector3};

use crate::entity::EntityBase;

/// Fired when an entity causes a block-breaking explosion.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityExplodeEvent {
    /// The entity that caused the explosion, if known.
    pub entity: Option<Arc<dyn EntityBase>>,

    /// The location of the explosion.
    pub location: Vector3<f64>,

    /// The blocks that will be destroyed by the explosion.
    pub affected_blocks: Vec<BlockPos>,

    /// The yield / power of the explosion.
    pub yield_: f32,
}

impl EntityExplodeEvent {
    /// Creates a new [`EntityExplodeEvent`].
    #[must_use]
    pub fn new(
        entity: Option<Arc<dyn EntityBase>>,
        location: Vector3<f64>,
        affected_blocks: Vec<BlockPos>,
        yield_: f32,
    ) -> Self {
        Self {
            entity,
            location,
            affected_blocks,
            yield_,
            cancelled: false,
        }
    }
}
