use std::sync::Arc;

use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::vector3::Vector3;

use crate::entity::EntityBase;

/// Fired just before an entity explodes.
#[cancellable]
#[derive(Event, Clone)]
pub struct ExplosionPrimeEvent {
    /// The entity that is about to explode, if known.
    pub entity: Option<Arc<dyn EntityBase>>,

    /// The location of the impending explosion.
    pub location: Vector3<f64>,

    /// The radius / power of the explosion.
    pub radius: f32,

    /// Whether the explosion will create fire.
    pub fire: bool,
}

impl ExplosionPrimeEvent {
    /// Creates a new [`ExplosionPrimeEvent`].
    #[must_use]
    pub fn new(
        entity: Option<Arc<dyn EntityBase>>,
        location: Vector3<f64>,
        radius: f32,
        fire: bool,
    ) -> Self {
        Self {
            entity,
            location,
            radius,
            fire,
            cancelled: false,
        }
    }
}
