use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use crate::{
    LoggerOption, command::client_suggestions, net::ClientPlatform, plugin::PluginMetadata,
    plugin_log,
};
use pumpkin_nbt::tag::NbtTag;
use pumpkin_util::{
    PermissionLvl,
    permission::{Permission, PermissionManager},
};
use tokio::sync::RwLock;
use tracing::Level;

use crate::{
    entity::player::Player,
    plugin::{EventHandler, HandlerMap, PluginManager, TypedEventHandler},
    server::Server,
};

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use super::{EventPriority, Payload};

/// The `Context` struct represents the context of a plugin, containing metadata,
/// a server reference, and event handlers.
///
/// # Fields
/// - `metadata`: Metadata of the plugin.
/// - `server`: A reference to the server on which the plugin operates.
/// - `handlers`: A map of event handlers, protected by a read-write lock for safe access across threads.
pub struct Context {
    metadata: PluginMetadata,
    pub server: Arc<Server>,
    pub handlers: Arc<RwLock<HandlerMap>>,
    pub plugin_manager: Arc<PluginManager>,
    pub permission_manager: Arc<RwLock<PermissionManager>>,
    pub logger: Arc<OnceLock<LoggerOption>>,
}
impl Context {
    fn persistent_data_namespace(&self) -> String {
        self.metadata
            .name
            .bytes()
            .map(|byte| {
                let byte = byte.to_ascii_lowercase();
                if byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte) {
                    char::from(byte)
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// Creates a new instance of `Context`.
    ///
    /// # Arguments
    /// - `metadata`: The metadata of the plugin.
    /// - `server`: A reference to the server.
    /// - `handlers`: A collection containing the event handlers.
    ///
    /// # Returns
    /// A new instance of `Context`.
    #[must_use]
    pub fn new(
        metadata: PluginMetadata,
        server: Arc<Server>,
        handlers: Arc<RwLock<HandlerMap>>,
        plugin_manager: Arc<PluginManager>,
        logger: Arc<OnceLock<LoggerOption>>,
    ) -> Self {
        let permission_manager = server.permission_manager.clone();
        Self {
            metadata,
            server,
            handlers,
            plugin_manager,
            permission_manager,
            logger,
        }
    }

    #[must_use]
    pub const fn get_metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    /// Retrieves the data folder path for the plugin, creating it if it does not exist.
    ///
    /// # Returns
    /// A string representing the path to the data folder.
    #[must_use]
    pub fn get_data_folder(&self) -> PathBuf {
        let path = Path::new("plugins").join(&self.metadata.name);
        if !path.exists() {
            fs::create_dir_all(&path).unwrap();
        }
        path
    }

    /// Asynchronously retrieves a player by their name.
    ///
    /// # Arguments
    /// - `player_name`: The name of the player to retrieve.
    ///
    /// # Returns
    /// An optional reference to the player if found, or `None` if not.
    #[must_use]
    pub fn get_player_by_name(&self, player_name: &str) -> Option<Arc<Player>> {
        self.server.get_player_by_name(player_name)
    }

    /// Reads a value in this plugin's namespace from online or offline player data.
    pub async fn get_player_data(
        &self,
        player_uuid: uuid::Uuid,
        key: &str,
    ) -> Result<Option<NbtTag>, super::persistent_data::PluginDataError> {
        if !super::persistent_data::valid_component(key) {
            return Err(super::persistent_data::PluginDataError::InvalidKey);
        }

        let namespace = self.persistent_data_namespace();
        if let Some(player) = self.server.get_player_by_uuid(player_uuid) {
            return Ok(player.get_plugin_data(&namespace, key));
        }

        let Some(data) = self
            .server
            .player_data_storage
            .load_data(&player_uuid)
            .await
            .map_err(|error| super::persistent_data::PluginDataError::Storage(error.to_string()))?
        else {
            return Ok(None);
        };
        Ok(data
            .get_compound("PumpkinPluginData")
            .and_then(|root| root.get(&namespace))
            .and_then(NbtTag::extract_compound)
            .and_then(|namespace_data| namespace_data.get(key))
            .cloned())
    }

    /// Stores a value in this plugin's namespace and persists it immediately.
    pub async fn set_player_data(
        &self,
        player_uuid: uuid::Uuid,
        key: &str,
        value: NbtTag,
    ) -> Result<(), super::persistent_data::PluginDataError> {
        let namespace = self.persistent_data_namespace();
        if let Some(player) = self.server.get_player_by_uuid(player_uuid) {
            player.set_plugin_data(&namespace, key, value)?;
            return self
                .server
                .player_data_storage
                .save_player(&player)
                .await
                .map_err(|error| {
                    super::persistent_data::PluginDataError::Storage(error.to_string())
                });
        }

        let mut data = self
            .server
            .player_data_storage
            .load_data(&player_uuid)
            .await
            .map_err(|error| super::persistent_data::PluginDataError::Storage(error.to_string()))?
            .unwrap_or_default();
        let mut plugin_root = data
            .get_compound("PumpkinPluginData")
            .cloned()
            .unwrap_or_default();
        let namespace_data =
            super::persistent_data::namespace_with_value(&plugin_root, &namespace, key, value)?;
        plugin_root
            .child_tags
            .insert(namespace.into(), NbtTag::Compound(namespace_data));
        data.child_tags
            .insert("PumpkinPluginData".into(), NbtTag::Compound(plugin_root));
        self.server
            .player_data_storage
            .save_data(player_uuid, data)
            .await
            .map_err(|error| super::persistent_data::PluginDataError::Storage(error.to_string()))
    }

    /// Removes a value in this plugin's namespace and persists the update.
    pub async fn remove_player_data(
        &self,
        player_uuid: uuid::Uuid,
        key: &str,
    ) -> Result<(), super::persistent_data::PluginDataError> {
        if !super::persistent_data::valid_component(key) {
            return Err(super::persistent_data::PluginDataError::InvalidKey);
        }

        let namespace = self.persistent_data_namespace();
        if let Some(player) = self.server.get_player_by_uuid(player_uuid) {
            player.remove_plugin_data(&namespace, key);
            return self
                .server
                .player_data_storage
                .save_player(&player)
                .await
                .map_err(|error| {
                    super::persistent_data::PluginDataError::Storage(error.to_string())
                });
        }

        let Some(mut data) = self
            .server
            .player_data_storage
            .load_data(&player_uuid)
            .await
            .map_err(|error| super::persistent_data::PluginDataError::Storage(error.to_string()))?
        else {
            return Ok(());
        };
        let Some(mut plugin_root) = data.get_compound("PumpkinPluginData").cloned() else {
            return Ok(());
        };
        let Some(NbtTag::Compound(mut namespace_data)) =
            plugin_root.child_tags.remove(namespace.as_str())
        else {
            return Ok(());
        };
        namespace_data.child_tags.remove(key);
        if !namespace_data.is_empty() {
            plugin_root
                .child_tags
                .insert(namespace.into(), NbtTag::Compound(namespace_data));
        }
        data.child_tags
            .insert("PumpkinPluginData".into(), NbtTag::Compound(plugin_root));
        self.server
            .player_data_storage
            .save_data(player_uuid, data)
            .await
            .map_err(|error| super::persistent_data::PluginDataError::Storage(error.to_string()))
    }

    /// Reads a value owned by this plugin from an entity's persistent data.
    pub fn get_entity_data(
        &self,
        entity: &dyn crate::entity::EntityBase,
        key: &str,
    ) -> Result<Option<NbtTag>, super::persistent_data::PluginDataError> {
        if !super::persistent_data::valid_component(key) {
            return Err(super::persistent_data::PluginDataError::InvalidKey);
        }
        Ok(entity
            .get_entity()
            .get_plugin_data(&self.persistent_data_namespace(), key))
    }

    /// Stores a bounded value owned by this plugin on an entity.
    pub fn set_entity_data(
        &self,
        entity: &dyn crate::entity::EntityBase,
        key: &str,
        value: NbtTag,
    ) -> Result<(), super::persistent_data::PluginDataError> {
        entity
            .get_entity()
            .set_plugin_data(&self.persistent_data_namespace(), key, value)
    }

    /// Removes a value owned by this plugin from an entity.
    pub fn remove_entity_data(
        &self,
        entity: &dyn crate::entity::EntityBase,
        key: &str,
    ) -> Result<(), super::persistent_data::PluginDataError> {
        if !super::persistent_data::valid_component(key) {
            return Err(super::persistent_data::PluginDataError::InvalidKey);
        }
        entity
            .get_entity()
            .remove_plugin_data(&self.persistent_data_namespace(), key);
        Ok(())
    }

    /// Reads block metadata in this plugin's namespace.
    pub fn get_block_metadata(
        &self,
        world: &crate::world::World,
        position: &pumpkin_util::math::position::BlockPos,
        key: &str,
    ) -> Result<Option<pumpkin_nbt::compound::NbtCompound>, super::persistent_data::PluginDataError>
    {
        if !super::persistent_data::valid_component(key) {
            return Err(super::persistent_data::PluginDataError::InvalidKey);
        }
        Ok(world.get_block_metadata(
            position,
            &format!("{}:{key}", self.persistent_data_namespace()),
        ))
    }

    /// Stores or removes block metadata in this plugin's namespace.
    pub fn set_block_metadata(
        &self,
        world: &crate::world::World,
        position: &pumpkin_util::math::position::BlockPos,
        key: &str,
        value: Option<pumpkin_nbt::compound::NbtCompound>,
    ) -> Result<(), super::persistent_data::PluginDataError> {
        if !super::persistent_data::valid_component(key) {
            return Err(super::persistent_data::PluginDataError::InvalidKey);
        }
        world.set_block_metadata(
            position,
            &format!("{}:{key}", self.persistent_data_namespace()),
            value,
        )
    }

    /// Breaks a bounded set of blocks through Pumpkin's normal protection, drop, and durability paths.
    pub async fn break_blocks(
        &self,
        world: Arc<crate::world::World>,
        player: Arc<Player>,
        positions: impl IntoIterator<Item = pumpkin_util::math::position::BlockPos>,
    ) -> Result<Vec<pumpkin_util::math::position::BlockPos>, super::persistent_data::BatchBlockError>
    {
        let mut unique = std::collections::HashSet::new();
        for position in positions {
            unique.insert(position);
            if unique.len() > 128 {
                return Err(super::persistent_data::BatchBlockError::TooLarge);
            }
        }

        let mut broken = Vec::with_capacity(unique.len());
        for position in unique {
            let state = world.get_block_state(&position);
            if world
                .break_block(
                    &position,
                    Some(player.clone()),
                    pumpkin_world::world::BlockFlags::NOTIFY_ALL,
                )
                .await
                .is_some()
            {
                player.apply_tool_damage_for_block_break(state).await;
                broken.push(position);
            }
        }
        Ok(broken)
    }

    /// Registers a service with the plugin context.
    ///
    /// This method allows you to associate a service instance with a given name,
    /// making it available for retrieval by plugins or other components.
    /// The service must be wrapped in an `Arc` and implement `Payload`.
    ///
    /// # Arguments
    ///
    /// * `name` - The unique name to register the service under.
    /// * `service` - The service instance to register, wrapped in an `Arc`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// context.register_service("my_service", Arc::new(MyService::new())).await;
    /// ```
    pub async fn register_service<N: Into<String>, T: Payload + 'static>(
        &self,
        name: N,
        service: Arc<T>,
    ) {
        let mut services = self.plugin_manager.services.write().await;
        services.insert(name.into(), service);
    }

    /// Retrieves a registered service by name and type.
    ///
    /// This method attempts to fetch a service previously registered under the given name,
    /// and downcasts it to the requested type using name-based type checking.
    /// Returns `Some(Arc<T>)` if the service exists and the type matches, or `None` otherwise.
    ///
    /// This method is safe to use across compilation boundaries as it uses string-based
    /// type identification instead of `TypeId`.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the service to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option<Arc<T>>` containing the service if found and type matches, or `None`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(service) = context.get_service::<MyService>("my_service").await {
    ///     // Use the service
    /// }
    /// ```
    pub async fn get_service<T: Payload + 'static>(&self, name: &str) -> Option<Arc<T>> {
        let services = self.plugin_manager.services.read().await;
        let service = services.get(name)?.clone();
        <dyn Payload>::downcast_arc::<T>(service)
    }

    /// Asynchronously registers a command with the server.
    ///
    /// # Arguments
    /// - `tree`: The command tree to register.
    /// - `permission`: The permission level required to execute the command.
    pub async fn register_command<P: Into<String>>(
        &self,
        tree: crate::command::tree::CommandTree,
        permission: P,
    ) {
        let permission = permission.into();

        let mut tree = tree.clone();
        tree.source = Some(self.metadata.name.clone());

        let full_permission_node = if permission.contains(':') {
            permission
        } else {
            format!("{}:{permission}", self.metadata.name)
        };

        {
            let mut dispatcher_lock = self.server.command_dispatcher.write().await;
            dispatcher_lock
                .fallback_dispatcher
                .register(tree, full_permission_node);
        };

        self.reload_commands_for_everyone().await;
    }

    /// Asynchronously unregisters a command from the server.
    ///
    /// # Arguments
    /// - `name`: The name of the command to unregister.
    pub async fn unregister_command(&self, name: &str) {
        {
            let mut dispatcher_lock = self.server.command_dispatcher.write().await;
            dispatcher_lock.fallback_dispatcher.unregister(name);
        };

        self.reload_commands_for_everyone().await;
    }

    /// Asynchronously reloads (resends) all commands for all currently online players.
    pub async fn reload_commands_for_everyone(&self) {
        for world in self.server.worlds.load().iter() {
            for player in world.players.load().iter() {
                self.reload_commands_for(player).await;
            }
        }
    }

    /// Asynchronously reloads (resends) all commands for a particular player on the server.
    ///
    /// # Arguments
    /// - `player`: The player for which the commands will be reloaded.
    pub async fn reload_commands_for(&self, player: &Arc<Player>) {
        let command_dispatcher = self.server.command_dispatcher.read().await;
        if let ClientPlatform::Bedrock(_) = player.client.as_ref() {
            client_suggestions::send_bedrock_commands_packet(
                player,
                &self.server,
                &command_dispatcher,
            )
            .await;
        } else {
            client_suggestions::send_c_commands_packet(player, &self.server, &command_dispatcher)
                .await;
        }
    }

    /// Register a permission for this plugin
    pub async fn register_permission(&self, permission: Permission) -> Result<(), String> {
        // Ensure the permission has the correct namespace
        if !permission
            .node
            .starts_with(&format!("{}:", self.metadata.name))
        {
            return Err(format!(
                "Permission {} must use the plugin's namespace ({})",
                permission.node, self.metadata.name
            ));
        }

        let registry = &self.permission_manager.read().await.registry;
        registry.write().await.register_permission(permission)
    }

    /// Check if a player has a permission
    pub async fn player_has_permission(&self, player_uuid: &uuid::Uuid, permission: &str) -> bool {
        let permission_manager = self.permission_manager.read().await;

        // If the player isn't online, we need to find their op level
        let player_op_level = self
            .server
            .get_player_by_uuid(*player_uuid)
            .map_or(PermissionLvl::Zero, |player| player.permission_lvl.load());

        permission_manager
            .has_permission(player_uuid, permission, player_op_level)
            .await
    }

    /// Asynchronously registers an event handler for a specific event type.
    ///
    /// # Type Parameters
    /// - `E`: The event type that the handler will respond to.
    /// - `H`: The type of the event handler.
    ///
    /// # Arguments
    /// - `handler`: A reference to the event handler.
    /// - `priority`: The priority of the event handler.
    /// - `blocking`: A boolean indicating whether the handler is blocking.
    ///
    /// # Constraints
    /// The handler must implement the `EventHandler<E>` trait.
    pub async fn register_event<E: Payload + 'static, H>(
        &self,
        handler: Arc<H>,
        priority: EventPriority,
        blocking: bool,
    ) where
        H: EventHandler<E> + 'static,
    {
        let mut handlers = self.handlers.write().await;

        let handlers_vec = handlers
            .entry(E::get_name_static())
            .or_insert_with(Vec::new);

        let typed_handler = TypedEventHandler {
            handler,
            priority,
            blocking,
            _phantom: std::marker::PhantomData,
        };
        handlers_vec.push(Box::new(typed_handler));
    }

