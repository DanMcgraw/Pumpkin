use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

use crate::entity::EntityBase;

/// Fired when a projectile is launched.
///
/// This event fires before the projectile is added to the world, so cancelling
/// it prevents the projectile from spawning.
#[cancellable]
#[derive(Event, Clone)]
pub struct ProjectileLaunchEvent {
    /// The projectile being launched.
    pub projectile: Arc<dyn EntityBase>,

    /// The entity that launched the projectile, if known.
    pub shooter: Option<Arc<dyn EntityBase>>,
}

impl ProjectileLaunchEvent {
    /// Creates a new [`ProjectileLaunchEvent`].
    #[must_use]
    pub fn new(projectile: Arc<dyn EntityBase>, shooter: Option<Arc<dyn EntityBase>>) -> Self {
        Self {
            projectile,
            shooter,
            cancelled: false,
        }
    }
}
