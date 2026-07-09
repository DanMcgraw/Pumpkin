# Phase 2 — Block Interaction Event Parity Plan

This document is the detailed expansion of **Phase 2** from [`plan-api.md`](./plan-api.md). It covers the block-interaction events that mcMMO depends on and that are **not yet exposed** to native DLL plugins in Pumpkin.

**Goal of Phase 2:** implement and fire `BlockDamageEvent`, `BlockDropItemEvent`, `BlockPistonExtendEvent`, and `BlockPistonRetractEvent` so that the core mcMMO gathering skills (Mining, Woodcutting, Excavation, Herbalism) and anti-exploit logic can be ported.

---

## Phase 2 Event Checklist

| # | Bukkit/Spigot event (mcMMO) | Pumpkin event | Status |
|---|-----------------------------|---------------|--------|
| 1 | `BlockDamageEvent` | `BlockDamageEvent` | ❌ Not implemented |
| 2 | `BlockDropItemEvent` | `BlockDropItemEvent` | ❌ Not implemented |
| 3 | `BlockPistonExtendEvent` | `BlockPistonExtendEvent` | ❌ Not implemented |
| 4 | `BlockPistonRetractEvent` | `BlockPistonRetractEvent` | ❌ Not implemented |

---

## 1. BlockDamageEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/BlockListener.java:608-776`

mcMMO registers **three** handlers for `BlockDamageEvent` at different priorities:

1. **MONITOR priority** (`onBlockDamage`): ability preparation checks (Green Terra, Tree Feller, Super Breaker, Giga Drill Breaker, Berserk) and Tree Feller sounds.
2. **HIGHEST priority** (`onBlockDamageHigher`): ability trigger checks (Green Terra moss conversion, Berserk insta-break/glass break, Leaf Blower).
3. **MONITOR priority** (`onBlockDamageCleanup`): cleanup ability tool buffs and debug stick output.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onBlockDamage(BlockDamageEvent event) {
    final Player player = event.getPlayer();
    final Block block = event.getBlock();
    // ... world blacklist / world guard checks ...

    if (BlockUtils.canActivateAbilities(block)) {
        ItemStack heldItem = player.getInventory().getItemInMainHand();

        if (mmoPlayer.getToolPreparationMode(ToolType.HOE) && ItemUtils.isHoe(heldItem)
                && Permissions.greenTerra(player)) {
            mmoPlayer.checkAbilityActivation(PrimarySkillType.HERBALISM);
        } else if (mmoPlayer.getToolPreparationMode(ToolType.AXE) && ItemUtils.isAxe(heldItem)
                && BlockUtils.hasWoodcuttingXP(block) && Permissions.treeFeller(player)) {
            mmoPlayer.checkAbilityActivation(PrimarySkillType.WOODCUTTING);
        } else if (mmoPlayer.getToolPreparationMode(ToolType.PICKAXE) && ItemUtils.isPickaxe(heldItem)
                && BlockUtils.affectedBySuperBreaker(block) && Permissions.superBreaker(player)) {
            mmoPlayer.checkAbilityActivation(PrimarySkillType.MINING);
        } else if (mmoPlayer.getToolPreparationMode(ToolType.SHOVEL) && ItemUtils.isShovel(heldItem)
                && BlockUtils.affectedByGigaDrillBreaker(block) && Permissions.gigaDrillBreaker(player)) {
            mmoPlayer.checkAbilityActivation(PrimarySkillType.EXCAVATION);
        } else if (mmoPlayer.getToolPreparationMode(ToolType.FISTS)
                && heldItem.getType() == Material.AIR && Permissions.berserk(player)) {
            mmoPlayer.checkAbilityActivation(PrimarySkillType.UNARMED);

            if (mmoPlayer.getAbilityMode(SuperAbilityType.BERSERK)) {
                if (SuperAbilityType.BERSERK.blockCheck(block) && EventUtils.simulateBlockBreak(block, player)) {
                    event.setInstaBreak(true);
                    // ... sounds ...
                }
            }
        }
    }

    if (mmoPlayer.getAbilityMode(SuperAbilityType.TREE_FELLER) && BlockUtils.hasWoodcuttingXP(block)) {
        SoundManager.sendSound(player, block.getLocation(), SoundType.FIZZ);
    }
}
```

Key fields mcMMO reads:

- `event.getPlayer()` — who damaged the block.
- `event.getBlock()` — the block being damaged.
- `event.getInstaBreak()` — whether the block will break instantly.
- `event.setInstaBreak(true)` — Berserk/Leaf Blower insta-break behavior.

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/block/block_damage.rs`

