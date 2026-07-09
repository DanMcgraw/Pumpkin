# Phase 7 — World & Chunk Event Parity Plan

This document is the detailed expansion of **Phase 7** from [`plan-api.md`](./plan-api.md). It covers the world and chunk lifecycle events that mcMMO depends on for per-world skill tracking, cleanup, persistence, and anti-exploit logic.

**Goal of Phase 7:** implement and fire `ChunkUnloadEvent`, `WorldLoadEvent`, and `WorldUnloadEvent` so that mcMMO-style plugins can react to world/chunk lifecycle changes and perform per-world configuration, data saving, and cleanup.

---

## Phase 7 Event Checklist

| # | Bukkit/Spigot event (mcMMO) | Pumpkin event | Status |
|---|-----------------------------|---------------|--------|
| 1 | `ChunkUnloadEvent` | `ChunkUnloadEvent` | ❌ Not implemented |
| 2 | `WorldLoadEvent` | `WorldLoadEvent` | ❌ Not implemented |
| 3 | `WorldUnloadEvent` | `WorldUnloadEvent` | ❌ Not implemented |

---

## 1. ChunkUnloadEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/BlockListener.java` (indirectly via `ChunkUnloadEvent` listeners)

mcMMO uses `ChunkUnloadEvent` to:

- Save per-chunk transient metadata (natural/unnatural block tracking, eligible blocks) to disk or clean it from memory.
- Flush any pending block tracker changes for the chunk before it is fully discarded.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onChunkUnload(ChunkUnloadEvent event) {
    Chunk chunk = event.getChunk();
    mcMMO.getUserBlockTracker().chunkUnloaded(chunk);
}
```

Key fields mcMMO reads:

- `event.getChunk()` — the chunk being unloaded.
- `event.getWorld()` — the world containing the chunk.

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/world/chunk_unload.rs`

Event shape:

```rust
use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_world::chunk::ChunkData;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Fired when a chunk is unloaded from a world.
#[cancellable]
#[derive(Event, Clone)]
pub struct ChunkUnloadEvent {
    /// The world from which the chunk is being unloaded.
    pub world: Arc<World>,
    /// The chunk data being unloaded.
    pub chunk: Arc<RwLock<ChunkData>>,
    /// The chunk position.
    pub pos: Vector2<i32>,
}

impl ChunkUnloadEvent {
    pub fn new(
        world: Arc<World>,
        chunk: Arc<RwLock<ChunkData>>,
        pos: Vector2<i32>,
    ) -> Self {
        Self {
            world,
            chunk,
            pos,
            cancelled: false,
        }
    }
}
```

**Register module:** add `pub mod chunk_unload;` and `pub use chunk_unload::ChunkUnloadEvent;` to `pumpkin/src/plugin/api/events/world/mod.rs`.

**Fire the event:** `pumpkin-world/src/chunk_system/schedule.rs:632-672`, inside `GenerationSchedule::process_unload_queue`.

Insert after a chunk is removed from `chunk_map` but before it is queued for I/O write:

```rust
fn process_unload_queue(&mut self) {
    // ... existing logic that swaps unload_chunks out of self.unload_chunks ...

    for pos in unload_chunks {
        let holder = self.chunk_map.get_mut(&pos).unwrap();
        // ... existing target_stage / occupied checks ...

        if holder.occupied.is_null() {
            let mut tmp = None;
            swap(&mut holder.chunk, &mut tmp);
            let Some(tmp) = tmp else { continue; };

            match tmp {
                Chunk::Level(chunk) => {
                    if holder.public {
                        self.public_chunk_map.remove(&pos);
                        holder.public = false;
                    }

                    // --- NEW: ChunkUnloadEvent ---
                    // The world reference is held by the level; obtain Arc<World> from
                    // level.world_portal or store a Weak<World> in GenerationSchedule.
                    if let Some(world) = self.level.world_portal.load().as_ref()
                        .and_then(|portal| portal.upgrade_world_arc())
                    {
                        let event = ChunkUnloadEvent::new(
                            world,
                            Arc::new(RwLock::new(chunk.data.clone())),
                            pos,
                        );
                        let server = world.server.upgrade();
                        if let Some(server) = server {
                            let event = server.plugin_manager.fire(event).await;
                            if event.cancelled {
                                // Re-insert the chunk into the chunk_map and continue
                                // without saving/unloading it.
                                continue;
                            }
                        }
                    }
                    // --- END NEW ---

                    if chunk.is_dirty() {
                        chunks.push((pos, Chunk::Level(chunk)));
                    }
                    self.chunk_map.remove(&pos);
                }
                Chunk::Proto(chunk) => {
                    // Proto chunks are not yet public; fire event similarly if desired.
                    chunks.push((pos, Chunk::Proto(chunk)));
                    self.chunk_map.remove(&pos);
                }
            }
        } else {
            self.unload_chunks.insert(pos);
        }
    }

    // ... existing I/O write scheduling ...
}
```

