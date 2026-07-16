use std::any::Any;
use std::sync::Arc;

use pumpkin_data::{item::Item, item_stack::ItemStack, screen::WindowType};
use pumpkin_world::inventory::Inventory;

use crate::{
    player::player_inventory::PlayerInventory,
    screen_handler::{
        AnvilOperation, InventoryPlayer, ItemStackFuture, ScreenHandler, ScreenHandlerBehaviour,
        ScreenHandlerFuture, offer_or_drop_stack,
    },
    slot::NormalSlot,
    window_property::{Anvil, WindowProperty},
};

pub struct AnvilScreenHandler {
    pub inventory: Arc<dyn Inventory>,
    behaviour: ScreenHandlerBehaviour,
    pub rename_text: String,
    pub repair_cost: i16,
    material_cost: u8,
}

impl AnvilScreenHandler {
    fn is_repair_material(item: &ItemStack, material: &ItemStack) -> bool {
        let key = item.item.registry_key;
        let material = material.item;
        (key.contains("leather_") && material == &Item::LEATHER)
            || (key.contains("diamond_") && material == &Item::DIAMOND)
            || (key.contains("netherite_") && material == &Item::NETHERITE_INGOT)
            || ((key.contains("iron_") || key.contains("chainmail_"))
                && material == &Item::IRON_INGOT)
            || (key.contains("golden_") && material == &Item::GOLD_INGOT)
            || (key.contains("stone_") && material == &Item::COBBLESTONE)
            || (key.contains("wooden_") && material.registry_key.ends_with("_planks"))
            || (key.ends_with("elytra") && material == &Item::PHANTOM_MEMBRANE)
    }

    #[expect(clippy::needless_pass_by_value)]
    pub fn new(
        sync_id: u8,
        player_inventory: &Arc<PlayerInventory>,
        inventory: Arc<dyn Inventory>,
    ) -> Self {
        let mut handler = Self {
            inventory: inventory.clone(),
            behaviour: ScreenHandlerBehaviour::new(sync_id, Some(WindowType::Anvil)),
            rename_text: String::new(),
            repair_cost: 0,
            material_cost: 0,
        };

        // Anvil specific slots: 2 input, 1 output
        for i in 0..3 {
            handler.add_slot(Arc::new(NormalSlot::new(inventory.clone(), i)));
        }

        let player_inventory: Arc<dyn Inventory> = player_inventory.clone();
        handler.add_player_slots(&player_inventory);

        handler
    }

    pub async fn update_item_name(&mut self, name: String, player: &dyn InventoryPlayer) {
        self.rename_text = name;
        self.update_result_slot(player).await;
        self.send_content_updates().await;
    }

    pub async fn update_result_slot(&mut self, player: &dyn InventoryPlayer) {
        let input_a = {
            let lock = self.inventory.get_stack(0).await;
            lock.lock().await.clone()
        };

        if input_a.is_empty() {
            self.inventory.set_stack(2, ItemStack::EMPTY.clone()).await;
            self.set_repair_cost(0).await;
            self.material_cost = 0;
            return;
        }

        let input_b = {
            let lock = self.inventory.get_stack(1).await;
            lock.lock().await.clone()
        };
        let mut result_item = input_a.clone();
        let mut operation_cost = 0_i32;
        let mut material_cost = 0_u8;
        let mut changed = false;

        if !input_b.is_empty() {
            if input_a.is_damageable() && Self::is_repair_material(&input_a, &input_b) {
                let max_damage = input_a.get_max_damage().unwrap_or(0);
                let repair_per_item = (max_damage / 4).max(1);
                let mut damage = input_a.get_damage();
                while damage > 0 && material_cost < input_b.item_count {
                    damage = (damage - repair_per_item).max(0);
                    material_cost += 1;
                    operation_cost += 1;
                }
                result_item.set_damage(damage);
                changed = material_cost > 0;
            } else if input_a.item == input_b.item && input_a.is_damageable() {
                let max_damage = input_a.get_max_damage().unwrap_or(0);
                let remaining = max_damage - input_a.get_damage();
                let second_remaining = max_damage - input_b.get_damage();
                let bonus = max_damage * 12 / 100;
                result_item.set_damage((max_damage - remaining - second_remaining - bonus).max(0));
                operation_cost += 2;
                material_cost = 1;
                changed = true;
            }

            for (enchantment, second_level) in input_b.enchantments() {
                if !enchantment.can_enchant(result_item.item) {
                    continue;
                }
                if result_item
                    .enchantments()
                    .iter()
                    .any(|(existing, _)| !existing.are_compatible(enchantment))
                {
                    operation_cost += 1;
                    continue;
                }
                let first_level = result_item.get_enchantment_level(enchantment);
                let merged_level = if first_level == second_level {
                    first_level + 1
                } else {
                    first_level.max(second_level)
                }
                .min(enchantment.max_level);
                if merged_level != first_level {
                    result_item.set_enchantment(enchantment, merged_level);
                    operation_cost += enchantment.anvil_cost as i32 * merged_level;
                    material_cost = 1;
                    changed = true;
                }
            }
        }

        // Basic renaming logic for now
        if !self.rename_text.is_empty() {
            result_item.set_custom_name(self.rename_text.clone());
            operation_cost += 1;
            changed = true;
        }

        if changed {
            let prior_work = |level: i32| 2_i32.saturating_pow(level.min(15) as u32) - 1;
            let cost = (operation_cost
                + prior_work(input_a.repair_cost_level())
                + prior_work(input_b.repair_cost_level()))
            .clamp(1, i16::MAX as i32) as i16;
            result_item.set_repair_cost_level(
                input_a.repair_cost_level().max(input_b.repair_cost_level()) + 1,
            );
            let operation = AnvilOperation {
                input_first: input_a,
                input_second: input_b,
                output: result_item,
                level_cost: cost,
                material_cost,
            };
            if let Some(operation) = player.on_anvil_prepare(operation).await {
                self.material_cost = operation.material_cost;
                self.inventory.set_stack(2, operation.output).await;
                self.set_repair_cost(operation.level_cost.max(0)).await;
            } else {
                self.material_cost = 0;
                self.inventory.set_stack(2, ItemStack::EMPTY.clone()).await;
                self.set_repair_cost(0).await;
            }
        } else {
            self.material_cost = 0;
            self.inventory.set_stack(2, ItemStack::EMPTY.clone()).await;
            self.set_repair_cost(0).await;
        }
    }