Event shape:

```rust
use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

use crate::entity::player::Player;
use super::BlockEvent;

/// Fired when a player starts damaging (mining) a block.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockDamageEvent {
    pub player: Arc<Player>,
    pub block: &'static Block,
    pub block_position: BlockPos,
    pub insta_break: bool,
}

impl BlockDamageEvent {
    pub fn new(player: Arc<Player>, block: &'static Block, block_position: BlockPos, insta_break: bool) -> Self {
        Self {
            player,
            block,
            block_position,
            insta_break,
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockDamageEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}
```

**Register module:** add `pub mod block_damage;` to `pumpkin/src/plugin/api/events/block/mod.rs`.

**Fire the event:** `pumpkin/src/net/java/play.rs:1841-1952`, in `handle_player_action` under `Status::StartedDigging`.

Insert after the reach check and before the creative-mode fast path:

```rust
Status::StartedDigging => {
    if !player.can_interact_with_block_at(&player_action.position, 1.0) {
        // ... existing reach warning ...
        return;
    }

    let position = player_action.position;
    let entity = &player.get_entity();
    let world = entity.world.load_full();
    let (block, state) = world.get_block_and_state(&position);

    // --- NEW: BlockDamageEvent ---
    let speed = block::calc_block_breaking(player, state, block).await;
    let insta_break = speed >= 1.0 || player.gamemode.load() == GameMode::Creative;
    let damage_event = BlockDamageEvent::new(
        player.clone(),
        block,
        position,
        insta_break,
    );
    let damage_event = server.plugin_manager.fire(damage_event).await;

    if damage_event.cancelled {
        // Re-sync the block to the client so it reappears
        self.enqueue_packet(&CBlockUpdate::new(
            position,
            VarInt(i32::from(state.id.as_u16())),
        )).await;
        self.update_sequence(player, player_action.sequence.0);
        return;
    }

    let insta_break = damage_event.insta_break;
    // --- END NEW ---

    if block == &pumpkin_data::Block::NOTE_BLOCK {
        // ... existing note block logic ...
    }

    if !server.item_registry.can_mine(held.lock().await.item, player) {
        // ... existing cannot-mine handling ...
        return;
    }

    if player.gamemode.load() == GameMode::Creative {
        // creative insta-break
    } else if insta_break {
        // survival insta-break (speed >= 1.0)
    } else {
        // start mining progress
    }
}
```

### Required behavior for mcMMO parity

- Must fire when the player **starts** digging a block (`StartedDigging`).
- Must be cancellable so other plugins can prevent block damage.
- Must allow `insta_break` to be toggled by plugins (Berserk, Leaf Blower).
- Must include the player, block, and position.

### Gaps / action items

- mcMMO uses `FakeBlockDamageEvent` internally to trigger abilities on simulated clicks. A Pumpkin port will need a similar internal mechanism (e.g., construct and fire `BlockDamageEvent` programmatically from an ability command).
- Add `hand: Hand` to the event if mcMMO needs to distinguish main/off hand.

---

## 2. BlockDropItemEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/BlockListener.java:75-199`

mcMMO uses `BlockDropItemEvent` at **LOWEST** priority to:

- Clean up metadata (`METADATA_KEY_BONUS_DROPS`, `METADATA_KEY_EXCAVATION_TREASURE_ROLL`) when the event is cancelled.
- Apply bonus drops from Mining, Herbalism, and Woodcutting.
- Inject Excavation treasure drops for natural blocks.

