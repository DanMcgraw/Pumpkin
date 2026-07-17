use std::any::Any;
use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::Mutex;

use pumpkin_data::{item_stack::ItemStack, screen::WindowType};
use pumpkin_inventory::screen_handler::{
    InventoryPlayer, ItemStackFuture, ScreenHandler, ScreenHandlerBehaviour, ScreenHandlerFuture,
};
use pumpkin_inventory::slot::NormalSlot;
use pumpkin_util::text::TextComponent;
use pumpkin_world::inventory::{Clearable, Inventory, InventoryFuture};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    entity::player::Player, plugin::BoxFuture, plugin::api::transaction::PluginGuiSessionId,
};

/// Public, stable identity of the plugin that owns a GUI session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PluginIdentity(String);

impl PluginIdentity {
    #[must_use]
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Returns the plugin's declared name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0
    }
}

/// Attribution attached to inventory lifecycle events for plugin screens.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PluginGuiEventContext {
    pub session_id: PluginGuiSessionId,
    pub owner_plugin: PluginIdentity,
    pub sync_id: u8,
}

/// High-level description of a native plugin GUI.
#[derive(Clone)]
pub struct PluginGuiSpec {
    pub window_type: WindowType,
    pub title: TextComponent,
    pub slots: Vec<ItemStack>,
    pub allow_grab_items: bool,
    pub allow_put_items: bool,
}

/// Why a plugin GUI session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginGuiCloseReason {
    PlayerEscape,
    PlayerInventoryKey,
    Replaced,
    PluginRequested,
    Disconnect,
    Death,
    WorldChange,
    PluginUnload,
    ServerShutdown,
    ProtocolError,
}

/// Result of a direct plugin GUI input callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginGuiInputResult {
    Cancel,
    AllowVanilla,
}

/// Owned click snapshot delivered to the session owner.
#[derive(Clone)]
pub struct PluginGuiClickContext {
    pub session_id: PluginGuiSessionId,
    pub player: Arc<Player>,
    pub slot: i16,
    pub raw_slot: i16,
    pub is_container_slot: bool,
    pub click_type: pumpkin_inventory::screen_handler::ClickType,
    pub hotbar_button: i32,
    pub cursor: ItemStack,
    pub clicked_stack: Option<ItemStack>,
    pub revision: i32,
    pub sync_id: u8,
}

/// Owned drag snapshot delivered to the session owner.
#[derive(Clone)]
pub struct PluginGuiDragContext {
    pub session_id: PluginGuiSessionId,
    pub player: Arc<Player>,
    pub slots: Vec<i16>,
    pub cursor: ItemStack,
    pub click_type: pumpkin_inventory::screen_handler::ClickType,
    pub revision: i32,
    pub sync_id: u8,
}

/// Immutable close notification delivered exactly once.
#[derive(Clone)]
pub struct PluginGuiCloseContext {
    pub session_id: PluginGuiSessionId,
    pub player: Arc<Player>,
    pub reason: PluginGuiCloseReason,
}

/// Direct, ownership-routed callbacks for a native plugin GUI session.
pub trait PluginGuiHandler: Send + Sync {
    fn on_click<'a>(
        &'a self,
        _context: PluginGuiClickContext,
    ) -> BoxFuture<'a, PluginGuiInputResult> {
        Box::pin(async { PluginGuiInputResult::Cancel })
    }

    fn on_drag<'a>(
        &'a self,
        _context: PluginGuiDragContext,
    ) -> BoxFuture<'a, PluginGuiInputResult> {
        Box::pin(async { PluginGuiInputResult::Cancel })
    }

    fn on_close<'a>(&'a self, _context: PluginGuiCloseContext) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }
}

pub(crate) struct PassthroughPluginGuiHandler;

impl PluginGuiHandler for PassthroughPluginGuiHandler {
    fn on_click<'a>(
        &'a self,
        _context: PluginGuiClickContext,
    ) -> BoxFuture<'a, PluginGuiInputResult> {
        Box::pin(async { PluginGuiInputResult::AllowVanilla })
    }

    fn on_drag<'a>(
        &'a self,
        _context: PluginGuiDragContext,
    ) -> BoxFuture<'a, PluginGuiInputResult> {
        Box::pin(async { PluginGuiInputResult::AllowVanilla })
    }
}

#[derive(Debug, Error)]
pub enum PluginGuiError {
    #[error("GUI slot count {actual} does not match the {expected}-slot window")]
    InvalidSlotCount { expected: usize, actual: usize },
    #[error("an inventory-open handler rejected the GUI")]
    OpenCancelled,
    #[error("slot {slot} is outside the plugin container")]
    InvalidSlot { slot: usize },
}

pub struct PluginGui {
    pub window_type: WindowType,
    pub title: TextComponent,
    pub inventory: Arc<PluginInventory>,
    pub allow_grab_items: bool,
    pub allow_put_items: bool,
}

pub struct PluginInventory {
    pub slots: Vec<Arc<Mutex<ItemStack>>>,
}

