# Pumpkin Block-Break Context and Post-Break Event Plan

## Goal

Give native and WASM plugins enough reliable context to react after a block has actually been broken. This supports Cabbage's future ore-revelation mechanic without putting that mechanic in Pumpkin.

This change has two deliverables:

1. Preserve the block face a player mined and expose it on `BlockBreakEvent`.
2. Fire a non-cancellable `BlockBrokenEvent` after a successful block replacement and its normal drop processing.

Persistent block provenance and bulk block mutation are deliberately out of scope. They should be separate proposals because they affect chunk persistence and world mutation APIs more broadly.

## Required semantics

- `face` is the outward-facing side of the block selected by the client.
- A plugin that wants the block deeper into the wall uses `face.opposite()`.
- Player-originated Java and Bedrock breaks should provide `Some(face)` when the packet contains a valid face.
- Programmatic, environmental, command, and internal `World::break_block` calls continue to work and report `None` unless their caller explicitly supplies a face.
- Invalid packet face values must not panic. Reject the player action using the edition's existing invalid-action behavior, or safely record `None` if rejection would be inconsistent with current behavior.
- `BlockBreakEvent` remains cancellable and continues to fire before world mutation.
- `BlockBrokenEvent` is not cancellable and fires exactly once only when `BlockBreakEvent` was not cancelled and the block was replaced.
- No post event fires for air, cancelled breaks, or a failed/no-op replacement.
- The post event observes the block position in its replacement state (`air` or the appropriate water state for a waterlogged block).
- Existing `World::break_block(position, cause, flags)` callers remain source-compatible.

## Event API

### Extend `BlockBreakEvent`

Update `pumpkin/src/plugin/api/events/block/block_break.rs`:

```rust
pub struct BlockBreakEvent {
    pub player: Option<Arc<Player>>,
    pub block: &'static Block,
    pub block_position: BlockPos,
    pub face: Option<BlockDirection>,
    pub exp: u32,
    pub drop: bool,
    // generated `cancelled` field remains unchanged
}
```

Add `face` to `BlockBreakEvent::new`. Update every constructor, native test, WASM conversion, and generated binding affected by the signature.

### Add `BlockBrokenEvent`

Create `pumpkin/src/plugin/api/events/block/block_broken.rs` and export it from the block event module.

Recommended native shape:

```rust
#[derive(Event, Clone)]
pub struct BlockBrokenEvent {
    pub world: Arc<World>,
    pub player: Option<Arc<Player>>,
    pub block: &'static Block,
    pub block_state_id: BlockStateId,
    pub replacement_state_id: BlockStateId,
    pub block_position: BlockPos,
    pub face: Option<BlockDirection>,
    pub dropped_items: bool,
}
```

Field meanings:

- `block` and `block_state_id` describe the block before removal.
- `replacement_state_id` describes the actual state installed by `break_block`; it may be water rather than air.
- `world` makes the event useful even when `player` is `None`.
- `dropped_items` reports the final drop decision after cancellable `BlockBreakEvent` handlers have modified `drop`. It does not mean a nonempty loot list was produced.
- `face` uses the same outward-face convention as the pre-event.

Implement `BlockEvent` for the new event. Do not derive or implement cancellation.

## Preserve source compatibility in `World`

Keep the existing public method:

```rust
pub async fn break_block(
    self: &Arc<Self>,
    position: &BlockPos,
    cause: Option<Arc<Player>>,
    flags: BlockFlags,
) -> Option<BlockStateId>
```

Make it delegate to a new face-aware entry point:

```rust
pub async fn break_block_with_face(
    self: &Arc<Self>,
    position: &BlockPos,
    cause: Option<Arc<Player>>,
    face: Option<BlockDirection>,
    flags: BlockFlags,
) -> Option<BlockStateId>
```

This avoids changing the many environmental, command, redstone, plant, fluid, and block-behavior call sites. Only the Java and Bedrock player-action paths need to call `break_block_with_face` initially.

If Pumpkin prefers a more extensible API, use a `BlockBreakContext` instead, but retain the existing wrapper. Do not require every existing caller to manufacture a context as part of this change.

## Java face propagation

Relevant files:

- `pumpkin-protocol/src/java/server/play/player_action.rs`
- `pumpkin/src/net/java/play.rs`
- `pumpkin/src/entity/player.rs`

The Java `SPlayerAction` packet already contains `face: u8`. Convert it with `BlockDirection::try_from(i32::from(player_action.face))`.

Handle both paths:

1. Instant/creative break: pass the validated packet face directly to `break_block_with_face`.
2. Timed break: retain the face captured by `StartedDigging` and use it when the break completes.

Store position and face together so stale state cannot associate a face with a different block. A small player field is preferable to independent `mining_pos` and `mining_face` fields:

```rust
pub struct MiningTarget {
    pub position: BlockPos,
    pub face: Option<BlockDirection>,
}
```

Replace or wrap the current `mining_pos` with `Mutex<Option<MiningTarget>>`. Clear the target on cancellation, completion, invalidation, teleport/world change, and when the target block becomes air. If changing the existing field creates excessive unrelated churn, add a second field temporarily but always verify the stored position before using its face.

## Bedrock face propagation

Relevant files:

- `pumpkin-protocol/src/bedrock/server/player_action.rs`
- `pumpkin/src/net/bedrock/play.rs`
- `pumpkin/src/entity/player.rs`

Bedrock `SPlayerAction` already contains `face: VarInt`. Validate and convert it to `BlockDirection` for `StartBreak`, `CreativePlayerDestroyBlock`, `ContinueDestroyBlock`, `PredictDestroyBlock`, and `StopBreak` as appropriate.

