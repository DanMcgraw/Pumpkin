use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam::atomic::AtomicCell;
use pumpkin_nbt::compound::NbtCompound;
use uuid::Uuid;

/// Persistent vanilla ownership and sitting state shared by tameable mobs.
pub struct Tameable {
    owner: AtomicCell<Option<Uuid>>,
    sitting: AtomicBool,
}

impl Tameable {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            owner: AtomicCell::new(None),
            sitting: AtomicBool::new(false),
        }
    }

    #[must_use]
    pub fn owner_uuid(&self) -> Option<Uuid> {
        self.owner.load()
    }

    pub fn set_owner(&self, owner: Option<Uuid>) {
        self.owner.store(owner);
    }

    #[must_use]
    pub fn is_tamed(&self) -> bool {
        self.owner_uuid().is_some()
    }

    #[must_use]
    pub fn is_sitting(&self) -> bool {
        self.sitting.load(Ordering::Relaxed)
    }

    pub fn set_sitting(&self, sitting: bool) {
        self.sitting.store(sitting, Ordering::Relaxed);
    }

    pub fn write_nbt(&self, nbt: &mut NbtCompound) {
        if let Some(owner) = self.owner_uuid() {
            nbt.put_string("Owner", owner.to_string());
        }
        nbt.put_bool("Sitting", self.is_sitting());
    }

    pub fn read_nbt(&self, nbt: &NbtCompound) {
        self.set_owner(
            nbt.get_string("Owner")
                .and_then(|value| Uuid::parse_str(value).ok()),
        );
        self.set_sitting(nbt.get_bool("Sitting").unwrap_or(false));
    }
}

impl Default for Tameable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_and_sitting_state_round_trip_through_nbt() {
        let owner = Uuid::new_v4();
        let tameable = Tameable::new();
        tameable.set_owner(Some(owner));
        tameable.set_sitting(true);
        let mut nbt = NbtCompound::new();
        tameable.write_nbt(&mut nbt);

        let decoded = Tameable::new();
        decoded.read_nbt(&nbt);
        assert_eq!(decoded.owner_uuid(), Some(owner));
        assert!(decoded.is_tamed());
        assert!(decoded.is_sitting());
    }
}
