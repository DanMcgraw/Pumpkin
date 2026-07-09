use std::sync::Arc;

use pumpkin_macros::{Event, cancellable};

use crate::entity::{EntityBase, player::Player};

/// Fired when an entity is tamed by a player.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityTameEvent {
    /// The entity being tamed.
    pub entity: Arc<dyn EntityBase>,

    /// The player taming the entity.
    pub owner: Arc<Player>,
}

impl EntityTameEvent {
    /// Creates a new [`EntityTameEvent`].
    #[must_use]
    pub fn new(entity: Arc<dyn EntityBase>, owner: Arc<Player>) -> Self {
        Self {
            entity,
            owner,
            cancelled: false,
        }
    }
}
