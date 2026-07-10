use crate::{
    events::{
        FromIntoEvent,
        block::{
            BlockDamageEvent, BlockDropItemEvent, BlockPistonExtendEvent, BlockPistonRetractEvent,
            BrewEvent, FurnaceBurnEvent, FurnaceSmeltEvent,
        },
        entity::{
            EntityBreedEvent, EntityCombustByEntityEvent, EntityDamageByEntityEvent,
            EntityDamageEvent, EntityDeathEvent, EntityExplodeEvent, EntityPickupItemEvent,
            EntityShootBowEvent, EntityTameEvent, EntityTargetEvent, EntityTargetLivingEntityEvent,
            EntityTransformEvent, ExplosionPrimeEvent, PotionSplashEvent, ProjectileHitEvent,
            ProjectileLaunchEvent,
        },
        inventory::{InventoryClickEvent, InventoryCloseEvent, InventoryMoveItemEvent},
        player::{
            CraftItemEvent, FoodLevelChangeEvent, FurnaceExtractEvent, PlayerDeathEvent,
            PlayerDropItemEvent, PlayerInteractEntityEvent,
        },
    },
    wit::pumpkin::plugin::event::EventType,
};

macro_rules! assert_event_type {
    ($event:ty, $variant:ident) => {
        assert!(matches!(
            <$event as FromIntoEvent>::EVENT_TYPE,
            EventType::$variant
        ));
    };
}

#[test]
fn newly_exposed_plugin_event_markers_have_expected_event_types() {
    assert_event_type!(BlockDamageEvent, BlockDamageEvent);
    assert_event_type!(BlockDropItemEvent, BlockDropItemEvent);
    assert_event_type!(EntityDamageEvent, EntityDamageEvent);
    assert_event_type!(EntityDamageByEntityEvent, EntityDamageByEntityEvent);
    assert_event_type!(EntityDeathEvent, EntityDeathEvent);
    assert_event_type!(PlayerDeathEvent, PlayerDeathEvent);
    assert_event_type!(PlayerDropItemEvent, PlayerDropItemEvent);
    assert_event_type!(PlayerInteractEntityEvent, PlayerInteractEntityEvent);
    assert_event_type!(CraftItemEvent, CraftItemEvent);
    assert_event_type!(FoodLevelChangeEvent, FoodLevelChangeEvent);
    assert_event_type!(FurnaceExtractEvent, FurnaceExtractEvent);
    assert_event_type!(BlockPistonExtendEvent, BlockPistonExtendEvent);
    assert_event_type!(BlockPistonRetractEvent, BlockPistonRetractEvent);
    assert_event_type!(BrewEvent, BrewEvent);
    assert_event_type!(FurnaceBurnEvent, FurnaceBurnEvent);
    assert_event_type!(FurnaceSmeltEvent, FurnaceSmeltEvent);
    assert_event_type!(ProjectileLaunchEvent, ProjectileLaunchEvent);
    assert_event_type!(ProjectileHitEvent, ProjectileHitEvent);
    assert_event_type!(PotionSplashEvent, PotionSplashEvent);
    assert_event_type!(EntityShootBowEvent, EntityShootBowEvent);
    assert_event_type!(EntityExplodeEvent, EntityExplodeEvent);
    assert_event_type!(ExplosionPrimeEvent, ExplosionPrimeEvent);
    assert_event_type!(EntityBreedEvent, EntityBreedEvent);
    assert_event_type!(EntityTameEvent, EntityTameEvent);
    assert_event_type!(EntityTargetEvent, EntityTargetEvent);
    assert_event_type!(EntityTargetLivingEntityEvent, EntityTargetLivingEntityEvent);
    assert_event_type!(EntityTransformEvent, EntityTransformEvent);
    assert_event_type!(EntityPickupItemEvent, EntityPickupItemEvent);
    assert_event_type!(EntityCombustByEntityEvent, EntityCombustByEntityEvent);
    assert_event_type!(InventoryClickEvent, InventoryClickEvent);
    assert_event_type!(InventoryCloseEvent, InventoryCloseEvent);
    assert_event_type!(InventoryMoveItemEvent, InventoryMoveItemEvent);
}