    /// Registers a custom plugin loader that can load additional plugin types.
    ///
    /// This method allows plugins to extend the server with support for loading
    /// plugins in different formats (e.g., Lua, JavaScript, Python). When a new
    /// loader is registered, the plugin manager will automatically attempt to load
    /// any previously unloadable files in the plugins directory with this new loader.
    ///
    /// # Arguments
    /// - `loader`: The custom plugin loader implementation to register.
    ///
    /// # Returns
    /// `true` if new plugins were loaded as a result of registering this loader, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Create and register a custom Lua plugin loader
    /// let lua_loader = Arc::new(LuaPluginLoader::new());
    /// context.register_plugin_loader(lua_loader).await;
    /// ```
    pub async fn register_plugin_loader(
        &self,
        loader: Arc<dyn crate::plugin::loader::PluginLoader>,
    ) -> bool {
        let before_count = self.plugin_manager.loaded_plugins().await.len();
        self.plugin_manager.add_loader(loader).await;
        let after_count = self.plugin_manager.loaded_plugins().await.len();

        // Return true if any new plugins were loaded
        after_count > before_count
    }

    /// Initializes logging via the tracing crate for the plugin.
    pub fn init_log(&self) {
        if let Some(Some((_logger_impl, level, config))) = self.logger.get() {
            let fmt_layer = fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(config.color)
                .with_target(true)
                .with_thread_names(config.threads)
                .with_thread_ids(config.threads);

            if config.timestamp {
                let fmt_layer = fmt_layer.with_timer(fmt::time::UtcTime::new(
                    time::macros::format_description!(
                        "[year]-[month]-[day] [hour]:[minute]:[second]"
                    ),
                ));
                tracing_subscriber::registry()
                    .with(*level)
                    .with(fmt_layer)
                    .init();
            } else {
                let fmt_layer = fmt_layer.without_time();
                tracing_subscriber::registry()
                    .with(*level)
                    .with(fmt_layer)
                    .init();
            }
        }
    }

    pub fn log(&self, message: impl std::fmt::Display) {
        let level = if let Some(Some((_, level, _))) = self.logger.get() {
            level.into_level().unwrap_or(Level::INFO)
        } else {
            Level::INFO
        };
        plugin_log!(level, &self.metadata.name, "{}", message);
    }
}
