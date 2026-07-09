# Pumpkin Event API Expansion Plan

## Goal

Expose the Bukkit/Spigot events that mcMMO depends on so that mcMMO-style logic can be written as native DLL plugins for Pumpkin. This document describes the order, integration points, and implementation patterns.

---

## Current State

Pumpkin already exposes 39 event types to DLL plugins. The event system is mature:

- Events are Rust structs in `pumpkin/src/plugin/api/events/{category}/`.
- `#[derive(Event)]` from `pumpkin-macros` implements the `Payload` trait.
- `#[cancellable]` injects `pub cancelled: bool` and implements `Cancellable`.
- `PluginManager::fire(event).await` dispatches to handlers.
- Blocking handlers receive `&mut E`; non-blocking handlers receive `&E`.

See the existing events for reference patterns:

- Non-cancellable: `pumpkin/src/plugin/api/events/server/server_tick_start.rs`
- Cancellable: `pumpkin/src/plugin/api/events/block/block_break.rs`
- Category trait usage: `pumpkin/src/plugin/api/events/player/player_chat.rs`
- Firing helpers: `send_cancellable!` in `pumpkin-macros/src/lib.rs`

---

## Integration Order

The phases below are ordered by **impact for mcMMO** and **implementation difficulty**. Within each phase, implement events in the order listed.

### Phase 1: Foundation (already present)

These events are already exposed and do not need new work. A detailed expansion with mcMMO/Pumpkin file references, testing steps, and sample output is available in [`plan-api-phase1.md`](./plan-api-phase1.md).

| Event | mcMMO use case |
|-------|----------------|
| `PlayerJoinEvent` | Welcome messages, leaderboards, ability cleanup |
| `PlayerLeaveEvent` | Save data, cleanup |
| `PlayerChatEvent` | Party/admin chat |
| `PlayerCommandSendEvent` | `/mcmmo` commands |
| `PlayerInteractEvent` | Ability activation, tool inspection |
| `PlayerInteractEntityEvent` | Taming, leashing |
| `PlayerFishEvent` | Fishing skill |
| `PlayerTeleportEvent` | Ability cancellation |
| `PlayerChangeWorldEvent` | Per-world config |
| `PlayerRespawnEvent` | Skill retention |
| `InventoryClickEvent` | Inventory management |
| `BlockBreakEvent` | Mining, woodcutting, excavation |
| `BlockPlaceEvent` | Repair, construction |
| `BlockGrowEvent` | Herbalism crop tracking |
| `EntitySpawnEvent` | Spawn tracking (partial) |

**Action:** verify these are all actually fired from the expected code paths. `BlockGrowEvent` is currently crop-only; ensure it covers all growth callers.

---

### Phase 2: Block Interaction Events

These unlock the core mcMMO gathering skills (Mining, Woodcutting, Excavation, Herbalism) and are the easiest to add after the existing block events. A detailed expansion with mcMMO/Pumpkin file references, code snippets, testing steps, and sample output is available in [`plan-api-phase2.md`](./plan-api-phase2.md).

| Event | Why mcMMO needs it | Suggested Pumpkin integration |
|-------|--------------------|-------------------------------|
| `BlockDamageEvent` | Super Breaker / Tree Feller / Giga Drill activation triggers when a player *starts* breaking a block | Add to the Java/Bedrock packet handler that processes `PlayerAction` (start digging). Look near `BlockBreakEvent` handling in `pumpkin/src/net/java/play.rs` and `pumpkin/src/world/mod.rs`. |
| `BlockDropItemEvent` | Double drops, auto-pickup, Herbalism green thumb | Fire where the world drops item entities after a block is broken. Add near block-break item spawning logic in `pumpkin/src/world/mod.rs` or `pumpkin/src/entity/item.rs`. |
| `BlockPistonExtendEvent` | Prevent abilities from being bypassed by pistons; logging | Add to piston extend logic in `pumpkin/src/block/blocks/redstone/piston/` or equivalent. |
| `BlockPistonRetractEvent` | Same as above | Add to piston retract logic alongside extend. |

