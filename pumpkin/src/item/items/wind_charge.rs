use std::pin::Pin;
use std::sync::Arc;

use crate::entity::player::Player;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::sound::Sound;

use crate::entity::Entity;
use crate::entity::EntityBase;
use crate::entity::projectile::ThrownItemEntity;
use crate::entity::projectile::wind_charge::{WIND_CHARGE_GRAVITY, WindChargeEntity};
use crate::item::{ItemBehaviour, ItemMetadata};
use crate::plugin::api::events::entity::projectile_launch::ProjectileLaunchEvent;

pub struct WindChargeItem;

impl ItemMetadata for WindChargeItem {
    fn ids() -> Box<[u16]> {
        [Item::WIND_CHARGE.id].into()
    }
}

const POWER: f32 = 1.5;

impl ItemBehaviour for WindChargeItem {
    fn normal_use<'a>(
        &'a self,
        _block: &'a Item,
        player: &'a Player,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let world = player.world();
            let position = player.position();

            world.play_sound(
                Sound::EntityWindChargeThrow,
                pumpkin_data::sound::SoundCategory::Neutral,
                &position,
            );

            let entity = Entity::new(world.clone(), position, &EntityType::WIND_CHARGE);

            let wind_charge =
                ThrownItemEntity::new(entity, player.get_entity(), WIND_CHARGE_GRAVITY);
            let (yaw, pitch) = player.rotation();

            wind_charge.set_velocity_from(player.get_entity(), pitch, yaw, 0.0, POWER, 1.0);
            // TODO: player.incrementStat(Stats.USED)

            // TODO: Implement that the projectile will explode on impact
            let wind_charge_arc: Arc<dyn EntityBase> =
                Arc::new(WindChargeEntity::new_normal(wind_charge));
            let shooter = world
                .get_player_by_id(player.entity_id())
                .map(|p| p as Arc<dyn EntityBase>);
            let launch_event = ProjectileLaunchEvent::new(wind_charge_arc.clone(), shooter);
            let launch_event = world
                .server
                .upgrade()
                .expect("server is gone")
                .plugin_manager
                .fire(launch_event)
                .await;
            if launch_event.cancelled {
                return;
            }

            world.spawn_entity(wind_charge_arc).await;
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
