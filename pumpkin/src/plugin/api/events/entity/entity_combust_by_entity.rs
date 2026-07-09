use std::sync::Arc;

use pumpkin_macros::{Event, cancellable};

use crate::entity::EntityBase;

/// Fired when an entity is set on fire by another entity.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityCombustByEntityEvent {
    /// The entity being set on fire.
    pub entity: Arc<dyn EntityBase>,

    /// The entity causing the fire.
    pub combuster: Arc<dyn EntityBase>,

    /// The duration of the fire in seconds.
    pub duration: f32,
}

impl EntityCombustByEntityEvent {
    /// Creates a new [`EntityCombustByEntityEvent`].
    #[must_use]
    pub fn new(entity: Arc<dyn EntityBase>, combuster: Arc<dyn EntityBase>, duration: f32) -> Self {
        Self {
            entity,
            combuster,
            duration,
            cancelled: false,
        }
    }
}
