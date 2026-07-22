# Item Entity Lifecycle and Concurrency Plan

## Goal

Make entity removal, chunk unloading, dropped-item merging, and pickup safe under Pumpkin's concurrent tick model without changing normal gameplay semantics or weakening plugin event delivery.

## Invariants

1. An entity has one observable lifecycle: active, removal in progress, or removed with a reason.
2. At most one removal attempt may fire events and mutate world indexes at a time.
3. Cancelling a removal returns the entity to an active state.
4. Chunk-unload saves cannot overlap the world's entity tick phase.
5. Item stack mutation is revalidated after every plugin-event await.
6. Merge participants must still be active after both mutation locks are acquired.
7. Async entity workflows treat registry disappearance as an expected race, not a panic.

## Phase 1 — Entity removal lifecycle

- Add an atomic removal-in-progress claim while retaining the public `removed` and `removal_reason` fields for plugin compatibility.
- Make `Entity::is_removed` account for both legacy fields.
- Add reason-aware removal and an explicit removal outcome.
- Make `World::remove_entity` idempotent: only the winning attempt fires `EntityRemoveEvent`, updates indexes, and broadcasts removal.
- Restore the active state when `EntityRemoveEvent` is cancelled.
- Record `RemovalReason::Killed` for normal living-entity death and reset all lifecycle state on player respawn.
- Add focused lifecycle tests.

## Phase 2 — Chunk unload barrier

- Add a per-world gate around the entity tick phase.
- Require chunk entity unload/save to acquire that gate, so stale tick tasks cannot mutate an entity while it is serialized.
- Fire `ChunkEntityUnloadEvent` while the entity is still discoverable.
- Claim the entity lifecycle during the unload event, restore it on cancellation, and finish it with `UnloadedToChunk` before saving and detaching.
- Preserve existing block-entity cleanup and chunk-index behavior.

## Phase 3 — Item mutation transactions

- Add a per-item mutation lock and mutation epoch.
- Acquire two item locks in entity-ID order for merges, then revalidate lifecycle, stack contents, age, and pickup eligibility under the locks.
- Reject empty or removed merge participants.
- Snapshot pickup for the plugin event, then acquire the mutation lock and reject the pickup if the entity or mutation epoch changed.
- Compute pickup statistics from the stack immediately before and after insertion with defensive arithmetic.
- Serialize item damage with merge/pickup mutation and stop tick work immediately after removal.
- Discard empty item entities at tick entry, matching vanilla's defensive behavior.
- Add merge and pickup-accounting regression tests.

## Phase 4 — Registry-race hardening

- Replace remaining registry `expect` calls in asynchronous entity/projectile workflows with retained references or defensive early returns.
- Cover player disconnect during pickup and projectile removal during impact.
- Add regression tests for the defensive lookup helpers where direct integration setup is impractical.
- Run formatting, Pumpkin library tests, and a normal Pumpkin compile.

Each phase is validated and committed separately. The unrelated cross-edition test-matrix edit remains outside every commit.