```java
@EventHandler(priority = EventPriority.LOWEST, ignoreCancelled = false)
public void onBlockDropItemEvent(BlockDropItemEvent event) {
    final Block block = event.getBlock();
    if (event.isCancelled()) {
        // cleanup metadata
        return;
    }

    // Apply bonus drops
    if (!block.getMetadata(METADATA_KEY_BONUS_DROPS).isEmpty()) {
        // double drops etc.
    }

    // Inject excavation treasures
    if (block.hasMetadata(METADATA_KEY_EXCAVATION_TREASURE_ROLL)) {
        // roll treasure drops and add them to event.getItems()
    }
}
```

Key fields mcMMO reads:

- `event.getBlock()` — the block that was broken (may be AIR by the time the event fires).
- `event.getPlayer()` — the player who broke it.
- `event.getItems()` — mutable list of dropped item entities.
- `event.isCancelled()` — cleanup metadata on cancel.

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/block/block_drop_item.rs`

Event shape:

```rust
use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

use crate::entity::player::Player;
use crate::item::ItemStack;
use super::BlockEvent;

/// Fired when a block drops items after being broken.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockDropItemEvent {
    pub player: Arc<Player>,
    pub block: &'static Block,
    pub block_position: BlockPos,
    pub items: Vec<ItemStack>,
}

