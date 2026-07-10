use std::{collections::HashMap, sync::Arc};
use wasmtime::component::Resource;

use crate::plugin::loader::wasm::wasm_host::{
    DowncastResourceExt,
    state::{CommandResource, ContextResource, PluginHostState},
    wit::v0_1::{
        events::{ToFromWasmEvent, WasmPluginEventHandler},
        pumpkin::{
            self,
            plugin::{
                command::Command,
                context::Context,
                event::{EventPriority, EventType},
                permission::{Permission, PermissionDefault, PermissionLevel},
                server::Server,
            },
        },
    },
};

macro_rules! register_host_event {
    ($resource:expr, $handler:expr, $priority:expr, $blocking:expr, $event_ty:ty) => {
        $resource
            .provider
            .register_event::<$event_ty, _>(Arc::clone($handler), $priority, $blocking)
            .await
    };
}

async fn register_typed_event<E: crate::plugin::Payload + ToFromWasmEvent + 'static>(
    resource: &ContextResource,
    handler: &Arc<WasmPluginEventHandler>,
    priority: crate::plugin::EventPriority,
    blocking: bool,
) {
    register_host_event!(resource, handler, priority, blocking, E);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventRoute {
    Server,
    World,
    Block,
    Entity,
    Inventory,
    Player,
}

#[expect(clippy::too_many_lines)]
fn event_route(event_type: &EventType) -> EventRoute {
    match event_type {
        EventType::PacketReceivedEvent
        | EventType::PacketSentEvent
        | EventType::ServerCommandEvent
        | EventType::ServerBroadcastEvent
        | EventType::ServerTickEndEvent
        | EventType::ServerTickStartEvent => EventRoute::Server,
        EventType::SpawnChangeEvent => EventRoute::World,
        EventType::BlockRedstoneEvent
        | EventType::BlockBreakEvent
        | EventType::BlockDamageEvent
        | EventType::BlockDropItemEvent
        | EventType::BlockBurnEvent
        | EventType::BlockCanBuildEvent
        | EventType::BlockGrowEvent
        | EventType::BlockFormEvent
        | EventType::BlockMultiPlaceEvent
        | EventType::StructureGrowEvent
        | EventType::BlockPlaceEvent
        | EventType::BlockPistonExtendEvent
        | EventType::BlockPistonRetractEvent
        | EventType::BrewEvent
        | EventType::FurnaceBurnEvent
        | EventType::FurnaceSmeltEvent => EventRoute::Block,
        EventType::EntitySpawnEvent
        | EventType::EntityRemoveEvent
        | EventType::ChunkEntityLoadEvent
        | EventType::ChunkEntityUnloadEvent
        | EventType::EntityBlockFormEvent
        | EventType::EntityChangeBlockEvent
        | EventType::EntityDamageEvent
        | EventType::EntityDamageByEntityEvent
        | EventType::EntityDeathEvent
        | EventType::ProjectileLaunchEvent
        | EventType::ProjectileHitEvent
        | EventType::PotionSplashEvent
        | EventType::EntityShootBowEvent
        | EventType::EntityExplodeEvent
        | EventType::ExplosionPrimeEvent
        | EventType::EntityBreedEvent
        | EventType::EntityTameEvent
        | EventType::EntityTargetEvent
        | EventType::EntityTargetLivingEntityEvent
        | EventType::EntityTransformEvent
        | EventType::EntityPickupItemEvent
        | EventType::EntityCombustByEntityEvent => EventRoute::Entity,
        EventType::InventoryMoveItemEvent => EventRoute::Inventory,
        _ => EventRoute::Player,
    }
}

#[expect(clippy::too_many_lines)]
async fn register_player_event(
    resource: &ContextResource,
    handler: &Arc<WasmPluginEventHandler>,
    priority: crate::plugin::EventPriority,
    blocking: bool,
    event_type: EventType,
) {
    use crate::plugin::player::{
        changed_main_hand::PlayerChangedMainHandEvent, craft_item::CraftItemEvent,
        egg_throw::PlayerEggThrowEvent, exp_change::PlayerExpChangeEvent, fish::PlayerFishEvent,
        food_level_change::FoodLevelChangeEvent, furnace_extract::FurnaceExtractEvent,
        item_held::PlayerItemHeldEvent, player_change_world::PlayerChangeWorldEvent,
        player_chat::PlayerChatEvent, player_command_send::PlayerCommandSendEvent,
        player_custom_payload::PlayerCustomPayloadEvent, player_death::PlayerDeathEvent,
        player_drop_item::PlayerDropItemEvent, player_gamemode_change::PlayerGamemodeChangeEvent,
        player_interact_entity_event::PlayerInteractEntityEvent,
        player_interact_event::PlayerInteractEvent,
        player_interact_unknown_entity_event::PlayerInteractUnknownEntityEvent,
        player_join::PlayerJoinEvent, player_leave::PlayerLeaveEvent,
        player_login::PlayerLoginEvent, player_move::PlayerMoveEvent,
        player_permission_check::PlayerPermissionCheckEvent, player_respawn::PlayerRespawnEvent,
        player_teleport::PlayerTeleportEvent, player_toggle_flight_event::PlayerToggleFlightEvent,
        player_toggle_sneak_event::PlayerToggleSneakEvent,
        player_toggle_sprint_event::PlayerToggleSprintEvent,
    };

    match event_type {
        EventType::PlayerJoinEvent => {
            register_typed_event::<PlayerJoinEvent>(resource, handler, priority, blocking).await;
        }
        EventType::PlayerLeaveEvent => {
            register_typed_event::<PlayerLeaveEvent>(resource, handler, priority, blocking).await;
        }
        EventType::PlayerLoginEvent => {
            register_typed_event::<PlayerLoginEvent>(resource, handler, priority, blocking).await;
        }
        EventType::PlayerChatEvent => {
            register_typed_event::<PlayerChatEvent>(resource, handler, priority, blocking).await;
        }
        EventType::PlayerCommandSendEvent => {
            register_typed_event::<PlayerCommandSendEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerPermissionCheckEvent => {
            register_typed_event::<PlayerPermissionCheckEvent>(
                resource, handler, priority, blocking,
            )
            .await;
        }
        EventType::PlayerMoveEvent => {
            register_typed_event::<PlayerMoveEvent>(resource, handler, priority, blocking).await;
        }
        EventType::PlayerTeleportEvent => {
            register_typed_event::<PlayerTeleportEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerChangeWorldEvent => {
            register_typed_event::<PlayerChangeWorldEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerRespawnEvent => {
            register_typed_event::<PlayerRespawnEvent>(resource, handler, priority, blocking).await;
        }
        EventType::PlayerDeathEvent => {
            register_typed_event::<PlayerDeathEvent>(resource, handler, priority, blocking).await;
        }
        EventType::PlayerExpChangeEvent => {
            register_typed_event::<PlayerExpChangeEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerItemHeldEvent => {
            register_typed_event::<PlayerItemHeldEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerChangedMainHandEvent => {
            register_typed_event::<PlayerChangedMainHandEvent>(
                resource, handler, priority, blocking,
            )
            .await;
        }
        EventType::PlayerGamemodeChangeEvent => {
            register_typed_event::<PlayerGamemodeChangeEvent>(
                resource, handler, priority, blocking,
            )
            .await;
        }
        EventType::PlayerCustomPayloadEvent => {
            register_typed_event::<PlayerCustomPayloadEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerFishEvent => {
            register_typed_event::<PlayerFishEvent>(resource, handler, priority, blocking).await;
        }
        EventType::PlayerEggThrowEvent => {
            register_typed_event::<PlayerEggThrowEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerDropItemEvent => {
            register_typed_event::<PlayerDropItemEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerInteractUnknownEntityEvent => {
            register_typed_event::<PlayerInteractUnknownEntityEvent>(
                resource, handler, priority, blocking,
            )
            .await;
        }
        EventType::PlayerInteractEvent => {
            register_typed_event::<PlayerInteractEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerInteractEntityEvent => {
            register_typed_event::<PlayerInteractEntityEvent>(
                resource, handler, priority, blocking,
            )
            .await;
        }
        EventType::CraftItemEvent => {
            register_typed_event::<CraftItemEvent>(resource, handler, priority, blocking).await;
        }
        EventType::FoodLevelChangeEvent => {
            register_typed_event::<FoodLevelChangeEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::FurnaceExtractEvent => {
            register_typed_event::<FurnaceExtractEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerToggleSneakEvent => {
            register_typed_event::<PlayerToggleSneakEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerToggleFlightEvent => {
            register_typed_event::<PlayerToggleFlightEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PlayerToggleSprintEvent => {
            register_typed_event::<PlayerToggleSprintEvent>(resource, handler, priority, blocking)
                .await;
        }
        _ => {
            tracing::error!("non-player event should not be routed to register_player_event");
        }
    }
}

impl PluginHostState {
    fn get_context(&self, res: &Resource<Context>) -> wasmtime::Result<&ContextResource> {
        self.resource_table
            .get::<ContextResource>(&Resource::new_own(res.rep()))
            .map_err(wasmtime::Error::from)
    }

    fn take_command(&mut self, res: &Resource<Command>) -> wasmtime::Result<CommandResource> {
        self.resource_table
            .delete::<CommandResource>(Resource::new_own(res.rep()))
            // Convert ResourceTableError -> wasmtime::Error
            .map_err(wasmtime::Error::from)
    }
}

async fn register_world_event(
    resource: &ContextResource,
    handler: &Arc<WasmPluginEventHandler>,
    priority: crate::plugin::EventPriority,
    blocking: bool,
    event_type: EventType,
) {
    use crate::plugin::world::spawn_change::SpawnChangeEvent;

    match event_type {
        EventType::SpawnChangeEvent => {
            register_typed_event::<SpawnChangeEvent>(resource, handler, priority, blocking).await;
        }
        _ => {
            tracing::error!("non-world event should not be routed to register_world_event");
        }
    }
}

async fn register_block_event(
    resource: &ContextResource,
    handler: &Arc<WasmPluginEventHandler>,
    priority: crate::plugin::EventPriority,
    blocking: bool,
    event_type: EventType,
) {
    use crate::plugin::block::{
        block_break::BlockBreakEvent, block_burn::BlockBurnEvent,
        block_can_build::BlockCanBuildEvent, block_damage::BlockDamageEvent,
        block_drop_item::BlockDropItemEvent, block_form::BlockFormEvent,
        block_grow::BlockGrowEvent, block_multi_place::BlockMultiPlaceEvent,
        block_piston_extend::BlockPistonExtendEvent, block_piston_retract::BlockPistonRetractEvent,
        block_place::BlockPlaceEvent, block_redstone::BlockRedstoneEvent, brew::BrewEvent,
        furnace_burn::FurnaceBurnEvent, furnace_smelt::FurnaceSmeltEvent,
        structure_grow::StructureGrowEvent,
    };

    match event_type {
        EventType::BlockRedstoneEvent => {
            register_typed_event::<BlockRedstoneEvent>(resource, handler, priority, blocking).await;
        }
        EventType::BlockBreakEvent => {
            register_typed_event::<BlockBreakEvent>(resource, handler, priority, blocking).await;
        }
        EventType::BlockDamageEvent => {
            register_typed_event::<BlockDamageEvent>(resource, handler, priority, blocking).await;
        }
        EventType::BlockDropItemEvent => {
            register_typed_event::<BlockDropItemEvent>(resource, handler, priority, blocking).await;
        }
        EventType::BlockBurnEvent => {
            register_typed_event::<BlockBurnEvent>(resource, handler, priority, blocking).await;
        }
        EventType::BlockCanBuildEvent => {
            register_typed_event::<BlockCanBuildEvent>(resource, handler, priority, blocking).await;
        }
        EventType::BlockGrowEvent => {
            register_typed_event::<BlockGrowEvent>(resource, handler, priority, blocking).await;
        }
        EventType::BlockFormEvent => {
            register_typed_event::<BlockFormEvent>(resource, handler, priority, blocking).await;
        }
        EventType::BlockMultiPlaceEvent => {
            register_typed_event::<BlockMultiPlaceEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::StructureGrowEvent => {
            register_typed_event::<StructureGrowEvent>(resource, handler, priority, blocking).await;
        }
        EventType::BlockPlaceEvent => {
            register_typed_event::<BlockPlaceEvent>(resource, handler, priority, blocking).await;
        }
        EventType::BlockPistonExtendEvent => {
            register_typed_event::<BlockPistonExtendEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::BlockPistonRetractEvent => {
            register_typed_event::<BlockPistonRetractEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::BrewEvent => {
            register_typed_event::<BrewEvent>(resource, handler, priority, blocking).await;
        }
        EventType::FurnaceBurnEvent => {
            register_typed_event::<FurnaceBurnEvent>(resource, handler, priority, blocking).await;
        }
        EventType::FurnaceSmeltEvent => {
            register_typed_event::<FurnaceSmeltEvent>(resource, handler, priority, blocking).await;
        }
        _ => {
            tracing::error!("non-block event should not be routed to register_block_event");
        }
    }
}

async fn register_inventory_event(
    resource: &ContextResource,
    handler: &Arc<WasmPluginEventHandler>,
    priority: crate::plugin::EventPriority,
    blocking: bool,
    event_type: EventType,
) {
    use crate::plugin::inventory::inventory_move_item::InventoryMoveItemEvent;

    match event_type {
        EventType::InventoryMoveItemEvent => {
            register_typed_event::<InventoryMoveItemEvent>(resource, handler, priority, blocking)
                .await;
        }
        _ => {
            tracing::error!("non-inventory event should not be routed to register_inventory_event");
        }
    }
}
async fn register_entity_event(
    resource: &ContextResource,
    handler: &Arc<WasmPluginEventHandler>,
    priority: crate::plugin::EventPriority,
    blocking: bool,
    event_type: EventType,
) {
    use crate::plugin::entity::{
        chunk_entity_load::ChunkEntityLoadEvent, chunk_entity_unload::ChunkEntityUnloadEvent,
        entity_block_form::EntityBlockFormEvent, entity_breed::EntityBreedEvent,
        entity_change_block::EntityChangeBlockEvent,
        entity_combust_by_entity::EntityCombustByEntityEvent, entity_damage::EntityDamageEvent,
        entity_damage_by_entity::EntityDamageByEntityEvent, entity_death::EntityDeathEvent,
        entity_explode::EntityExplodeEvent, entity_pickup_item::EntityPickupItemEvent,
        entity_remove::EntityRemoveEvent, entity_shoot_bow::EntityShootBowEvent,
        entity_spawn::EntitySpawnEvent, entity_tame::EntityTameEvent,
        entity_target::EntityTargetEvent,
        entity_target_living_entity::EntityTargetLivingEntityEvent,
        entity_transform::EntityTransformEvent, explosion_prime::ExplosionPrimeEvent,
        potion_splash::PotionSplashEvent, projectile_hit::ProjectileHitEvent,
        projectile_launch::ProjectileLaunchEvent,
    };

    match event_type {
        EventType::EntitySpawnEvent => {
            register_typed_event::<EntitySpawnEvent>(resource, handler, priority, blocking).await;
        }
        EventType::EntityRemoveEvent => {
            register_typed_event::<EntityRemoveEvent>(resource, handler, priority, blocking).await;
        }
        EventType::ChunkEntityLoadEvent => {
            register_typed_event::<ChunkEntityLoadEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::ChunkEntityUnloadEvent => {
            register_typed_event::<ChunkEntityUnloadEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::EntityBlockFormEvent => {
            register_typed_event::<EntityBlockFormEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::EntityChangeBlockEvent => {
            register_typed_event::<EntityChangeBlockEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::EntityDamageEvent => {
            register_typed_event::<EntityDamageEvent>(resource, handler, priority, blocking).await;
        }
        EventType::EntityDamageByEntityEvent => {
            register_typed_event::<EntityDamageByEntityEvent>(
                resource, handler, priority, blocking,
            )
            .await;
        }
        EventType::EntityDeathEvent => {
            register_typed_event::<EntityDeathEvent>(resource, handler, priority, blocking).await;
        }
        EventType::ProjectileLaunchEvent => {
            register_typed_event::<ProjectileLaunchEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::ProjectileHitEvent => {
            register_typed_event::<ProjectileHitEvent>(resource, handler, priority, blocking).await;
        }
        EventType::PotionSplashEvent => {
            register_typed_event::<PotionSplashEvent>(resource, handler, priority, blocking).await;
        }
        EventType::EntityShootBowEvent => {
            register_typed_event::<EntityShootBowEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::EntityExplodeEvent => {
            register_typed_event::<EntityExplodeEvent>(resource, handler, priority, blocking).await;
        }
        EventType::ExplosionPrimeEvent => {
            register_typed_event::<ExplosionPrimeEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::EntityBreedEvent => {
            register_typed_event::<EntityBreedEvent>(resource, handler, priority, blocking).await;
        }
        EventType::EntityTameEvent => {
            register_typed_event::<EntityTameEvent>(resource, handler, priority, blocking).await;
        }
        EventType::EntityTargetEvent => {
            register_typed_event::<EntityTargetEvent>(resource, handler, priority, blocking).await;
        }
        EventType::EntityTargetLivingEntityEvent => {
            register_typed_event::<EntityTargetLivingEntityEvent>(
                resource, handler, priority, blocking,
            )
            .await;
        }
        EventType::EntityTransformEvent => {
            register_typed_event::<EntityTransformEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::EntityPickupItemEvent => {
            register_typed_event::<EntityPickupItemEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::EntityCombustByEntityEvent => {
            register_typed_event::<EntityCombustByEntityEvent>(
                resource, handler, priority, blocking,
            )
            .await;
        }
        _ => {
            tracing::error!("non-entity event should not be routed to register_entity_event");
        }
    }
}
async fn register_server_event(
    resource: &ContextResource,
    handler: &Arc<WasmPluginEventHandler>,
    priority: crate::plugin::EventPriority,
    blocking: bool,
    event_type: EventType,
) {
    use crate::plugin::server::{
        packet::{PacketReceivedEvent, PacketSentEvent},
        server_broadcast::ServerBroadcastEvent,
        server_command::ServerCommandEvent,
        server_tick_end::ServerTickEndEvent,
        server_tick_start::ServerTickStartEvent,
    };

    match event_type {
        EventType::PacketReceivedEvent => {
            register_typed_event::<PacketReceivedEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::PacketSentEvent => {
            register_typed_event::<PacketSentEvent>(resource, handler, priority, blocking).await;
        }
        EventType::ServerCommandEvent => {
            register_typed_event::<ServerCommandEvent>(resource, handler, priority, blocking).await;
        }
        EventType::ServerBroadcastEvent => {
            register_typed_event::<ServerBroadcastEvent>(resource, handler, priority, blocking)
                .await;
        }
        EventType::ServerTickEndEvent => {
            register_typed_event::<ServerTickEndEvent>(resource, handler, priority, blocking).await;
        }
        EventType::ServerTickStartEvent => {
            register_typed_event::<ServerTickStartEvent>(resource, handler, priority, blocking)
                .await;
        }
        _ => {
            tracing::error!("non-server event should not be routed to register_server_event");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EventRoute, event_route};
    use crate::plugin::loader::wasm::wasm_host::wit::v0_1::pumpkin::plugin::event::EventType;

    #[test]
    fn newly_exposed_block_events_route_to_block_registration() {
        for event_type in [
            EventType::BlockDamageEvent,
            EventType::BlockDropItemEvent,
            EventType::BlockPistonExtendEvent,
            EventType::BlockPistonRetractEvent,
            EventType::BrewEvent,
            EventType::FurnaceBurnEvent,
            EventType::FurnaceSmeltEvent,
        ] {
            assert_eq!(event_route(&event_type), EventRoute::Block);
        }
    }

    #[test]
    fn newly_exposed_entity_events_route_to_entity_registration() {
        for event_type in [
            EventType::EntityDamageEvent,
            EventType::EntityDamageByEntityEvent,
            EventType::EntityDeathEvent,
            EventType::ProjectileLaunchEvent,
            EventType::ProjectileHitEvent,
            EventType::PotionSplashEvent,
            EventType::EntityShootBowEvent,
            EventType::EntityExplodeEvent,
            EventType::ExplosionPrimeEvent,
            EventType::EntityBreedEvent,
            EventType::EntityTameEvent,
            EventType::EntityTargetEvent,
            EventType::EntityTargetLivingEntityEvent,
            EventType::EntityTransformEvent,
            EventType::EntityPickupItemEvent,
            EventType::EntityCombustByEntityEvent,
        ] {
            assert_eq!(event_route(&event_type), EventRoute::Entity);
        }
    }

    #[test]
    fn newly_exposed_inventory_events_route_to_inventory_registration() {
        assert_eq!(
            event_route(&EventType::InventoryMoveItemEvent),
            EventRoute::Inventory
        );
    }

    #[test]
    fn newly_exposed_player_events_route_to_player_registration() {
        for event_type in [
            EventType::PlayerDeathEvent,
            EventType::PlayerDropItemEvent,
            EventType::PlayerInteractEntityEvent,
            EventType::CraftItemEvent,
            EventType::FoodLevelChangeEvent,
            EventType::FurnaceExtractEvent,
            EventType::InventoryClickEvent,
            EventType::InventoryCloseEvent,
        ] {
            assert_eq!(event_route(&event_type), EventRoute::Player);
        }
    }
}

impl DowncastResourceExt<ContextResource> for Resource<Context> {
    fn downcast_ref<'a>(&'a self, state: &'a mut PluginHostState) -> &'a ContextResource {
        state
            .resource_table
            .get_any_mut(self.rep())
            .expect("invalid context resource handle")
            .downcast_ref()
            .expect("resource type mismatch")
    }

    fn downcast_mut<'a>(&'a self, state: &'a mut PluginHostState) -> &'a mut ContextResource {
        state
            .resource_table
            .get_any_mut(self.rep())
            .expect("invalid context resource handle")
            .downcast_mut()
            .expect("resource type mismatch")
    }

    fn consume(self, state: &mut PluginHostState) -> ContextResource {
        state
            .resource_table
            .delete(Resource::new_own(self.rep()))
            .expect("invalid context resource handle")
    }
}

impl pumpkin::plugin::context::Host for PluginHostState {}

impl pumpkin::plugin::context::HostContext for PluginHostState {
    async fn register_event(
        &mut self,
        context: Resource<Context>,
        handler_id: u32,
        event_type: EventType,
        event_priority: EventPriority,
        blocking: bool,
    ) -> wasmtime::Result<()> {
        // Updated return type
        let priority = match event_priority {
            EventPriority::Highest => crate::plugin::EventPriority::Highest,
            EventPriority::High => crate::plugin::EventPriority::High,
            EventPriority::Normal => crate::plugin::EventPriority::Normal,
            EventPriority::Low => crate::plugin::EventPriority::Low,
            EventPriority::Lowest => crate::plugin::EventPriority::Lowest,
        };

        // Use ? to trap if the plugin was dropped or the context handle is dead
        let plugin = self
            .plugin
            .as_ref()
            .ok_or_else(|| wasmtime::Error::msg("Plugin state uninitialized"))?
            .upgrade()
            .ok_or_else(|| wasmtime::Error::msg("Plugin has been dropped"))?;

        let resource = self.get_context(&context)?;
        let handler = Arc::new(WasmPluginEventHandler { handler_id, plugin });

        match event_route(&event_type) {
            EventRoute::Server => {
                register_server_event(resource, &handler, priority, blocking, event_type).await;
            }
            EventRoute::World => {
                register_world_event(resource, &handler, priority, blocking, event_type).await;
            }
            EventRoute::Block => {
                register_block_event(resource, &handler, priority, blocking, event_type).await;
            }
            EventRoute::Entity => {
                register_entity_event(resource, &handler, priority, blocking, event_type).await;
            }
            EventRoute::Inventory => {
                register_inventory_event(resource, &handler, priority, blocking, event_type).await;
            }
            EventRoute::Player => {
                register_player_event(resource, &handler, priority, blocking, event_type).await;
            }
        }

        Ok(())
    }

    async fn register_command(
        &mut self,
        context: Resource<Context>,
        command: Resource<Command>,
        permission: String,
    ) -> wasmtime::Result<()> {
        // Updated return type
        // Use your helpers to safely take/get resources
        let command_res = self.take_command(&command)?;
        let context_res = self.get_context(&context)?;

        context_res
            .provider
            .register_command(command_res.provider, permission)
            .await;
        Ok(())
    }

    async fn register_permission(
        &mut self,
        context: Resource<Context>,
        permission: Permission,
    ) -> wasmtime::Result<Result<(), String>> {
        let mut children = HashMap::with_capacity(permission.children.len());
        for child in permission.children {
            children.insert(child.node, child.value);
        }

        let util_permission = pumpkin_util::permission::Permission {
            node: permission.node,
            description: permission.description,
            default: match permission.default {
                PermissionDefault::Deny => pumpkin_util::permission::PermissionDefault::Deny,
                PermissionDefault::Allow => pumpkin_util::permission::PermissionDefault::Allow,
                PermissionDefault::Op(lvl) => {
                    pumpkin_util::permission::PermissionDefault::Op(match lvl {
                        PermissionLevel::Zero => pumpkin_util::permission::PermissionLvl::Zero,
                        PermissionLevel::One => pumpkin_util::permission::PermissionLvl::One,
                        PermissionLevel::Two => pumpkin_util::permission::PermissionLvl::Two,
                        PermissionLevel::Three => pumpkin_util::permission::PermissionLvl::Three,
                        PermissionLevel::Four => pumpkin_util::permission::PermissionLvl::Four,
                    })
                }
            },
            children,
        };

        let context_res = self
            .resource_table
            .get_mut::<ContextResource>(&Resource::new_own(context.rep()))?;
        Ok(context_res
            .provider
            .register_permission(util_permission)
            .await)
    }

    async fn get_data_folder(&mut self, context: Resource<Context>) -> wasmtime::Result<String> {
        Ok(self
            .get_context(&context)?
            .provider
            .get_data_folder()
            .to_string_lossy()
            .into_owned())
    }

    async fn get_server(
        &mut self,
        context: Resource<Context>,
    ) -> wasmtime::Result<Resource<Server>> {
        let server_provider = self.get_context(&context)?.provider.server.clone();
        self.add_server(server_provider)
            .map_err(|_| wasmtime::Error::msg("failed to add server resource"))
    }

    async fn drop(&mut self, rep: Resource<Context>) -> wasmtime::Result<()> {
        let _ = self
            .resource_table
            .delete::<ContextResource>(Resource::new_own(rep.rep()));
        Ok(())
    }
}
