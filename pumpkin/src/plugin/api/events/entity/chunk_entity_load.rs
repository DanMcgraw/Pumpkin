use crate::entity::EntityBase;
use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::vector2::Vector2;
use std::sync::Arc;

/// An event that occurs when an entity is loaded from a chunk's saved NBT data.
#[cancellable]
#[derive(Event, Clone)]
pub struct ChunkEntityLoadEvent {
    /// The world the entity was loaded into.
    pub world: Arc<World>,
    /// The entity that was loaded.
    pub entity: Arc<dyn EntityBase>,
    /// The chunk position the entity was loaded from.
    pub chunk_pos: Vector2<i32>,
}

impl ChunkEntityLoadEvent {
    /// Creates a new `ChunkEntityLoadEvent`.
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