#### Implementation notes

- `BlockDamageEvent` should be cancellable and carry `player: Option<Arc<Player>>`, `block: &'static Block`, `block_position: BlockPos`, and `insta_break: bool`.
- `BlockDropItemEvent` should be cancellable and carry the block position, block type, and the list of dropped items (as `Vec<ItemStack>` or entity refs).
- Piston events should carry the piston position, direction, and the list of affected blocks.

---

### Phase 3: Entity Damage & Death Events

These are the largest gap for mcMMO. Combat skills (Unarmed, Swords, Axes, Archery), taming, and mob tracking cannot function without them. A detailed expansion with mcMMO/Pumpkin file references, code snippets, testing steps, and sample output is available in [`plan-api-phase3.md`](./plan-api-phase3.md).

| Event | Why mcMMO needs it | Suggested Pumpkin integration |
|-------|--------------------|-------------------------------|
| `EntityDamageEvent` | Apply/reduce damage based on skills; bleeding, counter-attack | Add to the generic damage pipeline in `pumpkin/src/entity/` (look for `hurt`, `damage`, or health modification methods). |
| `EntityDamageByEntityEvent` | Swords/Axes/Unarmed active/passive abilities, archery, taming | Fire from the same pipeline when `damage_source.attacker()` is an entity. Reuse `EntityDamageEvent` fields plus `damager`. |
| `EntityDeathEvent` | Combat XP, mob head drops, loot tables | Add to living entity death logic, before drops are spawned. |
| `PlayerDeathEvent` | Death penalty, hardcore mode, keep XP | Add to player death logic near where the death screen is sent. |
| `FoodLevelChangeEvent` | Herbalism, hunger abilities | Add to player hunger/saturation update logic in `pumpkin/src/entity/player.rs`. |
| `ProjectileLaunchEvent` | Archery skill, arrow retrieval | Add to bow/crossbow/trident/snowball/egg launch logic. |
| `ProjectileHitEvent` | Archery skill effects on hit | Add to projectile collision logic in `pumpkin/src/entity/projectile/`. |

#### Implementation notes

- Damage events should be cancellable and include:
  - `entity: Arc<dyn Entity>` or typed entity reference
  - `damage_source: DamageSource` (Pumpkin’s existing type)
  - `damage: f32` (mutable)
- `EntityDamageByEntityEvent` can extend `EntityDamageEvent` or duplicate its fields plus `damager: Arc<dyn Entity>`.
- Death events should include the entity, drops, and dropped XP.
- `PlayerDeathEvent` is a specialization of `EntityDeathEvent` but keep it separate for Bukkit parity.

---

### Phase 4: Item & Inventory Events

Needed for Repair, Salvage, Alchemy, Smelting, and crafting skills.

| Event | Why mcMMO needs it | Suggested Pumpkin integration |
|-------|--------------------|-------------------------------|
| `PlayerDropItemEvent` | Salvage, item tracking | Add to the packet handler that processes drop-item actions (`pumpkin/src/net/java/play.rs`). |
| `InventoryOpenEvent` | Block access to certain inventories | Add near container open logic in `pumpkin/src/entity/player.rs` (around where `InventoryCloseEvent` is already fired). |
| `InventoryDragEvent` | Anti-cheat, custom UI | Add to inventory drag packet handling alongside `InventoryClickEvent`. |
| `InventoryMoveItemEvent` | Hopper filters, auto-sorting | Add to hopper/piston item transfer logic in `pumpkin/src/block/` or inventory code. |
| `CraftItemEvent` | Repair, custom crafting recipes | Add to crafting result slot logic. |
| `FurnaceSmeltEvent` | Smelting skill, double-smelt | Add to furnace block-entity smelting tick. |
| `FurnaceBurnEvent` | Fuel efficiency abilities | Add to furnace fuel consumption logic. |
| `FurnaceExtractEvent` | Experience tracking | Add when a player pulls from furnace output slot. |
| `BrewEvent` | Alchemy skill | Add to brewing stand block-entity logic. |

