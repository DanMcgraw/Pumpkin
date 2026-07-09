use std::sync::Arc;

use pumpkin_macros::{Event, cancellable};

use crate::entity::EntityBase;

/// Fired when a mob targets a living entity.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityTargetLivingEntityEvent {
    /// The mob whose target changed.
    pub entity: Arc<dyn EntityBase>,

    /// The living entity being targeted.
    pub target: Arc<dyn EntityBase>,
}

impl EntityTargetLivingEntityEvent {
    /// Creates a new [`EntityTargetLivingEntityEvent`].
    #[must_use]
    pub fn new(entity: Arc<dyn EntityBase>, target: Arc<dyn EntityBase>) -> Self {
        Self {
            entity,
            target,
            cancelled: false,
        }
    }
}
