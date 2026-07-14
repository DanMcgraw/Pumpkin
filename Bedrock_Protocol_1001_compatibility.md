# Bedrock protocol 1001 compatibility matrix

## Baseline

- Pumpkin protocol: `1001` (`BedrockMinecraftVersion::V_1_26_30`).
- Real-client baseline: Windows Bedrock 1.26.33 joining, interacting with
  inventory, drowning, remaining on the death screen, and respawning against
  the `bedrock-phase-2-very-much-working` checkpoint.
- Translation reference: the local `../Geyser` checkout, especially its player
  inventory, health, combat-kill, respawn, and initialized-session translators.
- Safety rule: a packet is not added merely because Geyser emits it. Pumpkin
  adds a definition or replay packet only when its direct-server state owns the
  corresponding value and protocol 1001 requires it.

## Session lifecycle

| State | Accepted outbound packet groups | Completion signal |
| --- | --- | --- |
| Initializing | StartGame/bootstrap, registries, initial inventory and actor state | `SetLocalPlayerAsInitialized` with the local runtime ID |
| Playing | Gameplay updates and persistent UI | World change, lethal health, or disconnect |
| Changing dimension | ChangeDimension, teleport, chunks and world transition state | `PlayerAction::DimensionChangeAck` |
| Dead | Health zero and death information | `Respawn::ClientReadyToSpawn` |
| Respawning | Respawn, teleport, chunks and world transition state | final `PlayerAction::Respawn` |
| Disconnected | none | terminal |

Dimension and respawn acknowledgements open one recovery generation. Actor
metadata, movement/vital attributes, abilities, inventory, equipment, cursor,
selected slot, and scoreboard state are replayed at most once for that
generation.

## Wire and state ownership

| Area | Protocol 1001 behavior | Validation |
| --- | --- | --- |
| Runtime IDs | StartGame uses the player's runtime ID. Server-ready Respawn uses Bedrock's local-player sentinel `0`. | Respawn codec tests and real-client respawn |
| Air | `AIR_SUPPLY`, `AIR_SUPPLY_MAX`, and `BREATHING` actor metadata; no synthetic `minecraft:air` attribute | Player metadata tests and drowning smoke test |
| Vitals | Health, hunger, and saturation are one authoritative attribute group; natural regeneration is disabled client-side | Bedrock attribute tests |
| Inventory content IDs | main `0`, armor `120`, offhand `119` | Initial inventory replay and inventory interaction smoke test |
| Cursor snapshot | `InventorySlot`, UI window `124`, slot `0`; the optional full-container name is omitted like Geyser | InventorySlot golden-prefix test |
| Selected slot | `PlayerHotbar`, inventory container `0` | PlayerHotbar golden test |
| Stack request names | HotBar `28`, Inventory `29`, Offhand `34`, Cursor `59`, Dynamic `63` | Captured request fixture and container-name tests |
| Dropped items | `AddItemActor` uses the mapped Bedrock runtime item and omits inventory network-stack IDs, matching Geyser | Dropped-apple wire test and real-client item-entity smoke test |
| Score identities | Positive monotonically allocated IDs keyed by objective and entry; IDs survive score updates and snapshot replay | Scoreboard allocation test |
| Gamerules | Typed values with explicit server-authoritative overrides for regeneration, inventory retention, and spawn radius | Gamerule codec/state tests |
| Client cache | Capability is recorded; `LevelChunk.cache_enabled` remains `false` | Cache-status decode test and source assertion |
| Movement | Input ticks and resulting positions are observed. Stale ticks are logged but are not rejected or corrected. | Input-observation test and real-client movement baseline |

## Bootstrap definitions

Pumpkin currently sends StartGame, the item registry with component data,
creative content, crafting data, and chunk data. Protocol 1001 vanilla clients
in the real-client baseline do not require additional block properties, biome
definitions, or available-entity-identifier packets to enter and play in a
vanilla Pumpkin world. Those registries remain explicit compatibility gaps for
custom content or a future protocol revision; they must be backed by a
version-pinned packet trace and golden codec before being enabled.

Chunk blob caching is likewise deferred. Recording `ClientCacheStatus` does not
authorize cache-backed chunks: blob hash tracking, miss responses, mutation
invalidation, reconnect, and dimension-transition coverage are all required
before changing `cache_enabled`.

## Required smoke sequence

For each supported Bedrock version, exercise fresh join, reconnect, cursor and
hotbar inventory movement, eating and damage, drowning and air recovery,
delayed death-screen respawn, scoreboard state created before join, and an
overworld/nether/end round trip. Packet review must confirm that no gameplay or
persistent-UI packet crosses an invalid lifecycle boundary and that recovery
state is not duplicated.