**Important:** `GenerationSchedule` does not currently hold an `Arc<World>`. The `Level` stores a `world_portal: ArcSwap<Option<Arc<dyn WorldPortalExt>>>`. `WorldPortalExt` is implemented by `WorldPortal(Arc<World>)` in `pumpkin/src/world/mod.rs`. You will need to expose a method on `WorldPortalExt` (or cast) to retrieve `Arc<World>` so the event can be fired with the correct world reference. The simplest approach is to store a `Weak<World>` inside `GenerationSchedule` when the `Level` is created.

### Required behavior for mcMMO parity

- Must fire when a loaded chunk is removed from memory.
- Must expose the world and chunk (or at least chunk position).
- Should be cancellable so plugins can keep a chunk loaded temporarily.
- Should fire before the chunk data is written to disk so plugins can modify saved data.

### Gaps / action items

- `GenerationSchedule` needs access to `Arc<World>` (or `Weak<World>`) to construct the event. Add this during `Level` initialization.
- The event should not fire for proto chunks unless plugins need it. Start with level chunks only.
- Chunk unload is synchronous inside the chunk system tick. Firing an async event requires spawning a task or refactoring `process_unload_queue` to be async. If async firing is not practical, fire a synchronous event or buffer unload events and fire them at the end of the tick.

---

## 2. WorldLoadEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/WorldListener.java`

mcMMO uses `WorldLoadEvent` to:

- Load per-world configuration (world blacklists, experience multipliers).
- Initialize world-specific trackers (block trackers, mob health multipliers).

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onWorldLoad(WorldLoadEvent event) {
    World world = event.getWorld();
    mcMMO.getUserBlockTracker().loadWorld(world);
    mcMMO.getExperienceConfig().loadWorld(world);
}
```

Key fields mcMMO reads:

- `event.getWorld()` — the world that was loaded.

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/world/world_load.rs`

Event shape:

```rust
use crate::world::World;
use pumpkin_macros::Event;
use std::sync::Arc;

/// Fired when a world is loaded.
#[derive(Event, Clone)]
pub struct WorldLoadEvent {
    /// The world that was loaded.
    pub world: Arc<World>,
}

impl WorldLoadEvent {
    pub fn new(world: Arc<World>) -> Self {
        Self { world }
    }
}
```

**Register module:** add `pub mod world_load;` and `pub use world_load::WorldLoadEvent;` to `pumpkin/src/plugin/api/events/world/mod.rs`.

**Fire the event:** `pumpkin/src/server/mod.rs:337-347`, after all worlds have finished loading.

Insert after `server.worlds.store(Arc::new(worlds_vec));`:

```rust
server.worlds.store(Arc::new(worlds_vec));

// --- NEW: WorldLoadEvent ---
for world in server.worlds.load().iter() {
    server.plugin_manager.fire(WorldLoadEvent::new(world.clone())).await;
}
// --- END NEW ---

info!("All worlds loaded successfully.");
```

If worlds can be loaded dynamically after startup, also fire the event wherever a new world is created (e.g., `pumpkin/src/server/mod.rs:422`).

### Required behavior for mcMMO parity

