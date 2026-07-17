use pumpkin_data::damage::DamageType;
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

use super::damage_attribution::DamageAttribution;
use crate::entity::EntityBase;

/// Fired when a living entity is damaged.
///
/// This event fires after shield blocking and resistance reductions have been
/// processed, but before hurt cooldown absorption/health modification. Plugins
/// can cancel the damage or mutate the raw and final damage values.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityDamageEvent {
    /// The entity being damaged.
    pub entity: Arc<dyn EntityBase>,

    /// The type of damage (fall, fire, entity attack, etc.).
    pub damage_type: DamageType,

    /// The raw damage amount before reductions/absorption.
    pub damage: f32,

    /// The final damage that will be applied after this event.
    pub final_damage: f32,

    /// Authoritative source and weapon snapshot captured for this attack.
    pub attribution: DamageAttribution,
}

impl EntityDamageEvent {
    /// Creates a new [`EntityDamageEvent`].
    #[must_use]
    pub fn new(
        entity: Arc<dyn EntityBase>,
        damage_type: DamageType,
        damage: f32,
        final_damage: f32,
    ) -> Self {
        Self {
            entity,
            damage_type,
            damage,
            final_damage,
            attribution: DamageAttribution::environment(damage_type),
            cancelled: false,
        }
    }

    #[must_use]
    pub fn with_attribution(mut self, attribution: DamageAttribution) -> Self {
        self.attribution = attribution;
        self
    }
}