#### Implementation notes

- Inventory events should implement a shared `InventoryEvent` trait if created, but at minimum reuse the existing inventory types.
- `CraftItemEvent` needs the crafting inventory, result item, and player.
- Furnace events need the furnace block position, world, and relevant item/fuel.
- `BrewEvent` needs the brewing stand position, ingredient, and potion items.

---

### Phase 5: Entity Lifecycle & Behavior Events

Needed for Taming, Beast Lore, Call of the Wild, and advanced mob interactions.

| Event | Why mcMMO needs it | Suggested Pumpkin integration |
|-------|--------------------|-------------------------------|
| `EntityBreedEvent` | Animal taming/breeding XP | Add to animal breeding logic. |
| `EntityTameEvent` | Taming skill | Add to wolf/cat/horse tame logic. |
| `EntityTargetEvent` | Beast Lore, ability mob control | Add to mob AI target selection. |
| `EntityTargetLivingEntityEvent` | Mob aggro redirection | Fire alongside `EntityTargetEvent` when target is living. |
| `EntityPickupItemEvent` | Item tracking, auto-pickup | Add to entity item-pickup logic. |
| `EntityShootBowEvent` | Archery skill data | Fire from bow/crossbow shooting code; may overlap with `ProjectileLaunchEvent`. |
| `EntityCombustByEntityEvent` | Fire abilities | Add to entity ignition logic when ignited by another entity. |
| `EntityExplodeEvent` | Blast Mining, explosion protection | Add to TNT/creeper explosion logic. |
| `ExplosionPrimeEvent` | Pre-explosion cancellation | Add just before an explosion is processed. |
| `EntityTransformEvent` | Mob transformation tracking | Add to villager zombie/skeleton conversion logic. |
| `PotionSplashEvent` | Alchemy splash effects | Add to splash potion entity impact. |

---

### Phase 6: Block Transformation Events

Needed for accurate world state tracking in mcMMO.

| Event | Why mcMMO needs it | Suggested Pumpkin integration |
|-------|--------------------|-------------------------------|
| `BlockFormEvent` | Ice/snow formation tracking | Add to ice/snow/cobblestone generator formation logic. |
| `EntityBlockFormEvent` | Snowman snow trail, etc. | Add when entities cause block formation. |
| `EntityChangeBlockEvent` | Endermen, rabbits, withers, farmland trample | Add to entity block-modification logic. |
| `BlockMultiPlaceEvent` | Doors/beds/tables placed as multi-block | Add alongside `BlockPlaceEvent` for multi-block items. |
| `StructureGrowEvent` | Tree growth, mega mushrooms | Add to sapling/tree growth logic; can reuse `BlockGrowEvent` internally but should be a distinct event. |

---

### Phase 7: World & Chunk Events

Needed for world-based skill tracking, cleanup, and persistence. A detailed expansion with mcMMO/Pumpkin file references, code snippets, testing steps, and sample output is available in [`plan-api-phase7.md`](./plan-api-phase7.md).

| Event | Why mcMMO needs it | Suggested Pumpkin integration |
|-------|--------------------|-------------------------------|
| `ChunkUnloadEvent` | Save per-chunk data, unload tracking | Pumpkin has `ChunkLoad` and `ChunkSave` defined but not fired. Add unload firing in `pumpkin/src/world/mod.rs` chunk unload path. |
| `WorldLoadEvent` | Per-world skill config | Add where worlds are loaded at server start. |
| `WorldUnloadEvent` | Cleanup | Add where worlds are removed. |

---

## Implementation Pattern

