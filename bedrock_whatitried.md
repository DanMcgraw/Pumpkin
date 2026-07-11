# Bedrock join crash – what has been tried

Phone client reaches the "Let's go!" loading screen, then Minecraft crashes.
Server log (`../PumpkinRunner/logs/latest.log`) shows:

```text
[ERROR] UDP socket error: An existing connection was forcibly closed by the remote host. (os error 10054)
```

immediately after the player joins and the render-distance is negotiated, followed by
endless RakNet resends of the queued packets.

Protocol target: Bedrock 1.26.30 / 1.26.33, protocol `1001`.

---

## 1. LevelChunk single-value palette fix

**File:** `pumpkin-protocol/src/bedrock/client/level_chunk.rs`

Removed a bogus `VarInt(1)` count that was being written before single-value
block/biome palettes. In Bedrock a single-value storage is encoded as
`(bits_per_entry << 1 | 1)` (value `1`) followed by the single runtime ID,
not by a palette-count prefix.

**Result:** still crashed.

---

## 2. Biome volume and iteration order fix

**File:** `pumpkin-world/src/chunk/palette.rs`

Bedrock biome storage is per-block inside each sub-chunk (`16x16x16 = 4096`
entries), unlike the Java `4x4x4` biome palette. Changed `convert_be_network`
for biomes to output 4096 entries and iterate in XZY order (Bedrock sub-chunk
order).

**Result:** still crashed.

---

## 3. Block sub-chunk iteration order fix

**File:** `pumpkin-world/src/chunk/palette.rs`

Changed `BlockPalette::convert_be_network` to pack blocks in XZY order
(`x` outer, `z` middle, `y` inner), which matches Bedrock's sub-chunk ordering.

**Result:** still crashed.

---

## 4. Removed the biome-section prefix from LevelChunk

**File:** `pumpkin-protocol/src/bedrock/client/level_chunk.rs`

Removed the `[version][num_storages][y]` prefix from biome data in the
`LevelChunk` payload. Biomes are raw palette serializations only, not full
sub-chunks.

**Result:** still crashed.

---

## 5. Switched to the SubChunk Request System

Modern Bedrock (1.18.10+) normally uses skeleton `LevelChunk` packets with
`sub_chunk_count = -1` (limitless) and pulls block sub-chunks via
`SubChunkRequest` / `SubChunk` packets. Pumpkin was still sending 24 inline
block sub-chunks, which 1.26.x may no longer accept.

### Changes

- **`pumpkin-protocol/src/bedrock/client/level_chunk.rs`**
  - `CLevelChunk` now sends `sub_chunk_count = u32::MAX` and only writes the
    24 biome sections + a zero border-block count.
  - Added a public helper `serialize_bedrock_block_subchunk(y, palette)` for
    re-use in `SubChunk` responses.

- **`pumpkin-protocol/src/bedrock/server/sub_chunk_request.rs`** (new)
  - `SSubChunkRequest` (packet 175) with manual `PacketRead`.
  - `SubChunkOffset` struct (3x `i8`).

- **`pumpkin-protocol/src/bedrock/client/sub_chunk.rs`** (new)
  - `CSubChunkPacket` (packet 174) with manual `PacketWrite`.
  - `SubChunkEntry`, `SubChunkOffset` and result/heightmap constants.

- **`pumpkin/src/net/bedrock/play.rs`**
  - `handle_sub_chunk_request`: for each requested offset it loads the chunk,
    serializes the requested sub-chunk, computes a per-sub-chunk heightmap,
    and replies with a `CSubChunkPacket`.
  - `handle_request_chunk_radius` now also sends
    `CNetworkChunkPublisherUpdate` so the client knows where to request
    chunks from.

### Build

`cargo check -p pumpkin --bin pumpkin` passes.
Debug binary rebuilt at `../PumpkinRunner/target/debug/pumpkin.exe`.

**Result:** still crashed with the same `forcibly closed` error right after the
render-distance update. No `SubChunkRequest` packets appear in the log, so the
client is closing before it ever asks for sub-chunks.

---

## Current working hypothesis

The crash is happening before chunk data matters. The client successfully:

1. Handshakes / logs in.
2. Downloads / accepts resource packs.
3. Receives `StartGame`, `ItemRegistry`, `CreativeContent`, `CraftingData`,
   `InventoryContent`, `PlayerList`, `AddPlayer`, `SetActorData`,
   `UpdateAttributes`.
4. Sends `RequestChunkRadius`.

It then crashes immediately after the server replies with
`ChunkRadiusUpdate` (+ now `NetworkChunkPublisherUpdate`) and starts sending
chunk-related packets.

Possibilities still on the table:

- `CChunkRadiusUpdate` or `CNetworkChunkPublisherUpdate` is malformed for
  protocol 1001.
- The skeleton `LevelChunk` payload is still not exactly what 1.26.30 expects
  (e.g. wrong biome count/format, missing/extra fields).
- A packet sent *before* chunks (inventory, player list, add-player, actor
  data, attributes, etc.) is malformed and the client only crashes when it
  finishes processing the next burst.
- The RakNet fragmentation / reassembly path has a bug that corrupts large
  packets, and the first big packet after login (even a small skeleton chunk
  burst) triggers the close.

---

## Next useful step

To narrow it down we need to know exactly which packet the client dislikes.
The cheapest experiment is to temporarily suppress chunk sending after
`RequestChunkRadius` and see if the client stays connected. If it still
crashes, the problem is in the pre-chunk spawn packets; if it stays connected,
the problem is in `ChunkRadiusUpdate`, `NetworkChunkPublisherUpdate`, or
`LevelChunk`/`SubChunk`.
