use crate::entity::EntityBase;
use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::vector2::Vector2;
use std::sync::Arc;

/// An event that occurs when an entity is saved and removed because its chunk is unloading.
#[cancellable]
#[derive(Event, Clone)]
pub struct ChunkEntityUnloadEvent {
    /// The world the entity is being removed from.
    pub world: Arc<World>,
    /// The entity that is being unloaded.
    pub entity: Arc<dyn EntityBase>,
    /// The chunk position the entity belonged to.
    pub chunk_pos: Vector2<i32>,
}

impl ChunkEntityUnloadEvent {
    /// Creates a new `ChunkEntityUnloadEvent`.
    #[must_use]
    pub fn new(world: Arc<World>, entity: Arc<dyn EntityBase>, chunk_pos: Vector2<i32>) -> Self {
        Self {
            world,
            entity,
            chunk_pos,
            cancelled: false,
        }
    }
}