For every new event, follow this exact sequence.

### 1. Create the event file

Example for a new cancellable player event:

```rust
// pumpkin/src/plugin/api/events/player/player_drop_item.rs
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

use crate::entity::player::Player;
use crate::item::ItemStack;
use super::PlayerEvent;

/// Fired when a player drops an item.
#[cancellable]
#[derive(Event, Clone)]
pub struct PlayerDropItemEvent {
    pub player: Arc<Player>,
    pub item: ItemStack,
}

impl PlayerDropItemEvent {
    #[must_use]
    pub fn new(player: Arc<Player>, item: ItemStack) -> Self {
        Self {
            player,
            item,
            cancelled: false,
        }
    }
}

impl PlayerEvent for PlayerDropItemEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
```

For block/entity/server/world events, place the file in the matching subdirectory and implement the matching category trait (`BlockEvent`, etc.) when applicable.

### 2. Register the module

Add `pub mod your_event;` to the category `mod.rs`:

- Player events: `pumpkin/src/plugin/api/events/player/mod.rs`
- Block events: `pumpkin/src/plugin/api/events/block/mod.rs`
- Entity events: `pumpkin/src/plugin/api/events/entity/mod.rs`
- World events: `pumpkin/src/plugin/api/events/world/mod.rs`
- Server events: `pumpkin/src/plugin/api/events/server/mod.rs`

Optionally add a `pub use your_event::YourEvent;` re-export in the same file.

### 3. Fire the event

Use `PluginManager::fire`:

```rust
let event = server
    .plugin_manager
    .fire(PlayerDropItemEvent::new(player.clone(), item.clone()))
    .await;

if event.cancelled {
    return;
}

// continue with normal drop logic
```

Or use `send_cancellable!` for cleaner branching:

```rust
send_cancellable! {{
    server;
    PlayerDropItemEvent::new(player.clone(), item.clone());

    'after: {
        // spawn item entity
    }
}};
```

### 4. Add WASM support (if desired)

If the event should also be available to WASM plugins:

1. Add a WIT record in `pumpkin-plugin-wit/v0.1/event.wit`.
2. Add the variant case to the `event` and `event-type` definitions.
3. Implement `ToFromWasmEvent` in the matching `pumpkin/src/plugin/loader/wasm/wasm_host/wit/v0_1/events/*.rs` file.

DLL plugins do **not** need WIT changes.

---

## Suggested Work Breakdown

### Milestone A — Block interaction parity

- Implement `BlockDamageEvent`
- Implement `BlockDropItemEvent`
- Implement `BlockPistonExtendEvent`
- Implement `BlockPistonRetractEvent`

### Milestone B — Combat parity

- Implement `EntityDamageEvent`
- Implement `EntityDamageByEntityEvent`
- Implement `EntityDeathEvent`
- Implement `PlayerDeathEvent`
- Implement `ProjectileLaunchEvent`
- Implement `ProjectileHitEvent`
- Implement `FoodLevelChangeEvent`

### Milestone C — Crafting/economy parity

- Implement `PlayerDropItemEvent`
- Implement `InventoryOpenEvent`
- Implement `InventoryDragEvent`
- Implement `InventoryMoveItemEvent`
- Implement `CraftItemEvent`
- Implement `FurnaceSmeltEvent`
- Implement `FurnaceBurnEvent`
- Implement `FurnaceExtractEvent`
- Implement `BrewEvent`

### Milestone D — Mob/taming parity

- Implement `EntityBreedEvent`
- Implement `EntityTameEvent`
- Implement `EntityTargetEvent`
- Implement `EntityTargetLivingEntityEvent`
- Implement `EntityPickupItemEvent`
- Implement `EntityShootBowEvent`
- Implement `EntityCombustByEntityEvent`
- Implement `EntityExplodeEvent`
- Implement `ExplosionPrimeEvent`
- Implement `EntityTransformEvent`
- Implement `PotionSplashEvent`

