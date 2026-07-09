use std::sync::Arc;

use pumpkin_macros::{Event, cancellable};

use crate::entity::EntityBase;

/// Fired when a mob selects or changes its target.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityTargetEvent {
    /// The mob whose target changed.
    pub entity: Arc<dyn EntityBase>,

    /// The new target, if any.
    pub target: Option<Arc<dyn EntityBase>>,

    /// The reason for the target change.
    pub reason: Option<&'static str>,
}

impl EntityTargetEvent {
    /// Creates a new [`EntityTargetEvent`].
    #[must_use]
    pub fn new(
        entity: Arc<dyn EntityBase>,
        target: Option<Arc<dyn EntityBase>>,
        reason: Option<&'static str>,
    ) -> Self {
        Self {
            entity,
            target,
            reason,
            cancelled: false,
        }
    }
}