Use the same `MiningTarget` storage as Java. For an instant break, pass the current validated face directly. For a timed break, prefer the face stored with the matching start position so later packets cannot accidentally change the original mined face.

Java and Bedrock must produce identical native event semantics.

## Fire the events in `World::break_block_with_face`

Update `pumpkin/src/world/mod.rs`:

1. Read and retain `broken_block` and `broken_block_state`.
2. Return `None` immediately for air, as today.
3. Construct `BlockBreakEvent` with `face`.
4. Fire and await the cancellable event.
5. Return `None` if cancelled.
6. Apply the event's final drop setting to the flags.
7. Calculate and install the replacement state.
8. Perform the existing screen close, particles, loot, item-drop, and experience behavior.
9. Construct and fire `BlockBrokenEvent` using the retained old state and actual replacement state.
10. Return the existing result without changing its public meaning in this patch.

Fire the post event after drop processing so "broken" means the existing break operation has completed. The old block and state are already captured, so a post-event plugin changing nearby blocks cannot affect the original loot calculation.

Before firing, defensively confirm that the state returned by `set_block_state` is the expected old state. If another path can make this a no-op or replace a different state, document and test the chosen behavior rather than emitting a misleading post event.

## Native plugin registration

Update imports and event exports so a plugin can use:

```rust
use pumpkin::plugin::api::events::block::block_broken::BlockBrokenEvent;
```

The event should use the standard `EventHandler<BlockBrokenEvent>` path and be registerable as a normal non-blocking event. A handler may still perform awaited world operations; "non-blocking" here means it cannot mutate/cancel the event payload.

Check event priority behavior but do not redesign Pumpkin's dispatcher in this patch.

## WASM/WIT parity

Do not leave native and WASM event surfaces inconsistent.

Update at least:

- `pumpkin-plugin-wit/v0.1/event.wit`
- `pumpkin/src/plugin/loader/wasm/wasm_host/wit/v0_1/events/block.rs`
- `pumpkin/src/plugin/loader/wasm/wasm_host/wit/v0_1/context.rs`
- `pumpkin-plugin-api/src/events/block/mod.rs`
- a new `pumpkin-plugin-api/src/events/block/block_broken.rs`
- generated/re-exported WIT bindings required by Pumpkin's normal codegen workflow

Import/reuse `block-direction` from `world.wit` rather than encoding faces as arbitrary integers. Extend `block-break-event-data` with `face: option<block-direction>` and introduce a `block-broken-event-data` record matching the native fields.

Add `BlockBrokenEvent` to `EventType`, the event variant, registration routing, host conversion, and guest API. Because WIT v0.1 is a public compatibility surface, confirm whether adding a record field is accepted by the project's versioning policy. If not, introduce a v0.2 event shape while keeping the old v0.1 pre-event representation; native support must not be blocked on that decision.

## Tests

### Event construction and conversion

- `BlockBreakEvent::new` retains `Some(face)` and `None`.
- `BlockBrokenEvent` exposes old and replacement states correctly.
- Native-to-WASM-to-native conversion preserves all fields.
- All six `BlockDirection` values round-trip.

### World break behavior

- Air returns `None` and fires neither event.
- A cancelled pre-event leaves the block unchanged and fires no post event.
- A successful break fires one pre-event followed by one post-event.
- The post event observes air at an ordinary block position.
- Breaking a waterlogged block reports the actual water replacement state.
- A handler changing `drop` is reflected in `BlockBrokenEvent::dropped_items`.
- A programmatic `break_block` call reports `face == None`.
- Event fields preserve the exact old `BlockStateId`, including non-default states.

### Java behavior

- Creative/instant breaking forwards the packet face.
- Timed breaking forwards the face saved at start.
- Cancelling or switching targets clears/replaces the saved mining target.
- An invalid face never panics and never produces a false valid face.

### Bedrock behavior

- Creative/instant and timed breaks match Java semantics.
- Start and stop packets for different positions cannot mix position and face.
- Invalid `VarInt` face values are handled safely.

### Regression coverage

- Existing internal `break_block` callers compile unchanged.
- Item drops, XP drops, tool damage, statistics, particles, and block updates remain unchanged.
- Cancelling `BlockBreakEvent` still prevents all break consequences.

## Verification commands

From the Pumpkin repository root, use the repository's documented formatting and lint commands. At minimum:

```text
cargo fmt --all -- --check
cargo check -p pumpkin
cargo test -p pumpkin block_break
cargo check -p pumpkin-plugin-api
```

Also run the WIT/codegen verification command used by Pumpkin after changing `pumpkin-plugin-wit`. If no focused command exists, run the relevant workspace checks and ensure generated bindings have no diff after regeneration.

Finally, build Cabbage against the updated local Pumpkin checkout to confirm the public native API is usable without regressions.

## Suggested implementation order

1. Add the native `face` field and the source-compatible `break_block_with_face` wrapper.
2. Add shared `MiningTarget` state and wire Java.
3. Wire Bedrock to the same semantics.
4. Add and fire native `BlockBrokenEvent`.
5. Add focused native tests.
6. Update WIT, host conversions, guest API, and generated bindings.
7. Run Pumpkin checks and then compile Cabbage against Pumpkin.

## Acceptance criteria

- Existing Pumpkin `break_block` callers require no edits solely because of the new face parameter.
- Native Java and Bedrock player breaks expose the correct outward face.
- Cancelled or unsuccessful breaks never emit `BlockBrokenEvent`.
- Every successful break emits exactly one immutable post event with the correct world, player, position, old state, replacement state, drop decision, and optional face.
- Native and WASM plugin APIs either have parity or follow an explicitly documented WIT versioning decision.
- The Pumpkin workspace checks pass, and Cabbage can compile against the updated API.
