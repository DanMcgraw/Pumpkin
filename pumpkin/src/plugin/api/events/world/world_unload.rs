use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

/// Fired when a world is about to be unloaded.
#[cancellable]
#[derive(Event, Clone)]
pub struct WorldUnloadEvent {
    /// The world being unloaded.
    pub world: Arc<World>,
}

impl WorldUnloadEvent {
    #[must_use]
    pub const fn new(world: Arc<World>) -> Self {
        Self {
            world,
            cancelled: false,
        }
    }
}
