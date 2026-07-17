use std::{any::Any, sync::Arc};

use pumpkin_data::{item_stack::ItemStack, screen::WindowType};
use pumpkin_world::inventory::Inventory;

use crate::{
    player::player_inventory::PlayerInventory,
    screen_handler::{
        GrindstoneOperation, InventoryPlayer, ItemStackFuture, ScreenHandler,
        ScreenHandlerBehaviour, ScreenHandlerFuture, offer_or_drop_stack,
    },
    slot::NormalSlot,
};

pub struct GrindstoneScreenHandler {
    pub inventory: Arc<dyn Inventory>,
    behaviour: ScreenHandlerBehaviour,
    experience: i32,
    transaction_id: u64,
    transaction_tick: i32,
}

impl GrindstoneScreenHandler {
    pub fn new(
        sync_id: u8,
        player_inventory: &Arc<PlayerInventory>,
        inventory: Arc<dyn Inventory>,
    ) -> Self {
        let mut handler = Self {
            inventory: inventory.clone(),
            behaviour: ScreenHandlerBehaviour::new(sync_id, Some(WindowType::Grindstone)),
            experience: 0,
            transaction_id: 0,
            transaction_tick: 0,
        };
        for slot in 0..3 {
            handler.add_slot(Arc::new(NormalSlot::new(inventory.clone(), slot)));
        }
        let player_inventory: Arc<dyn Inventory> = player_inventory.clone();
        handler.add_player_slots(&player_inventory);
        handler
    }

    async fn update_result(&mut self, player: &dyn InventoryPlayer) {
        let top = self.inventory.get_stack(0).await.lock().await.clone();
        let bottom = self.inventory.get_stack(1).await.lock().await.clone();
        let source = if !top.is_empty() { &top } else { &bottom };
        if source.is_empty() || (!top.is_empty() && !bottom.is_empty() && top.item != bottom.item) {
            self.inventory.set_stack(2, ItemStack::EMPTY.clone()).await;
            self.experience = 0;
            self.transaction_id = 0;
            return;
        }

        let mut output = source.copy_with_count(1);
        if !top.is_empty() && !bottom.is_empty() && output.is_damageable() {
            let max_damage = output.get_max_damage().unwrap_or(0);
            let remaining = max_damage - top.get_damage();
            let second_remaining = max_damage - bottom.get_damage();
            output.set_damage((max_damage - remaining - second_remaining - max_damage / 20).max(0));
        }
        let experience = top
            .enchantments()
            .into_iter()
            .chain(bottom.enchantments())
            .map(|(_, level)| level.max(0))
            .sum();
        output.clear_enchantments();
        output.set_repair_cost_level(0);

        let operation = GrindstoneOperation {
            transaction_id: 0,
            transaction_tick: 0,
            screen_sync_id: self.behaviour.sync_id,
            input_top: top,
            input_bottom: bottom,
            output,
            experience,
        };
        if let Some(operation) = player.on_grindstone_prepare(operation).await {
            self.experience = operation.experience.clamp(0, 10_000);
            self.transaction_id = operation.transaction_id;
            self.transaction_tick = operation.transaction_tick;
            self.inventory.set_stack(2, operation.output).await;
        } else {
            self.experience = 0;
            self.transaction_id = 0;
            self.inventory.set_stack(2, ItemStack::EMPTY.clone()).await;
        }
    }
}

impl ScreenHandler for GrindstoneScreenHandler {
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

    fn on_closed<'a>(&'a mut self, player: &'a dyn InventoryPlayer) -> ScreenHandlerFuture<'a, ()> {
        Box::pin(async move {
            self.default_on_closed(player).await;
            self.inventory.on_close().await;
            for slot in 0..2 {
                let stack = self.inventory.remove_stack(slot).await;
                if !stack.is_empty() {
                    offer_or_drop_stack(player, stack).await;
                }
            }
            self.inventory.set_stack(2, ItemStack::EMPTY.clone()).await;
        })
    }

