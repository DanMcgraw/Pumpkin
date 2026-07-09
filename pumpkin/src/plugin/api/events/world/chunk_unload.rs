use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::vector2::Vector2;
use pumpkin_world::chunk::ChunkData;
use std::sync::Arc;

/// Fired when a chunk is unloaded from a world.
#[cancellable]
#[derive(Event, Clone)]
pub struct ChunkUnloadEvent {
    /// The world from which the chunk is being unloaded.
    pub world: Arc<World>,
    /// The chunk data being unloaded.
    pub chunk: Arc<ChunkData>,
    /// The chunk position.
    pub pos: Vector2<i32>,
}

impl ChunkUnloadEvent {
    #[must_use]
    pub const fn new(world: Arc<World>, chunk: Arc<ChunkData>, pos: Vector2<i32>) -> Self {
        Self {
            world,
            chunk,
            pos,
            cancelled: false,
        }
    }
}