impl PluginInventory {
    #[must_use]
    pub fn new(size: usize) -> Self {
        let mut slots = Vec::with_capacity(size);
        for _ in 0..size {
            slots.push(Arc::new(Mutex::new(ItemStack::EMPTY.clone())));
        }
        Self { slots }
    }

    #[must_use]
    pub fn from_stacks(stacks: Vec<ItemStack>) -> Self {
        Self {
            slots: stacks
                .into_iter()
                .map(|stack| Arc::new(Mutex::new(stack)))
                .collect(),
        }
    }
}

/// Bounded control surface for one open plugin GUI session.
#[derive(Clone)]
pub struct PluginGuiHandle {
    pub session_id: PluginGuiSessionId,
    pub player_uuid: Uuid,
    player: Weak<Player>,
    inventory: Arc<PluginInventory>,
}

impl PluginGuiHandle {
    pub async fn set_slot(&self, slot: usize, stack: ItemStack) -> Result<(), PluginGuiError> {
        let Some(target) = self.inventory.slots.get(slot) else {
            return Err(PluginGuiError::InvalidSlot { slot });
        };
        *target.lock().await = stack;
        self.refresh().await;
        Ok(())
    }

    pub async fn refresh(&self) {
        let Some(player) = self.player.upgrade() else {
            return;
        };
        let current = player.current_screen_handler.lock().await.clone();
        let mut screen = current.lock().await;
        let is_current = screen
            .as_any()
            .downcast_ref::<PluginScreenHandler>()
            .is_some_and(|handler| handler.event_context.session_id == self.session_id);
        if is_current {
            screen.send_content_updates().await;
        }
    }

    pub async fn close(&self, reason: PluginGuiCloseReason) {
        let Some(player) = self.player.upgrade() else {
            return;
        };
        if player.plugin_gui_session().await == Some(self.session_id) {
            player.close_plugin_gui(reason).await;
        }
    }

    pub async fn is_open(&self) -> bool {
        let Some(player) = self.player.upgrade() else {
            return false;
        };
        player.plugin_gui_session().await == Some(self.session_id)
    }
}

impl Clearable for PluginInventory {
    fn clear(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            for slot in &self.slots {
                *slot.lock().await = ItemStack::EMPTY.clone();
            }
        })
    }
}

impl Inventory for PluginInventory {
    fn size(&self) -> usize {
        self.slots.len()
    }

    fn is_empty(&self) -> InventoryFuture<'_, bool> {
        Box::pin(async move {
            for slot in &self.slots {
                if !slot.lock().await.is_empty() {
                    return false;
                }
            }
            true
        })
    }

    fn get_stack(&self, slot: usize) -> InventoryFuture<'_, Arc<Mutex<ItemStack>>> {
        Box::pin(async move { self.slots[slot].clone() })
    }

    fn remove_stack(&self, slot: usize) -> InventoryFuture<'_, ItemStack> {
        Box::pin(async move {
            let mut stack = self.slots[slot].lock().await;
            std::mem::replace(&mut *stack, ItemStack::EMPTY.clone())
        })
    }

    fn remove_stack_specific(&self, slot: usize, amount: u8) -> InventoryFuture<'_, ItemStack> {
        Box::pin(async move {
            let mut stack = self.slots[slot].lock().await;
            stack.split(amount)
        })
    }

    fn set_stack(&self, slot: usize, stack: ItemStack) -> InventoryFuture<'_, ()> {
        Box::pin(async move {
            *self.slots[slot].lock().await = stack;
        })
    }

    fn on_open(&self) -> InventoryFuture<'_, ()> {
        Box::pin(async move {})
    }

    fn on_close(&self) -> InventoryFuture<'_, ()> {
        Box::pin(async move {})
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct PluginScreenHandler {
    pub inventory: Arc<PluginInventory>,
    pub(crate) event_context: PluginGuiEventContext,
    pub(crate) handler: Arc<dyn PluginGuiHandler>,
    player: Weak<Player>,
    close_reason: PluginGuiCloseReason,
    close_fired: AtomicBool,
    behaviour: ScreenHandlerBehaviour,
}

impl PluginScreenHandler {
    #[must_use]
    pub(crate) fn new(
        sync_id: u8,
        window_type: WindowType,
        inventory: &Arc<PluginInventory>,
        allow_grab_items: bool,
        allow_put_items: bool,
        session_id: PluginGuiSessionId,
        owner_plugin: PluginIdentity,
        player: Weak<Player>,
        handler: Arc<dyn PluginGuiHandler>,
    ) -> Self {
        let mut behaviour = ScreenHandlerBehaviour::new(sync_id, Some(window_type));
        behaviour.allow_grab_items = allow_grab_items;
        behaviour.allow_put_items = allow_put_items;
        behaviour.container_slots = inventory.size();

        let mut handler = Self {
            inventory: inventory.clone(),
            event_context: PluginGuiEventContext {
                session_id,
                owner_plugin,
                sync_id,
            },
            handler,
            player,
            close_reason: PluginGuiCloseReason::PlayerEscape,
            close_fired: AtomicBool::new(false),
            behaviour,
        };

        for i in 0..inventory.size() {
            handler.add_slot(Arc::new(NormalSlot::new(inventory.clone(), i)));
        }

        handler
    }

