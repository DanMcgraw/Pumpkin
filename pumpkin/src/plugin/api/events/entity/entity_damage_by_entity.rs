use pumpkin_data::damage::DamageType;
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

use super::damage_attribution::DamageAttribution;
use crate::entity::EntityBase;

/// Fired when a living entity is damaged by another entity.
///
/// This event fires in place of [`EntityDamageEvent`] when the direct damage
/// source is an entity. It exposes both the direct damager (e.g. an arrow or
/// zombie) and the underlying attacker (e.g. the player who shot the arrow).
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityDamageByEntityEvent {
    /// The entity being damaged.
    pub entity: Arc<dyn EntityBase>,

    /// The direct damager (e.g. a zombie, an arrow, a TNT entity).
    pub damager: Arc<dyn EntityBase>,

    /// The underlying attacker if `damager` is a projectile (e.g. the player
    /// who shot the arrow).
    pub attacker: Option<Arc<dyn EntityBase>>,

    /// The type of damage.
    pub damage_type: DamageType,

    /// Raw damage.
    pub damage: f32,

    /// Final damage to apply.
    pub final_damage: f32,

    /// Authoritative source and weapon snapshot captured for this attack.
    pub attribution: DamageAttribution,
}

impl EntityDamageByEntityEvent {
    /// Creates a new [`EntityDamageByEntityEvent`].
    #[must_use]
    pub fn new(
        entity: Arc<dyn EntityBase>,
        damager: Arc<dyn EntityBase>,
        attacker: Option<Arc<dyn EntityBase>>,
        damage_type: DamageType,
        damage: f32,
        final_damage: f32,
    ) -> Self {
        Self {
            entity,
            damager,
            attacker,
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
