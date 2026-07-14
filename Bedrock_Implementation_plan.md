# Bedrock implementation discrepancy plan

## Scope and comparison method

This is a code comparison of Pumpkin's direct Bedrock server implementation with
the state-translation patterns used by `../Geyser` as inspected on 2026-07-13.
Geyser is a Java-to-Bedrock proxy, so its Java packet translators are not a
feature checklist for Pumpkin. A discrepancy below is included only where
Pumpkin has a Bedrock packet/state responsibility that is missing, incomplete,
or likely to become inconsistent during normal play.

Pumpkin already has substantial Bedrock coverage: RakNet ordering and batched
packet handling, login/resource-pack negotiation, player and actor spawn/remove,
movement and metadata broadcasts, inventory transactions/responses, chunks,
basic respawn handling, commands, scoreboards, boss bars, time, health,
abilities, and dimension-change packets. The priority is to make the server's
authoritative state and the Bedrock client's state converge reliably.

## Reference model

Geyser's important pattern is **state ownership plus targeted packet updates**:

- It keeps per-session state for world, entities, inventory, movement, time,
  gamerules, and chunk visibility.
- A Java state packet updates that cache and emits the corresponding Bedrock
  packet only when necessary.
- It uses cache removal/empty chunks for chunk unloads, sends actual typed
  gamerule values, updates attributes as a group, and resets dependent state on
  respawn or dimension changes.

Pumpkin is authoritative rather than a proxy, but needs the same observable
contract: update every client-visible state field when it changes, initialise
that state before play, and invalidate client state that is no longer valid.

## Discrepancies

### P0 — chunk dimension and visibility state are incorrect outside the overworld

Evidence:

- `pumpkin/src/net/bedrock/mod.rs` constructs every `CLevelChunk` with
  `dimension: 0`.
- `pumpkin/src/world/chunker.rs` only emits `CUnloadChunk` for Java clients.
  Bedrock clients receive no empty/replacement chunk when a watched chunk leaves
  view.
- Dimension change does send `CChangeDimension` in
  `pumpkin/src/entity/player.rs`, so its dimension value can disagree with the
  subsequently streamed `LevelChunk` packets.
- Geyser's `JavaForgetLevelChunkTranslator` removes its cache entry and sends an
  empty Bedrock chunk; it also tracks the active Bedrock dimension in session
  state.

Impact: Nether/End terrain may be interpreted in the wrong dimension, and
chunks that leave view can remain stale client-side. This is especially risky
when changing dimensions, changing view distance, or returning to a previously
visited area.

Plan:

1. Store the active Bedrock dimension on `BedrockClient` (or pass it from the
   player's current world when serialising a chunk); map all supported Pumpkin
   dimensions in one shared helper.
2. Use that value in `CLevelChunk`, never a literal `0`.
3. Add a Bedrock unload/empty-chunk packet path and call it for
   `unloading_chunks`. Keep the server's watched-chunk bookkeeping independent
   of delivery, but make client invalidation ordered before replacement chunks.
4. Add regression tests for overworld -> nether -> overworld and for shrinking
   render distance. Inspect captured packet dimension fields and assert that an
   unload is emitted exactly once per removed chunk.

### P0 — gamerules and the client clock are not synchronised

Evidence:

- `pumpkin-protocol/src/bedrock/client/gamerules_changed.rs` defines `GameRules`
  as only `list_size`; it cannot encode a name, type, or value.
- `pumpkin/src/world/mod.rs` sends an empty gamerule list in `StartGame`, and
  `CGamerulesChanged` has no runtime sender.
- `pumpkin/src/world/time.rs` always sends `CSetTime`, while its own comment
  notes that it does not tell Bedrock whether time is frozen.
- Geyser sends concrete `GameRuleData` entries at setup and emits a
  `dodaylightcycle` update when the authoritative clock rate changes.

Impact: client-side presentation/behaviour can drift from server rules. The
most visible case is a stopped day-night cycle that the Bedrock client continues
to advance. Respawn-screen, coordinates, inventory-death, and other
client-controlled behaviours have no authoritative Bedrock rule state.

Plan:

1. Replace the placeholder `GameRules` codec with a typed list of Bedrock rule
   entries (boolean, integer, and float as required by the protocol version).
2. Define an explicit Pumpkin-to-Bedrock rule mapping. Start with
   `dodaylightcycle`, `doimmediaterespawn`, `keepinventory`,
   `naturalregeneration`, `mobgriefing`, `showcoordinates`, and `spawnradius`.
   Document intentional compatibility overrides separately from world rules.
3. Send the initial list in `StartGame` and send a minimal
   `CGamerulesChanged` delta whenever a mapped rule changes.
4. Drive `dodaylightcycle` from `advance_time` and make `CSetTime`/gamerule
   updates atomic from the player's perspective.
5. Add codec golden tests and an integration test that toggles `advance_time`
   and observes a typed gamerule update before/with the next time update.

### P1 — initial and runtime player attributes can diverge from authoritative state

Evidence:

- The initial `CUpdateAttributes` in `pumpkin/src/world/mod.rs` hard-codes
  movement, air, health, and hunger values (for example 20 health and hunger),
  instead of reading the spawning player's attribute and hunger state.
- `Player::send_health` in `pumpkin/src/entity/player.rs` only sends the
  Bedrock health attribute and `CSetHealth`; it does not update hunger or
  saturation attributes after food changes.
- `tick_health` detects food/saturation changes and calls `send_health`, so the
  omission affects routine gameplay rather than only initial spawn.
- Geyser's `JavaSetHealthTranslator` sends one `UpdateAttributesPacket`
  containing health, hunger, and saturation, and updates its cached attributes
  together.

Impact: a joining player with non-default health/food/max-health can be shown
incorrect values, and the Bedrock hunger bar/saturation-derived behaviour can
remain stale after eating, starvation, or plugin changes.

Plan:

1. Create one `Player::bedrock_attributes()` builder that derives health,
   max-health, hunger, saturation, movement, air, and other supported values
   from live server state.
2. Use it for initial spawn, health/food updates, respawn, and any max-health
   or modifier change. Do not retain a separate hard-coded initial list.
3. Coalesce same-tick changes into a single `CUpdateAttributes` with the
   current player tick (rather than a constant zero where protocol semantics
   require a tick).
4. Verify survival join, eating, damage/heal, max-health modifier, drowning,
   and respawn on an actual Bedrock client.

### P1 — movement reconciliation packet exists but no reconciliation policy uses it

Evidence:

- `CCorrectPlayerMove` is implemented in
  `pumpkin-protocol/src/bedrock/client/correct_player_move.rs` but has no
  runtime sender.
- `handle_player_auth_input` in `pumpkin/src/net/bedrock/play.rs` accepts the
  absolute predicted position as authoritative and broadcasts it; it does not
  compare it with a server simulation/collision result or correlate prediction
  ticks.
- Geyser has a dedicated Bedrock input translator and movement/collision state
  around its session, rather than treating every predicted position as final.

Impact: invalid or desynchronised movement has no Bedrock-native correction
path. Later server-side collision, knockback, vehicles, or anti-cheat work will
produce visible rubber-banding or allow client state to lead server state.

Plan:

1. Make the server movement result authoritative: validate the proposed motion,
   collision, world/dimension, and relevant input tick before committing it.
2. Track the last accepted Bedrock input tick/position per player.
3. Send `CCorrectPlayerMove` for rejected or adjusted predictions; reserve
   `MovePlayer::MODE_TELEPORT` for explicit teleports.
4. Exercise normal walking, collision against a wall, knockback, fall landing,
   and a deliberately stale/out-of-range input packet.

### P1 — client-cache capability is parsed but deliberately discarded

Evidence:

- `SClientCacheStatus` is decoded, but the play handler in
  `pumpkin/src/net/bedrock/mod.rs` contains only `// TODO`.
- `CLevelChunk` always uses `cache_enabled: false`.

Impact: this is not a correctness failure while caching remains disabled, but
it is a clear throughput discrepancy from a mature Bedrock implementation. The
code must not enable cache-backed chunks until the capability and blob lifecycle
are tracked correctly.

Plan:

1. Keep cache disabled as the safe current behaviour, but store the negotiated
   capability explicitly.
2. Before enabling it, implement blob hash tracking, cache miss responses,
   invalidation on chunk/block changes, and tests for reconnect/dimension
   change. Do not merely flip `cache_enabled`.

### P2 — initial state is incomplete for persistent world UI and inventories

Evidence:

- Initial Bedrock inventory sync in `World::send_world_info` sends only
  container 0 (main inventory). Armour and offhand containers are handled by
  later inventory update paths but are not included in this initial snapshot.
- `Scoreboard` sends Bedrock objective/score packets when state mutates, but
  the joining-player setup path does not replay existing objectives, scores, or
  teams. A player who joins after the scoreboard is populated can therefore
  miss its current state.
- Geyser maintains per-session inventory/scoreboard caches and supplies state
  as the session becomes ready.

Impact: newly joined Bedrock players can see stale/empty equipment or miss a
server's existing scoreboard until a later mutation happens.

Plan:

1. Add a `send_initial_bedrock_inventory_state` helper that sends main,
   armour, offhand, selected hotbar slot, and cursor state in protocol-valid
   order.
2. Add `Scoreboard::send_snapshot_to(player)` and call it after the Bedrock
   player is ready to receive UI state. Use stable, allocated Bedrock scoreboard
   IDs instead of the current string-pointer-derived IDs.
3. Test joining after equipment and a sidebar score already exist; then change
   and remove a score to verify the same IDs are used.

### P2 — registry/bootstrap completeness should be validated, not assumed

Evidence:

- Pumpkin sends `StartGame`, an item registry, creative content, crafting data,
  and chunk data, but `StartGame` currently has zero block properties and an
  empty registry compound (`block_properties_size: 0`, `compound_len: 0`).
- Geyser additionally sends Bedrock biome definitions, available entity
  identifiers, item components, and other version-specific registry
  definitions before normal play state.

Impact: vanilla content may work through static client knowledge, but biome
rendering, entities, component-based items, custom data, or a newer Bedrock
protocol can fail without an obvious server-side error.

Plan:

1. Produce a version-pinned bootstrap packet trace from a supported vanilla
   Bedrock server and compare the required definitions with Pumpkin's trace.
2. Add only definitions needed by Pumpkin's negotiated protocol version; do not
   copy Geyser's proxy-specific registries blindly.
3. Add packet decode/golden tests and a smoke test covering biome changes,
   non-player actors, and component-based inventory items.

## Three-phase implementation plan

### Phase 1 — world-state correctness

**Goal:** make the world the client sees unambiguously match the world the
server has selected.

Work:

1. Replace the literal `dimension: 0` on `CLevelChunk` with the active
   Bedrock dimension and introduce one shared Pumpkin-to-Bedrock dimension
   mapper.
2. Send a Bedrock empty/unload chunk for every `unloading_chunks` entry, in
   the correct order relative to newly loaded/replacement chunks.
3. Implement typed Bedrock gamerules and send their initial state in
   `StartGame`.
4. Send gamerule deltas for `dodaylightcycle` and the initially supported
   gameplay rules; coordinate time and daylight-cycle updates.

Exit criteria:

- Overworld -> nether/end -> overworld sends chunks with the matching
  dimension field and leaves no stale chunks after a view-distance reduction.
- A frozen clock stays frozen on a Bedrock client; a rule change produces a
  decodable typed gamerule packet.
- Unit/golden tests cover gamerule encoding and chunk dimension/unload
  packets, plus a Bedrock-client smoke test covers the two scenarios above.

### Phase 2 — player-state convergence

**Goal:** ensure spawn, routine updates, and recovery all report the same
player state to Bedrock.

**Current implementation note (2026-07-14):** the join, inventory, attribute,
air, death-screen, and respawn paths are stable at the
`bedrock-phase-2-very-much-working` checkpoint. The attempted automatic
`CCorrectPlayerMove` policy was rolled back because it could correct an
otherwise valid client immediately after initialisation. Phase 2 therefore
keeps the known-good client-predicted movement path; authoritative movement
reconciliation is staged in Phase 3 instead of being treated as complete.

Work:

1. Replace hard-coded initial attributes with a shared live player-attribute
   builder.
2. Emit health, max-health, hunger, saturation, air, and supported movement
   attribute deltas together when their authoritative values change.
3. Add initial armour/offhand, selected-slot, and cursor inventory sync.
4. Preserve the stable `PlayerAuthInput` path while retaining explicit reset
   points at join, respawn, teleport, and dimension boundaries. Do not emit
   automatic movement corrections until Phase 3's staged validation proves
   their wire semantics and timing against a real client.
5. Recheck respawn and dimension-transition ordering against this consolidated
   player snapshot path.
6. Generate a player's death message once, select a valid translation key for
   each edition, and reuse the result for chat broadcast, the Bedrock
   `DeathInfo` packet, and a formatted server-console log entry.

Exit criteria:

- Join, eating, damage/healing, drowning, a max-health change, death/respawn,
  and a dimension transition display the correct Bedrock HUD and inventory.
- Death produces correctly localised chat and death-screen text, writes the
  same event to the server console, and never exposes a raw translation key.
- The stable movement baseline handles normal walking, collision, and
  knockback without unsolicited corrections or persistent client drift.
- Packet tests assert the field values and a real Bedrock smoke test exercises
  the full state sequence.

### Phase 3 — session completeness and protocol hardening

**Goal:** make every Bedrock session boundary use an ordered, repeatable state
replay before adding protocol features that improve performance and
future-version safety.

**Implementation baseline:** protocol ownership and wire decisions for the
current supported client are recorded in
`Bedrock_Protocol_1001_compatibility.md`. Phase 3 keeps movement correction and
chunk caching disabled by default; their observation/capability state is
implemented without opting the client into either unsafe path.

**Implementation status (2026-07-14):** the lifecycle state machine,
generation-claimed recovery replay, cursor/selected-slot snapshot, stable
scoreboard identities and team decoration, cache-capability tracking, movement
observation, compatibility matrix, and packet regressions are implemented.
The real-client smoke sequence in the verification gate remains the release
check before enabling movement correction or chunk caching.

Work:

1. Define explicit Bedrock session states for initialising, playing, changing
   dimensions, dead, respawning, and disconnected. Document which packet
   groups may be emitted in each state and gate asynchronous updates against
   the current state.
2. Make the player snapshot replay ordered and idempotent across fresh join,
   reconnect, dimension change, and respawn. Include actor metadata,
   attributes, abilities, inventory/equipment, selected slot, cursor, and
   persistent UI state without sending duplicate or premature packets.
3. Add `Scoreboard::send_snapshot_to(player)` for objectives, scores, and
   teams, replacing pointer-derived scoreboard IDs with stable allocations.
   Send it only after the session is ready for persistent UI packets.
4. Reintroduce movement reconciliation incrementally. First record input ticks
   and server results without changing client movement; then reject demonstrably
   stale or impossible inputs; finally enable `CCorrectPlayerMove` only after
   captured packet comparison and real-client tests prove its fields and
   post-initialisation timing. Keep the correction stage disabled by default
   until that gate passes.
5. Capture a version-pinned vanilla Bedrock bootstrap trace before adding more
   bootstrap data. Compare packet ordering, runtime IDs, container wire IDs,
   metadata-versus-attribute ownership, and required registries. Close only the
   required gaps (biomes, entity identifiers, item components, or other
   negotiated-version definitions).
6. Record `SClientCacheStatus` capability but keep chunk caching disabled.
   Treat blob tracking, misses, invalidation, reconnect, and dimension changes
   as a separate enablement gate; do not enable caching as part of capability
   recording.
7. Expand packet capture/golden coverage and maintain a compatibility matrix
   for each supported Bedrock version. Include payload decoding, lifecycle
   ordering, compatibility gamerule overrides, and stable identifier reuse in
   addition to individual packet codecs.

Exit criteria:

- Fresh join, reconnect, dimension change, delayed death-screen respawn, and
  normal respawn all produce the same authoritative snapshot without a crash,
  stale HUD field, duplicate actor, or invalid lifecycle packet.
- Inventory remains interactive after every snapshot replay, and air, health,
  hunger, saturation, metadata, abilities, equipment, and selected-slot state
  survive respawn and dimension transitions.
- A player joining an already populated world receives a complete scoreboard
  and persistent UI snapshot with stable IDs before interaction.
- Bootstrap packets and representative entity/biome/component-item traffic are
  validated against the selected Bedrock protocol version, including ordering
  and identifier semantics rather than only successful encoding.
- Cache negotiation records the client capability and always has an explicit
  cache-disabled fallback. Cache support is not enabled until its separate
  reconnect, invalidation, and dimension-change gate passes.
- Movement observation and validation do not alter known-good play. Movement
  correction remains disabled unless stale input, collision, knockback, and
  post-initialisation real-client tests all pass without unsolicited
  corrections.

## Verification gate

For each completed item, capture both Pumpkin's decoded outgoing packet stream
and a Bedrock-client smoke test. Cover at least: fresh join, reconnect,
inventory/armour changes, damage/eating, frozen and advancing time, scoreboard
created before join, render-distance reduction, overworld/nether/end round
trip, drowning and air recovery, a delayed death-screen respawn, inventory
interaction after respawn, correctly localised death broadcast/console output,
and collision/knockback. Assert that no packet group is emitted outside its
allowed lifecycle state and that replaying a snapshot does not duplicate
actors, scoreboard IDs, or inventory state. Keep golden codec tests close to
the packet implementations so protocol field, wire-ID, and ordering changes
cannot silently regress state synchronisation.