    fn on_slot_click<'a>(
        &'a mut self,
        slot_index: i32,
        button: i32,
        action_type: pumpkin_protocol::java::server::play::SlotActionType,
        player: &'a dyn InventoryPlayer,
    ) -> ScreenHandlerFuture<'a, ()> {
        Box::pin(async move {
            if slot_index == 2 {
                if self.transaction_id == 0 {
                    self.send_content_updates().await;
                    return;
                }
                let output = self.inventory.get_stack(2).await.lock().await.clone();
                if !output.is_empty() {
                    let operation = GrindstoneOperation {
                        transaction_id: self.transaction_id,
                        transaction_tick: self.transaction_tick,
                        screen_sync_id: self.behaviour.sync_id,
                        input_top: self.inventory.get_stack(0).await.lock().await.clone(),
                        input_bottom: self.inventory.get_stack(1).await.lock().await.clone(),
                        output,
                        experience: self.experience,
                    };
                    if !player.on_grindstone_take(operation.clone()).await {
                        self.send_content_updates().await;
                        return;
                    }
                    self.inventory.set_stack(0, ItemStack::EMPTY.clone()).await;
                    self.inventory.set_stack(1, ItemStack::EMPTY.clone()).await;
                    player.award_experience(self.experience).await;
                    self.internal_on_slot_click(slot_index, button, action_type, player)
                        .await;
                    self.update_result(player).await;
                    self.send_content_updates().await;
                    player.on_grindstone_complete(operation).await;
                    return;
                }
            }
            self.internal_on_slot_click(slot_index, button, action_type, player)
                .await;
            if (0..=2).contains(&slot_index) {
                self.update_result(player).await;
                self.send_content_updates().await;
            }
        })
    }

    fn quick_move<'a>(
        &'a mut self,
        player: &'a dyn InventoryPlayer,
        slot_index: i32,
    ) -> ItemStackFuture<'a> {
        Box::pin(async move {
            if slot_index == 2 {
                let result = self.inventory.get_stack(2).await.lock().await.clone();
                if result.is_empty() || self.transaction_id == 0 {
                    return ItemStack::EMPTY.clone();
                }
                let operation = GrindstoneOperation {
                    transaction_id: self.transaction_id,
                    transaction_tick: self.transaction_tick,
                    screen_sync_id: self.behaviour.sync_id,
                    input_top: self.inventory.get_stack(0).await.lock().await.clone(),
                    input_bottom: self.inventory.get_stack(1).await.lock().await.clone(),
                    output: result.clone(),
                    experience: self.experience,
                };
                if !player.on_grindstone_take(operation.clone()).await {
                    return ItemStack::EMPTY.clone();
                }
                let output_slot = self.inventory.get_stack(2).await;
                let mut output = output_slot.lock().await;
                if !self
                    .insert_item(
                        &mut output,
                        3,
                        self.get_behaviour().slots.len() as i32,
                        true,
                    )
                    .await
                {
                    return ItemStack::EMPTY.clone();
                }
                drop(output);
                self.inventory.set_stack(0, ItemStack::EMPTY.clone()).await;
                self.inventory.set_stack(1, ItemStack::EMPTY.clone()).await;
                player.award_experience(self.experience).await;
                self.update_result(player).await;
                self.send_content_updates().await;
                player.on_grindstone_complete(operation).await;
                return result;
            }

            let slot = self.get_behaviour().slots[slot_index as usize].clone();
            if !slot.has_stack().await {
                return ItemStack::EMPTY.clone();
            }
            let original = slot.get_cloned_stack().await;
            let lock = slot.get_stack().await;
            let mut stack = lock.lock().await;
            let moved = if slot_index < 3 {
                self.insert_item(&mut stack, 3, self.get_behaviour().slots.len() as i32, true)
                    .await
            } else {
                self.insert_item(&mut stack, 0, 2, false).await
            };
            if !moved {
                return ItemStack::EMPTY.clone();
            }
            drop(stack);
            self.update_result(player).await;
            original
        })
    }
}