- Must fire once for each world after it is fully loaded and available.
- Must expose the loaded world.
- Does not need to be cancellable (Bukkit's is not cancellable).

### Gaps / action items

- Pumpkin currently loads all dimensions in parallel at startup. Ensure the event fires after the `worlds` ArcSwap is populated.
- Dynamic world loading (if supported) must also fire this event.

---

## 3. WorldUnloadEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/WorldListener.java`

mcMMO uses `WorldUnloadEvent` to:

- Save and discard per-world transient data (block trackers, experience caches).
- Clean up world-specific scheduled tasks.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onWorldUnload(WorldUnloadEvent event) {
    World world = event.getWorld();
    mcMMO.getUserBlockTracker().unloadWorld(world);
}
```

Key fields mcMMO reads:

- `event.getWorld()` — the world being unloaded.
- `event.isCancelled()` — plugins can prevent unloading.

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/world/world_unload.rs`

Event shape:

```rust
use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

/// Fired when a world is about to be unloaded.
#[cancellable]
#[derive(Event, Clone)]
pub struct WorldUnloadEvent {
    /// The world being unloaded.
    pub world: Arc<World>,
}

impl WorldUnloadEvent {
    pub fn new(world: Arc<World>) -> Self {
        Self {
            world,
            cancelled: false,
        }
    }
}
```

**Register module:** add `pub mod world_unload;` and `pub use world_unload::WorldUnloadEvent;` to `pumpkin/src/plugin/api/events/world/mod.rs`.

**Fire the event:** `pumpkin/src/server/mod.rs`, before `World::shutdown` is called.

Pumpkin does not currently appear to unload worlds dynamically at runtime; worlds are only shut down on server stop. Add the event before the shutdown loop:

```rust
// In server shutdown path (e.g., Server::shutdown or Drop impl)
for world in self.worlds.load().iter() {
    let event = WorldUnloadEvent::new(world.clone());
    let event = self.plugin_manager.fire(event).await;
    if event.cancelled {
        continue;
    }
    world.shutdown().await;
}
```

If a `/unloadworld` command or dynamic unload is added later, fire the event there too.

### Required behavior for mcMMO parity

- Must fire before the world is fully shut down.
- Must expose the world being unloaded.
- Should be cancellable (Bukkit's is cancellable).

### Gaps / action items

- Verify whether Pumpkin has a centralized server shutdown path that iterates worlds. If not, add one.
- If dynamic world unloading is added in the future, ensure the event fires there as well.

---

## Implementation Order Within Phase 7

1. **WorldLoadEvent** — simplest; fires once per world at startup.
2. **ChunkUnloadEvent** — requires plumbing `Arc<World>` into the chunk system.
3. **WorldUnloadEvent** — requires identifying or adding the server shutdown path.

---

## Step-by-Step Testing Guide

### Setup

1. Build Pumpkin with the new events.
2. Create a test DLL plugin that registers handlers for `WorldLoadEvent`, `ChunkUnloadEvent`, and `WorldUnloadEvent` and logs each firing.
3. Ensure the plugin can also cancel `ChunkUnloadEvent` and `WorldUnloadEvent` to verify cancellation behavior.

### Manual test script

| Step | Action | Expected event(s) logged |
|------|--------|--------------------------|
| 1 | Start server | `WorldLoadEvent: world=overworld`, `world=the_nether`, `world=the_end` |
| 2 | Teleport far away, wait for chunk unload grace period | `ChunkUnloadEvent: world=overworld, pos=ChunkPos { x: ..., z: ... }` |
| 3 | Cancel `ChunkUnloadEvent` via plugin and repeat | Chunk remains loaded; no unload log for that chunk |
| 4 | Shut down server | `WorldUnloadEvent: world=overworld`, then nether/end |
| 5 | Cancel `WorldUnloadEvent` via plugin and shut down | Cancelled world is skipped during shutdown (dangerous; test carefully) |

### Automated test

Add a Rust test in `pumpkin/src/plugin/api/events/phase7_tests.rs` that:

1. Creates a `PluginManager` and a mock `World`.
2. Fires `WorldLoadEvent`, `ChunkUnloadEvent`, and `WorldUnloadEvent`.
3. Verifies each handler receives the event.
4. Verifies cancellation works for cancellable events.

---

## Sample `output.log`

```text
[2026-07-09T12:00:00Z INFO  phase7_test_plugin] WorldLoadEvent: world=overworld
[2026-07-09T12:00:00Z INFO  phase7_test_plugin] WorldLoadEvent: world=the_nether
[2026-07-09T12:00:00Z INFO  phase7_test_plugin] WorldLoadEvent: world=the_end
[2026-07-09T12:05:12Z INFO  phase7_test_plugin] ChunkUnloadEvent: world=overworld, pos=ChunkPos { x: 10, z: -20 }
[2026-07-09T12:05:15Z INFO  phase7_test_plugin] ChunkUnloadEvent: world=overworld, pos=ChunkPos { x: 11, z: -20 }
[2026-07-09T12:10:00Z INFO  phase7_test_plugin] WorldUnloadEvent: world=overworld
[2026-07-09T12:10:00Z INFO  phase7_test_plugin] WorldUnloadEvent: world=the_nether
[2026-07-09T12:10:00Z INFO  phase7_test_plugin] WorldUnloadEvent: world=the_end
```

---

## Phase 7 Completion Criteria

Phase 7 is complete when:

1. `WorldLoadEvent` is defined, registered, and fires for every world after startup loading completes.
2. `ChunkUnloadEvent` is defined, registered, and fires when chunks are removed from memory.
3. `WorldUnloadEvent` is defined, registered, and fires before worlds are shut down.
4. All three events carry the correct world/chunk references.
5. `ChunkUnloadEvent` and `WorldUnloadEvent` support cancellation and the cancellation prevents the unload.
6. The automated smoke test passes.
7. `cargo clippy --all-targets --all-features` and `cargo test -p pumpkin` pass.

---

## References

- Parent plan: [`plan-api.md`](./plan-api.md)
- Phase 6 detail: [`plan-api-phase6.md`](./plan-api-phase6.md) (to be created)
- Pumpkin world definitions: `pumpkin/src/plugin/api/events/world/`
- Pumpkin world loading: `pumpkin/src/server/mod.rs:305-347`
- Pumpkin chunk system: `pumpkin-world/src/chunk_system/schedule.rs`
- Pumpkin chunk listener: `pumpkin-world/src/chunk_system/chunk_listener.rs`
- Pumpkin `World::shutdown`: `pumpkin/src/world/mod.rs:366-379`

---

*Document generated for Phase 7 of the Pumpkin / mcMMO event parity effort.*