    pub async fn set_repair_cost(&mut self, cost: i16) {
        self.repair_cost = cost;
        if let Some(sync_handler) = self.behaviour.sync_handler.as_ref() {
            let (property_id, property_value) =
                WindowProperty::new(Anvil::RepairCost, cost).into_tuple();
            sync_handler
                .update_property(&self.behaviour, property_id as i32, property_value as i32)
                .await;
        }
    }
}

impl ScreenHandler for AnvilScreenHandler {
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
            // Drop inputs from anvil
            for i in 0..2 {
                let stack = self.inventory.remove_stack(i).await;
                if !stack.is_empty() {
                    offer_or_drop_stack(player, stack).await;
                }
            }
            self.inventory.set_stack(2, ItemStack::EMPTY.clone()).await;
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
                if result.is_empty()
                    || (!player.is_creative()
                        && player.experience_level() < self.repair_cost as i32)
                {
                    return ItemStack::EMPTY.clone();
                }
                let operation = AnvilOperation {
                    input_first: self.inventory.get_stack(0).await.lock().await.clone(),
                    input_second: self.inventory.get_stack(1).await.lock().await.clone(),
                    output: result.clone(),
                    level_cost: self.repair_cost,
                    material_cost: self.material_cost,
                };
                if !player.on_anvil_take(operation).await {
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
                if !player.is_creative() {
                    player
                        .add_experience_levels(-(self.repair_cost as i32))
                        .await;
                }
                let input = self.inventory.get_stack(0).await;
                input.lock().await.decrement(1);
                let material = self.inventory.get_stack(1).await;
                material.lock().await.decrement(self.material_cost);
                self.update_result_slot(player).await;
                self.send_content_updates().await;
                return result;
            }

            let mut stack_left = ItemStack::EMPTY.clone();
            let slot = self.get_behaviour().slots[slot_index as usize].clone();

            if slot.has_stack().await {
                let slot_stack_lock = slot.get_stack().await;
                let slot_stack_guard = slot_stack_lock.lock().await;
                stack_left = slot_stack_guard.clone();
                drop(slot_stack_guard);

                let mut slot_stack_mut = slot_stack_lock.lock().await;

                if slot_index < 3 {
                    // From anvil to player
                    if !self
                        .insert_item(
                            &mut slot_stack_mut,
                            3,
                            self.get_behaviour().slots.len() as i32,
                            true,
                        )
                        .await
                    {
                        return ItemStack::EMPTY.clone();
                    }
                } else {
                    // From player to anvil input 0 and 1
                    if !self.insert_item(&mut slot_stack_mut, 0, 2, false).await {
                        return ItemStack::EMPTY.clone();
                    }
                }

                if slot_stack_mut.is_empty() {
                    drop(slot_stack_mut);
                    slot.set_stack(ItemStack::EMPTY.clone()).await;
                } else {
                    drop(slot_stack_mut);
                    slot.mark_dirty().await;
                }
            }

            stack_left
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
                // Taking from output slot
                let result_slot = self.get_behaviour().slots[2].clone();
                if result_slot.has_stack().await {
                    let result_stack = result_slot.get_cloned_stack().await;
                    if !result_stack.is_empty() {
                        if player.experience_level() >= self.repair_cost as i32
                            || player.is_creative()
                        {
                            let input_first =
                                self.inventory.get_stack(0).await.lock().await.clone();
                            let input_second =
                                self.inventory.get_stack(1).await.lock().await.clone();
                            let operation = AnvilOperation {
                                input_first,
                                input_second,
                                output: result_stack,
                                level_cost: self.repair_cost,
                                material_cost: self.material_cost,
                            };
                            if !player.on_anvil_take(operation).await {
                                self.send_content_updates().await;
                                return;
                            }
                            // Consume experience
                            if !player.is_creative() {
                                player
                                    .add_experience_levels(-(self.repair_cost as i32))
                                    .await;
                            }

                            // Consume inputs
                            let input_a = self.inventory.get_stack(0).await;
                            let mut input_a = input_a.lock().await;
                            input_a.decrement(1); // Usually consumes 1
                            if input_a.is_empty() {
                                *input_a = ItemStack::EMPTY.clone();
                            }
                            drop(input_a);
                            self.get_behaviour().slots[0].mark_dirty().await;
                            let input_b = self.inventory.get_stack(1).await;
                            let mut input_b = input_b.lock().await;
                            if !input_b.is_empty() {
                                input_b.decrement(self.material_cost);
                            }
                            drop(input_b);
                            self.get_behaviour().slots[1].mark_dirty().await;
                        } else {
                            // Cancel click
                            self.send_content_updates().await;
                            return;
                        }
                    }
                }
            }

            self.internal_on_slot_click(slot_index, button, action_type, player)
                .await;
            if slot_index == 0 || slot_index == 1 || slot_index == 2 {
                self.update_result_slot(player).await;
                self.send_content_updates().await;
            }
        })
    }
}