### Milestone E — World transformation parity

- Implement `BlockFormEvent`
- Implement `EntityBlockFormEvent`
- Implement `EntityChangeBlockEvent`
- Implement `BlockMultiPlaceEvent`
- Implement `StructureGrowEvent`
- Implement `ChunkUnloadEvent`
- Implement `WorldLoadEvent`
- Implement `WorldUnloadEvent`

---

## Testing Strategy

1. **Unit-style tests:** add a minimal DLL plugin in a new `pumpkin/examples/dll-plugin/` directory (or tests) that registers each new event and asserts it is fired.
2. **Integration tests:** write a small Rust test that constructs a `PluginManager`, registers a handler, fires the event, and checks cancellation/mutation.
3. **In-game verification:** for each milestone, run Pumpkin, join a client, and perform the action that should fire the event while a debug plugin logs it.

---

## Risks & Considerations

- **Performance:** `EntityDamageEvent` and `ProjectileLaunchEvent` are high-frequency. Keep event allocations cheap and avoid heavy work in blocking handlers.
- **Event ordering:** mcMMO sometimes relies on Bukkit’s listener priority ordering. Pumpkin’s `EventPriority` is equivalent (`Highest` → `Lowest`), so map priorities directly.
- **Async chat:** Bukkit fires `AsyncPlayerChatEvent` asynchronously. Pumpkin’s `PlayerChatEvent` is synchronous. If mcMMO logic expects async chat, evaluate whether to add an async variant or keep it sync.
- **Type design:** many of these events carry `Arc<Player>` or `Arc<dyn Entity>`. Be careful with circular references and ensure events are `Clone`.
- **WASM parity:** DLL plugins get new events automatically, but WASM plugins only see events that are explicitly added to the WIT interface. Decide per event whether WASM support is required.

---

## Reference: Already-Exposed Pumpkin Events

For convenience, the full list of events currently available to DLL plugins:

**Player:** `PlayerJoinEvent`, `PlayerLeaveEvent`, `PlayerLoginEvent`, `PlayerMoveEvent`, `PlayerChatEvent`, `PlayerCommandSendEvent`, `PlayerPermissionCheckEvent`, `PlayerTeleportEvent`, `PlayerChangeWorldEvent`, `PlayerRespawnEvent`, `PlayerExpChangeEvent`, `PlayerItemHeldEvent`, `PlayerChangedMainHandEvent`, `PlayerGamemodeChangeEvent`, `PlayerCustomPayloadEvent`, `PlayerFishEvent`, `PlayerEggThrowEvent`, `PlayerInteractEvent`, `PlayerInteractEntityEvent`, `PlayerInteractUnknownEntityEvent`, `PlayerToggleFlightEvent`, `PlayerToggleSneakEvent`, `PlayerToggleSprintEvent`, `BedrockFormResponseEvent`, `CustomClickActionEvent`, `InventoryClickEvent`, `InventoryCloseEvent`

**Block:** `BlockBreakEvent`, `BlockPlaceEvent`, `BlockGrowEvent`, `BlockRedstoneEvent`, `BlockBurnEvent` (defined, not fired), `BlockCanBuildEvent` (defined, not fired)

**Entity:** `EntitySpawnEvent`, `EntityRemoveEvent`, `ChunkEntityLoadEvent`, `ChunkEntityUnloadEvent`

**Server:** `ServerTickStartEvent`, `ServerTickEndEvent`, `ServerBroadcastEvent`, `ServerCommandEvent`, `PacketReceivedEvent`, `PacketSentEvent`

**World:** `SpawnChangeEvent`, `ChunkSend`, `ChunkLoad` (defined, not fired), `ChunkSave` (defined, not fired)

---

*Plan generated from analysis of `C:/Users/destr/Documents/Minecraft/Pumpkin` and comparison with mcMMO event usage.*
