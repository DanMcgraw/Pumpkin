use crate::world::World;
use pumpkin_macros::Event;
use std::sync::Arc;

/// Fired when a world is loaded.
#[derive(Event, Clone)]
pub struct WorldLoadEvent {
    /// The world that was loaded.
    pub world: Arc<World>,
}

impl WorldLoadEvent {
    #[must_use]
    pub const fn new(world: Arc<World>) -> Self {
        Self { world }
    }
}