impl BlockDropItemEvent {
    pub fn new(
        player: Arc<Player>,
        block: &'static Block,
        block_position: BlockPos,
        items: Vec<ItemStack>,
    ) -> Self {
        Self {
            player,
            block,
            block_position,
            items,
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockDropItemEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}
```

**Register module:** add `pub mod block_drop_item;` to `pumpkin/src/plugin/api/events/block/mod.rs`.

**Fire the event:** The best place is inside `World::break_block` in `pumpkin/src/world/mod.rs:4737-4766`, where loot is currently dropped via `block::drop_loot`.

Refactor the drop section as follows:

```rust
if !flags.contains(BlockFlags::SKIP_DROPS) {
    let tool = if let Some(player) = &cause {
        let hand_stack = player.inventory.get_stack_in_hand(pumpkin_util::Hand::Right).await;
        let stack_guard = hand_stack.lock().await;
        (stack_guard.item_count > 0).then(|| stack_guard.clone())
    } else {
        None
    };

    let is_raining = self.is_raining().await;
    let is_thundering = self.is_thundering().await;

    let params = LootContextParameters {
        block_state: Some(BlockState::from_id(broken_state_id)),
        luck,
        position: Some(...),
        world_time: self.level_info.load().day_time as u64,
        tool,
        is_raining: Some(is_raining),
        is_thundering: Some(is_thundering),
        ..Default::default()
    };

    // --- NEW: collect loot, fire event, then drop ---
    let mut dropped_items = Vec::new();
    if let Some(loot_table) = &broken_block.loot_table {
        dropped_items.extend(loot_table.get_loot(params));
    }

    if let Some(player) = &cause {
        let drop_event = BlockDropItemEvent::new(
            player.clone(),
            broken_block,
            *position,
            dropped_items,
        );
        let drop_event = self.server.upgrade().unwrap().plugin_manager.fire(drop_event).await;

        if !drop_event.cancelled {
            for stack in drop_event.items {
                self.drop_stack(position, stack).await;
            }
        }
    } else {
        for stack in dropped_items {
            self.drop_stack(position, stack).await;
        }
    }
    // --- END NEW ---
}
```

**Alternative:** keep `block::drop_loot` as the low-level helper and pass the already-computed `Vec<ItemStack>` into `BlockDropItemEvent`, then have the caller actually spawn item entities. This avoids duplicating loot logic.

### Required behavior for mcMMO parity

- Fire after the block is broken but **before** item entities are spawned.
- Include the block (pre-break type), position, player, and the list of item stacks to drop.
- Allow plugins to mutate the item list, cancel drops, or add new drops.

### Gaps / action items

- Pumpkin currently spawns item entities one by one via `World::drop_stack`. `BlockDropItemEvent` should collect the drops as `ItemStack`s, let plugins modify them, then spawn entities.
- mcMMO stores `METADATA_KEY_BONUS_DROPS` on the `Block`. Pumpkin has no equivalent metadata API. A port will need a custom block tracker (e.g., `HashMap<BlockPos, BlockTrackerEntry>` per world) to store natural/unnatural state and bonus-drop counts.
- mcMMO stores `METADATA_KEY_EXCAVATION_TREASURE_ROLL` on the block. Same tracker requirement.

---

## 3. BlockPistonExtendEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/BlockListener.java:201-228`

mcMMO monitors `BlockPistonExtendEvent` at **MONITOR** priority to:

- Prevent piston-based skill exploitation by marking moved blocks as unnatural.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onBlockPistonExtend(BlockPistonExtendEvent event) {
    if (WorldBlacklist.isWorldBlacklisted(event.getBlock().getWorld())) {
        return;
    }

    if (!ExperienceConfig.getInstance().isPistonCheatingPrevented()) {
        return;
    }

    final BlockFace direction = event.getDirection();

    for (final Block block : event.getBlocks()) {
        mcMMO.p.getFoliaLib().getScheduler().runAtLocation(block.getLocation(), t -> {
            final Block movedBlock = block.getRelative(direction);
            if (BlockUtils.isWithinWorldBounds(movedBlock)) {
                BlockUtils.setUnnaturalBlock(movedBlock);
            }
        });
    }
}
```

Key fields mcMMO reads:

- `event.getBlock()` — the piston base.
- `event.getDirection()` — which way the piston extends.
- `event.getBlocks()` — the list of blocks being pushed.

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/block/block_piston_extend.rs`

Event shape:

```rust
use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use std::sync::Arc;

use crate::world::World;
use super::BlockEvent;

/// Fired when a piston extends.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockPistonExtendEvent {
    pub world: Arc<World>,
    pub piston_pos: BlockPos,
    pub piston_block: &'static Block,
    pub direction: Vector3<i32>,
    pub moved_blocks: Vec<BlockPos>,
    pub broken_blocks: Vec<BlockPos>,
}

impl BlockPistonExtendEvent {
    pub fn new(
        world: Arc<World>,
        piston_pos: BlockPos,
        piston_block: &'static Block,
        direction: Vector3<i32>,
        moved_blocks: Vec<BlockPos>,
        broken_blocks: Vec<BlockPos>,
    ) -> Self {
        Self {
            world,
            piston_pos,
            piston_block,
            direction,
            moved_blocks,
            broken_blocks,
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockPistonExtendEvent {
    fn get_block(&self) -> &Block {
        self.piston_block
    }
}
```

**Register module:** add `pub mod block_piston_extend;` to `pumpkin/src/plugin/api/events/block/mod.rs`.

**Fire the event:** `pumpkin/src/block/blocks/piston/piston.rs:170-191`, inside `on_synced_block_event` when `r#type == 0` (extend).

Insert before the piston actually moves blocks:

```rust
// r#type == 0 means extend
if r#type == 0 {
    // --- NEW: BlockPistonExtendEvent ---
    let moved_positions: Vec<BlockPos> = handler.moved_blocks.clone();
    let broken_positions: Vec<BlockPos> = handler.broken_blocks.clone();

    let extend_event = BlockPistonExtendEvent::new(
        world.clone(),
        *pos,
        block,
        dir.to_offset(),
        moved_positions,
        broken_positions,
    );
    let extend_event = world.server.upgrade().unwrap().plugin_manager.fire(extend_event).await;

    if extend_event.cancelled {
        return false;
    }
    // --- END NEW ---

    if !move_piston(world, dir, pos, true, sticky).await {
        return false;
    }
    // ... rest of extend logic ...
}
```

`PistonHandler::calculate_push()` must be called before firing the event so the list of moved/broken blocks is known. In the current code, `move_piston` calls `calculate_push()` internally. Split it so the calculation happens first, the event fires, and then `move_piston` executes with the pre-calculated `PistonHandler`.

### Required behavior for mcMMO parity

- Fire before blocks are actually moved.
- Be cancellable.
- Provide the piston position, direction, and the list of blocks being pushed.
- Distinguish pushed blocks from blocks that will be broken (e.g., by a slime block launch).

### Gaps / action items

- The `move_piston` function currently both calculates and executes. Refactor to expose `PistonHandler` results before execution.
- mcMMO marks blocks as unnatural after the move. Plugins can do this in a MONITOR handler after the event fires.

---

## 4. BlockPistonRetractEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/BlockListener.java:235-262`

mcMMO monitors `BlockPistonRetractEvent` at **MONITOR** priority to:

- Mark the block in front of the piston and any pulled blocks as unnatural.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onBlockPistonRetract(BlockPistonRetractEvent event) {
    if (WorldBlacklist.isWorldBlacklisted(event.getBlock().getWorld())) {
        return;
    }

    if (!ExperienceConfig.getInstance().isPistonCheatingPrevented()) {
        return;
    }

    BlockFace direction = event.getDirection();
    Block movedBlock = event.getBlock().getRelative(direction);

    if (BlockUtils.isWithinWorldBounds(movedBlock)) {
        BlockUtils.setUnnaturalBlock(movedBlock);
    }

    for (Block block : event.getBlocks()) {
        // mark pulled blocks as unnatural
    }
}
```

Key fields mcMMO reads:

- `event.getBlock()` — the piston base.
- `event.getDirection()` — retraction direction.
- `event.getBlocks()` — blocks being pulled (for sticky pistons).

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/block/block_piston_retract.rs`

Event shape (similar to extend):

```rust
use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use std::sync::Arc;

use crate::world::World;
use super::BlockEvent;

/// Fired when a piston retracts.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockPistonRetractEvent {
    pub world: Arc<World>,
    pub piston_pos: BlockPos,
    pub piston_block: &'static Block,
    pub direction: Vector3<i32>,
    pub moved_blocks: Vec<BlockPos>,
}

impl BlockPistonRetractEvent {
    pub fn new(
        world: Arc<World>,
        piston_pos: BlockPos,
        piston_block: &'static Block,
        direction: Vector3<i32>,
        moved_blocks: Vec<BlockPos>,
    ) -> Self {
        Self {
            world,
            piston_pos,
            piston_block,
            direction,
            moved_blocks,
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockPistonRetractEvent {
    fn get_block(&self) -> &Block {
        self.piston_block
    }
}
```

**Register module:** add `pub mod block_piston_retract;` to `pumpkin/src/plugin/api/events/block/mod.rs`.

**Fire the event:** `pumpkin/src/block/blocks/piston/piston.rs:193-288`, inside `on_synced_block_event` when `r#type == 1` or `2` (retract).

Insert before the retract logic executes:

```rust
// r#type == 1 or 2 means retract
} else {
    // --- NEW: BlockPistonRetractEvent ---
    let retract_event = BlockPistonRetractEvent::new(
        world.clone(),
        *pos,
        block,
        dir.to_offset(),
        Vec::new(), // TODO: populate pulled blocks for sticky pistons
    );
    let retract_event = world.server.upgrade().unwrap().plugin_manager.fire(retract_event).await;

    if retract_event.cancelled {
        return false;
    }
    // --- END NEW ---

    // ... existing retract logic ...
}
```

For sticky pistons, the pulled blocks are determined in `move_piston` with `extend = false`. Refactor so the calculation happens before the event, similar to extend.

### Required behavior for mcMMO parity

- Fire before blocks are actually pulled/moved.
- Be cancellable.
- Provide piston position, direction, and pulled blocks.

### Gaps / action items

- Sticky piston retraction pulls blocks. The current `move_piston` function handles both extend and retract. Refactor to compute pulled blocks before firing the event.
- Non-sticky piston retraction does not move blocks, but the event should still fire for consistency.

---

## Implementation Order Within Phase 2

1. **BlockDamageEvent** — unlocks ability activation and is the simplest new event.
2. **BlockDropItemEvent** — unlocks bonus drops and excavation treasures; depends on loot drops.
3. **BlockPistonExtendEvent** — requires small refactor of `move_piston`.
4. **BlockPistonRetractEvent** — same refactor as extend.

---

## Step-by-Step Testing Guide

### Setup

1. Build Pumpkin with the new events.
2. Create a test DLL plugin that registers handlers for all 4 Phase 2 events and logs each firing.
3. Ensure the plugin can also cancel events to verify cancellation behavior.

### Manual test script

| Step | Action | Expected event(s) logged |
|------|--------|--------------------------|
| 1 | Start digging a stone block | `BlockDamageEvent: player=Steve, block=stone, insta_break=false` |
| 2 | Start digging a dirt block by hand | `BlockDamageEvent: player=Steve, block=dirt, insta_break=true` |
| 3 | Finish breaking a coal ore block | `BlockBreakEvent`, then `BlockDropItemEvent` with coal item(s) |
| 4 | Cancel `BlockDropItemEvent` via plugin and break a block | Block breaks, but no item entities spawn |
| 5 | Place a block in front of a piston and power it | `BlockPistonExtendEvent` with the pushed block in `moved_blocks` |
| 6 | Cancel `BlockPistonExtendEvent` via plugin and power piston | Piston does not extend |
| 7 | Power a sticky piston with a block attached, then unpower | `BlockPistonRetractEvent` with the pulled block in `moved_blocks` |
| 8 | Cancel `BlockPistonRetractEvent` via plugin and unpower | Piston does not retract/pull blocks |

### Automated test

Add a Rust test in `pumpkin/tests/phase2_events.rs` that:

1. Creates a `PluginManager` and a mock `World`/`Player`.
2. Fires `BlockDamageEvent`, `BlockDropItemEvent`, `BlockPistonExtendEvent`, and `BlockPistonRetractEvent`.
3. Verifies each handler receives the event.
4. Verifies cancellation prevents the downstream action.

---

## Sample `output.log`

```text
[2026-07-08T23:00:01Z INFO  phase2_test_plugin] BlockDamageEvent: player=Steve, block=stone, pos=BlockPos { x: 10, y: 64, z: -20 }, insta_break=false
[2026-07-08T23:00:03Z INFO  phase2_test_plugin] BlockDamageEvent: player=Steve, block=dirt, pos=BlockPos { x: 10, y: 64, z: -19 }, insta_break=true
[2026-07-08T23:00:05Z INFO  phase2_test_plugin] BlockBreakEvent: player=Steve, block=coal_ore, pos=BlockPos { x: 11, y: 64, z: -18 }
[2026-07-08T23:00:05Z INFO  phase2_test_plugin] BlockDropItemEvent: player=Steve, block=coal_ore, pos=BlockPos { x: 11, y: 64, z: -18 }, items=[ItemStack { item: coal, count: 1 }]
[2026-07-08T23:00:08Z INFO  phase2_test_plugin] BlockPistonExtendEvent: piston_pos=BlockPos { x: 5, y: 64, z: 0 }, direction=(1, 0, 0), moved_blocks=[BlockPos { x: 6, y: 64, z: 0 }], broken_blocks=[]
[2026-07-08T23:00:12Z INFO  phase2_test_plugin] BlockPistonRetractEvent: piston_pos=BlockPos { x: 5, y: 64, z: 0 }, direction=(1, 0, 0), moved_blocks=[BlockPos { x: 6, y: 64, z: 0 }]
```

---

## Phase 2 Completion Criteria

Phase 2 is complete when:

1. All 4 events are defined, registered, and fire from the documented code paths.
2. A test DLL plugin confirms each event fires during the manual test script.
3. Cancellation works for all 4 events and prevents the corresponding game action.
4. `BlockDamageEvent` correctly reports `insta_break` for instant-break blocks.
5. `BlockDropItemEvent` exposes the item drops and allows plugins to mutate/cancel them.
6. Piston events expose moved/pulled blocks and fire before the movement occurs.
7. The automated smoke test passes.

---

## References

- Parent plan: [`plan-api.md`](./plan-api.md)
- Phase 1 detail: [`plan-api-phase1.md`](./plan-api-phase1.md)
- mcMMO source: `../mcMMO/src/main/java/com/gmail/nossr50/listeners/BlockListener.java`
- Pumpkin event definitions: `pumpkin/src/plugin/api/events/block/`
- Pumpkin player action handling: `pumpkin/src/net/java/play.rs:1829-2045`
- Pumpkin block breaking: `pumpkin/src/world/mod.rs:4657-4771`
- Pumpkin piston logic: `pumpkin/src/block/blocks/piston/piston.rs`

---

*Document generated for Phase 2 of the Pumpkin / mcMMO event parity effort.*
