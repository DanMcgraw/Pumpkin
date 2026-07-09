use std::sync::Arc;

use pumpkin_data::entity::EntityType;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::vector3::Vector3;

use crate::entity::{EntityBase, player::Player};

/// Fired when two animals breed and produce a baby.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityBreedEvent {
    /// The first parent (mother).
    pub mother: Arc<dyn EntityBase>,

    /// The second parent (father).
    pub father: Arc<dyn EntityBase>,

    /// The player that caused the breeding, if any.
    pub breeder: Option<Arc<Player>>,

    /// The type of entity the baby will be.
    pub baby_type: &'static EntityType,

    /// The position where the baby will spawn.
    pub position: Vector3<f64>,

    /// The amount of experience to drop.
    pub experience: i32,
}

impl EntityBreedEvent {
    /// Creates a new [`EntityBreedEvent`].
    #[must_use]
    pub fn new(
        mother: Arc<dyn EntityBase>,
        father: Arc<dyn EntityBase>,
        breeder: Option<Arc<Player>>,
        baby_type: &'static EntityType,
        position: Vector3<f64>,
        experience: i32,
    ) -> Self {
        Self {
            mother,
            father,
            breeder,
            baby_type,
            position,
            experience,
            cancelled: false,
        }
    }
}
