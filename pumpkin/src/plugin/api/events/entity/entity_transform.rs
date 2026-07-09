use std::sync::Arc;

use pumpkin_data::entity::EntityType;
use pumpkin_macros::{Event, cancellable};

use crate::entity::EntityBase;

/// Fired when an entity is about to transform into another entity type.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityTransformEvent {
    /// The entity that will transform.
    pub entity: Arc<dyn EntityBase>,

    /// The entity type it will become.
    pub transform_to: &'static EntityType,

    /// The reason for the transformation.
    pub reason: Option<&'static str>,
}

impl EntityTransformEvent {
    /// Creates a new [`EntityTransformEvent`].
    #[must_use]
    pub fn new(
        entity: Arc<dyn EntityBase>,
        transform_to: &'static EntityType,
        reason: Option<&'static str>,
    ) -> Self {
        Self {
            entity,
            transform_to,
            reason,
            cancelled: false,
        }
    }
}
