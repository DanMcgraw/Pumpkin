use crate::entity::EntityBase;
use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

/// An event that occurs when an entity is spawned into a world.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntitySpawnEvent {
    /// The world the entity was spawned in.
    pub world: Arc<World>,
    /// The entity that was spawned.
    pub entity: Arc<dyn EntityBase>,
}

impl EntitySpawnEvent {
    /// Creates a new `EntitySpawnEvent`.
    #[must_use]
    pub fn new(world: Arc<World>, entity: Arc<dyn EntityBase>) -> Self {
        Self {
            world,
            entity,
            cancelled: false,
        }
    }
}