    pub(crate) fn set_close_reason(&mut self, reason: PluginGuiCloseReason) {
        self.close_reason = reason;
    }
}

impl ScreenHandler for PluginScreenHandler {
    fn on_closed<'a>(&'a mut self, player: &'a dyn InventoryPlayer) -> ScreenHandlerFuture<'a, ()> {
        Box::pin(async move {
            self.default_on_closed(player).await;
            self.inventory.on_close().await;
            if !self.close_fired.swap(true, Ordering::AcqRel)
                && let Some(player) = self.player.upgrade()
            {
                self.handler
                    .on_close(PluginGuiCloseContext {
                        session_id: self.event_context.session_id,
                        player,
                        reason: self.close_reason,
                    })
                    .await;
            }
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_behaviour(&self) -> &ScreenHandlerBehaviour {
        &self.behaviour
    }

    fn get_behaviour_mut(&mut self) -> &mut ScreenHandlerBehaviour {
        &mut self.behaviour
    }

    fn quick_move<'a>(
        &'a mut self,
        _player: &'a dyn InventoryPlayer,
        _slot_index: i32,
    ) -> ItemStackFuture<'a> {
        Box::pin(async move { ItemStack::EMPTY.clone() })
    }
}

#[must_use]
pub(crate) const fn window_slot_count(window_type: WindowType) -> usize {
    match window_type {
        WindowType::Generic9x1 => 9,
        WindowType::Generic9x2 => 18,
        WindowType::Generic9x3 => 27,
        WindowType::Generic9x4 => 36,
        WindowType::Generic9x5 => 45,
        WindowType::Generic9x6 => 54,
        WindowType::Generic3x3 | WindowType::Crafter3x3 => 9,
        WindowType::Hopper => 5,
        _ => 27,
    }
}

pub(crate) async fn open_plugin_gui_owned(
    player: Arc<Player>,
    owner_plugin: PluginIdentity,
    spec: PluginGuiSpec,
    handler: Arc<dyn PluginGuiHandler>,
) -> Result<PluginGuiHandle, PluginGuiError> {
    let expected = window_slot_count(spec.window_type);
    if spec.slots.len() != expected {
        return Err(PluginGuiError::InvalidSlotCount {
            expected,
            actual: spec.slots.len(),
        });
    }

    let inventory = Arc::new(PluginInventory::from_stacks(spec.slots));
    open_plugin_gui_inventory_owned(
        player,
        owner_plugin,
        spec.window_type,
        spec.title,
        inventory,
        spec.allow_grab_items,
        spec.allow_put_items,
        handler,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn open_plugin_gui_inventory_owned(
    player: Arc<Player>,
    owner_plugin: PluginIdentity,
    window_type: WindowType,
    title: TextComponent,
    inventory: Arc<PluginInventory>,
    allow_grab_items: bool,
    allow_put_items: bool,
    handler: Arc<dyn PluginGuiHandler>,
) -> Result<PluginGuiHandle, PluginGuiError> {
    player.increment_screen_handler_sync_id();
    let sync_id = player.screen_handler_sync_id.load(Ordering::Relaxed);
    let session_id = PluginGuiSessionId::allocate();
    let screen_handler = Arc::new(Mutex::new(PluginScreenHandler::new(
        sync_id,
        window_type,
        &inventory,
        allow_grab_items,
        allow_put_items,
        session_id,
        owner_plugin,
        Arc::downgrade(&player),
        handler,
    )));

    if !player
        .open_handled_screen_direct(screen_handler, title)
        .await
    {
        return Err(PluginGuiError::OpenCancelled);
    }

    Ok(PluginGuiHandle {
        session_id,
        player_uuid: player.gameprofile.id,
        player: Arc::downgrade(&player),
        inventory,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use pumpkin_data::screen::WindowType;

    use super::{PluginGuiSessionId, PluginIdentity, window_slot_count};

    #[test]
    fn standard_plugin_windows_have_exact_container_sizes() {
        assert_eq!(window_slot_count(WindowType::Generic9x1), 9);
        assert_eq!(window_slot_count(WindowType::Generic9x3), 27);
        assert_eq!(window_slot_count(WindowType::Generic9x6), 54);
        assert_eq!(window_slot_count(WindowType::Hopper), 5);
    }

    #[test]
    fn reopening_allocates_a_new_session_identity() {
        let owner = PluginIdentity::new("test-plugin");
        assert_eq!(owner.name(), "test-plugin");

        let sessions: HashSet<_> = (0..128).map(|_| PluginGuiSessionId::allocate()).collect();
        assert_eq!(sessions.len(), 128);
    }
}
