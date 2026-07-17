use pumpkin_data::damage::DamageType;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_macros::Event;
use std::sync::Arc;

use super::damage_attribution::DamageAttribution;
use crate::entity::EntityBase;

/// Fired when a living entity dies.
///
/// This event is non-cancellable. Plugins can mutate the item drops and the
/// amount of experience that will be spawned.
#[derive(Event, Clone)]
pub struct EntityDeathEvent {
    /// The entity that died.
    pub entity: Arc<dyn EntityBase>,

    /// The type of damage that caused the death.
    pub damage_type: DamageType,

    /// The entity credited with the kill, if any.
    pub killer: Option<Arc<dyn EntityBase>>,

    /// The item drops that will be spawned.
    pub drops: Vec<ItemStack>,

    /// The amount of experience that will be dropped.
    pub dropped_exp: i32,

    /// Final lethal attack attribution fixed before drops are prepared.
    pub attribution: DamageAttribution,
}

impl EntityDeathEvent {
    /// Creates a new [`EntityDeathEvent`].
    #[must_use]
    pub fn new(
        entity: Arc<dyn EntityBase>,
        damage_type: DamageType,
        killer: Option<Arc<dyn EntityBase>>,
        drops: Vec<ItemStack>,
        dropped_exp: i32,
    ) -> Self {
        Self {
            entity,
            damage_type,
            killer,
            drops,
            dropped_exp,
            attribution: DamageAttribution::environment(damage_type),
        }
    }

    #[must_use]
    pub fn with_attribution(mut self, attribution: DamageAttribution) -> Self {
        self.attribution = attribution;
        self
    }
}
