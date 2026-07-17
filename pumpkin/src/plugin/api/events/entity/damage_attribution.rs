use std::sync::Arc;

use pumpkin_data::{damage::DamageType, item_stack::ItemStack};

use crate::{
    entity::{EntityBase, player::Player, projectile::is_projectile},
    plugin::api::transaction::PluginTransactionId,
    world::World,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttackKind {
    Melee,
    Projectile,
    Magic,
    Thorns,
    Explosion,
    Pet,
    Environment,
    Other,
}

#[derive(Clone)]
pub struct DamageAttribution {
    pub attack_id: PluginTransactionId,
    pub kind: AttackKind,
    pub attacker: Option<Arc<dyn EntityBase>>,
    pub attacking_player: Option<Arc<Player>>,
    pub direct_source: Option<Arc<dyn EntityBase>>,
    pub projectile: Option<Arc<dyn EntityBase>>,
    pub projectile_owner: Option<Arc<dyn EntityBase>>,
    pub weapon: Option<ItemStack>,
    pub damage_type: DamageType,
}

impl DamageAttribution {
    #[must_use]
    pub fn environment(damage_type: DamageType) -> Self {
        Self {
            attack_id: PluginTransactionId::allocate(),
            kind: AttackKind::Environment,
            attacker: None,
            attacking_player: None,
            direct_source: None,
            projectile: None,
            projectile_owner: None,
            weapon: None,
            damage_type,
        }
    }

    pub(crate) async fn capture(
        world: &Arc<World>,
        damage_type: DamageType,
        direct_source: Option<Arc<dyn EntityBase>>,
        attacker: Option<Arc<dyn EntityBase>>,
    ) -> Self {
        let projectile = direct_source
            .as_ref()
            .filter(|source| is_projectile(source.get_entity().entity_type))
            .cloned();
        let projectile_owner = projectile
            .as_ref()
            .and_then(|source| source.projectile_owner_uuid())
            .and_then(|uuid| world.get_entity_by_uuid(uuid))
            .or_else(|| projectile.as_ref().and_then(|_| attacker.clone()));

        let attacking_player = attacker
            .as_ref()
            .and_then(|entity| world.get_player_by_uuid(entity.get_entity().entity_uuid))
            .or_else(|| {
                projectile_owner.as_ref().and_then(|entity| {
                    world.get_player_by_uuid(entity.get_entity().entity_uuid)
                })
            })
            .or_else(|| {
                attacker.as_ref().and_then(|entity| {
                    entity
                        .owner_uuid()
                        .and_then(|uuid| world.get_player_by_uuid(uuid))
                })
            });

        let weapon = if let Some(projectile) = &projectile {
            projectile.attack_weapon_snapshot().await
        } else if let Some(player) = &attacking_player {
            Some(player.inventory().held_item().lock().await.clone())
        } else {
            None
        };

        let kind = if projectile.is_some() {
            AttackKind::Projectile
        } else if damage_type == DamageType::MAGIC || damage_type == DamageType::INDIRECT_MAGIC {
            AttackKind::Magic
        } else if damage_type == DamageType::THORNS {
            AttackKind::Thorns
        } else if damage_type == DamageType::EXPLOSION
            || damage_type == DamageType::PLAYER_EXPLOSION
        {
            AttackKind::Explosion
        } else if attacker.as_ref().is_some_and(|entity| entity.owner_uuid().is_some()) {
            AttackKind::Pet
        } else if attacker.is_some() || direct_source.is_some() {
            AttackKind::Melee
        } else {
            AttackKind::Environment
        };

        Self {
            attack_id: PluginTransactionId::allocate(),
            kind,
            attacker,
            attacking_player,
            direct_source,
            projectile,
            projectile_owner,
            weapon,
            damage_type,
        }
    }
}
