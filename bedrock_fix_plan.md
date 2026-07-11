# Bedrock Join Failure: Investigation and Implementation Plan

## Objective

Restore reliable Bedrock client joins in this fork while avoiding changes to world generation. The initial implementation should return the Bedrock networking path to known-working behavior from `../Original-Pumpkin`, then use focused instrumentation to identify any remaining failure caused by the Runner world, entity traffic, or RakNet queuing.

## Implementation Status

The focused parity implementation is complete and awaiting runtime validation:

- The inline LevelChunk serializer is restored.
- The experimental SubChunk request/response path is removed.
- Duplicate chunk publisher and diagnostic suppression behavior are removed.
- Original chunk batching, PlayerSpawn sequencing, and `try_enqueue_packet` behavior are restored.
- All six affected core files match the pre-experiment revision by Git blob hash.
- `cargo check -p pumpkin --bin pumpkin` passes.
- No world-generation or `pumpkin-world` files were changed.

The controlled runtime matrix and later diagnostic/queue phases remain conditional on the Bedrock test result.

### Follow-up after parity test

The first parity test still crashed before `SetLocalPlayerAsInitialized`. Its log contained 514 `Failed to lock network writer for try_enqueue_packet` messages and 4,301 reliable-frame resends. This confirmed that copying Original's lossy `try_read` behavior is not sufficient for the Runner world's join-time traffic.

Phase 4 has therefore been partially implemented:

- Synchronous world/entity producers serialize only the typed packet payload.
- They submit that payload and packet ID directly to the existing bounded outgoing channel.
- The single outgoing consumer performs Bedrock wrapping, compression, encryption, and RakNet submission in queue order.
- A failed encoder `try_read` no longer drops the packet.
- No independent task is spawned per packet.

Runtime validation is required to determine whether the missing packets were the direct crash cause or whether the next trace must focus on the remaining traffic volume and resend behavior.

## Current Evidence

The core Bedrock login and spawn protocol is not broadly different between the two repositories:

- Before the recent debugging changes, `pumpkin/src/net/bedrock/mod.rs` and the inline `LevelChunk` serializer had the same Git blobs as `Original-Pumpkin`.
- `handle_request_chunk_radius` had the same behavior.
- `spawn_bedrock_player` is still byte-for-byte identical.
- StartGame, registries, inventory, player list, actor metadata, attributes, packet encoding/decoding, and the principal RakNet connection files are identical.

Archived failing runs show this sequence:

1. The client completes login and resource-pack negotiation.
2. The server negotiates render distance and queues chunks.
3. The client sends `SetLocalPlayerAsInitialized`.
4. The server receives a very large contiguous NACK range.
5. The connection times out or is closed.

Because the client sends `SetLocalPlayerAsInitialized`, at least those runs reached the post-loading state. This makes a failure in the initial StartGame sequence less likely and points toward the chunk/entity traffic burst or outgoing RakNet queue behavior after spawning.

The Windows UDP error `os error 10054` is not sufficient to identify the failure. The working `Original-Pumpkin` run also logs that error.

## Important Runtime Difference

The current comparison is not yet controlled:

- `Original-Pumpkin` runs against its own relatively fresh world and runtime directory.
- This fork is normally run from `../PumpkinRunner`, which has persistent world, player, and entity data.
- Runner uses view distance 6 and simulation distance 5; Original uses 16 and 10.
- The seeds, autosave settings, and logging settings differ.
- Failing Runner logs show the loaded entity count rising to roughly 170-180 entities after initialization.

The investigation must separate binary differences from runtime-data differences before assigning the original failure to a particular code change.

## Current Experimental Differences

The current branch has moved away from the working implementation in several places:

1. `pumpkin-protocol/src/bedrock/client/level_chunk.rs` sends skeleton LevelChunk packets rather than the working implementation's 24 inline block subchunks.
2. New `SubChunkRequest` and `SubChunk` packet implementations were added.
3. `handle_request_chunk_radius` sends a `NetworkChunkPublisherUpdate` even though the shared chunker already sends one.
4. Bedrock chunk delivery was limited to one chunk per tick, and the relationship between chunk delivery and `PlayerSpawn` was changed.
5. `try_enqueue_packet` was changed from immediate synchronous encoding/queuing to an independently spawned asynchronous task for every packet.
6. `CHUNK_DATA_ENABLED` currently suppresses LevelChunk, SubChunk, and PlayerSpawn packets.

The asynchronous `try_enqueue_packet` rewrite is the strongest suspect for the newer NACK flood. It can reorder packets and accumulate many independent tasks when chunks and large numbers of entity updates are produced together. Because it was introduced during debugging, it cannot by itself explain the earliest failure, but it can obscure or worsen that failure now.

## Scope Boundary

The first implementation should be restricted to:

- Bedrock protocol serialization.
- Bedrock packet sequencing.
- Outgoing packet queue behavior.
- RakNet fragmentation diagnostics.
- Initial entity synchronization and throttling, if the evidence requires it.

Do not initially change:

- Terrain or biome generation.
- Feature generation.
- Chunk palette generation.
- The general chunk-generation scheduler.
- Existing world files.

If a saved chunk is later proven to trigger the failure, handle it at the Bedrock serialization or corrupt-data validation boundary before considering any generator change.

## Phase 1: Controlled Reproduction Matrix

Use disposable copies of runtime data so the existing Runner world is not modified during comparison.

Test these combinations with the same Bedrock client:

| Binary | Runtime data | Purpose |
| --- | --- | --- |
| Original | Original clean runtime | Confirm the known-working baseline |
| Current fork | Original clean runtime | Determine whether the failure follows the binary |
| Original | Copied Runner runtime/world | Determine whether the failure follows saved data |
| Current fork | Copied Runner runtime/world | Reproduce the normal failing case under controlled conditions |

For every run, record:

- Whether the client reaches the world.
- Whether `SetLocalPlayerAsInitialized` is received.
- Time from join to initialization.
- Time and sequence range of the first substantial NACK.
- Loaded chunk and entity counts.
- Whether the connection remains usable for at least 60 seconds.

This matrix determines the next path:

- If the failure follows the current binary on both runtimes, prioritize protocol and queue parity.
- If the failure follows Runner data with both binaries, prioritize chunk/entity serialization diagnostics.
- If only the current binary plus Runner data fails, prioritize traffic volume, ordering, and backpressure interactions.

## Phase 2: Restore the Known-Working Bedrock Path

Make a narrow parity change based on `../Original-Pumpkin`:

1. Restore the inline LevelChunk serializer with 24 block subchunks and its known-working biome encoding.
2. Remove the experimental SubChunk request/response packet files, registrations, and handler.
3. Remove the duplicate `NetworkChunkPublisherUpdate` from `handle_request_chunk_radius`.
4. Restore the original Bedrock chunk batching and PlayerSpawn flow.
5. Remove `CHUNK_DATA_ENABLED` and its conditional behavior.
6. Restore the original `try_enqueue_packet` behavior for the parity test.

Keep this parity change separate from subsequent architectural improvements. Its purpose is to establish whether exact known-working behavior succeeds in this fork and with the Runner runtime.

## Phase 3: Add Targeted Networking Diagnostics

Add debug-level tracing around the outgoing Bedrock path. For each significant packet, record:

- Packet ID and packet name when available.
- Encoded game-packet size.
- Compressed/encrypted batch size.
- RakNet fragment count.
- Outgoing queue depth.
- Assigned RakNet sequence-number range.
- Whether the packet was newly sent or resent.
- Time relative to PlayerSpawn and client initialization.

Give special treatment to:

- LevelChunk.
- PlayerSpawn.
- AddActor/AddPlayer and actor metadata.
- Inventory and registry packets.
- Any packet that produces an unusually large batch.

The diagnostics should make it possible to map the first large NACK range back to the exact packet or burst that created it.

Avoid logging every frame indefinitely. Use debug configuration, sampling, or join-phase-only tracing so normal operation is not flooded with logs.

## Phase 4: Implement an Ordered, Bounded Outgoing Writer

If the parity test or diagnostics implicate queuing, replace the task-per-packet design with a single ordered writer:

1. All producers submit typed packets or pre-serialized payloads to one bounded channel.
2. One consumer performs final Bedrock wrapping, compression/encryption, and RakNet queuing in submission order.
3. Large packets cannot be overtaken by later entity updates.
4. Queue saturation applies controlled backpressure or an explicit packet-class policy.
5. Queue saturation and dropped/coalesced packets are observable in metrics or logs.
6. Encryption/compression state remains confined to the ordered writer.

Do not silently discard required join packets. If noncritical live entity updates must be reduced under pressure, coalesce superseded updates rather than allowing an unbounded backlog.

## Phase 5: Stage Initial Entity Synchronization

Only implement this phase if diagnostics show that the post-spawn entity burst is responsible.

1. Complete the minimum spawn and initial chunk sequence first.
2. Wait for `SetLocalPlayerAsInitialized` before draining nonessential entity state.
3. Send initial actor spawns and metadata in bounded batches.
4. Coalesce repeated movement, metadata, or attribute updates generated while the client is loading.
5. Inspect why the Runner world exposes approximately 170-180 entities during join and verify that restored entities are not duplicated.

This phase concerns entity lifecycle and network synchronization, not world generation.

## Phase 6: Validate Chunk Data Only if Required

If controlled testing shows that the failure follows particular Runner chunks even with the Original binary behavior, add serializer-side validation for:

- Java-to-Bedrock block runtime-ID mappings.
- Palette sizes and calculated bit widths.
- Single-value palette encoding.
- Biome palette contents.
- Section counts and vertical indices.
- Final LevelChunk encoded size.
- The coordinates of the first chunk associated with a NACK or disconnect.

Prefer rejecting, substituting, or reporting invalid network data over modifying generation. Do not regenerate or alter saved chunks without a separate finding and explicit approval.

## Tests to Add

1. **LevelChunk fixture test**
   - Serialize a deterministic `SyncChunk`.
   - Compare its output with a captured known-working fixture from `Original-Pumpkin`.

2. **Packet ordering test**
   - Submit packets concurrently from multiple producers.
   - Verify that the writer emits them in accepted queue order.

3. **Backpressure test**
   - Fill the outgoing channel with large packets.
   - Verify bounded memory use and the defined behavior for critical and coalescible packets.

4. **Join-sequence regression test**
   - Verify that required join packets, initial chunks, PlayerSpawn, and entity synchronization occur in the intended phases.

5. **Reconnect test**
   - Verify that stale queued packets from an old Bedrock connection cannot leak into a replacement connection.

## Acceptance Criteria

The fix is complete when all of the following hold:

- A Bedrock client reaches the world and sends `SetLocalPlayerAsInitialized`.
- The client remains connected and usable for at least 60 seconds.
- No large contiguous NACK storm or unbounded resend queue occurs.
- Joining works with a fresh runtime and a copied Runner runtime.
- First join and reconnect both work.
- Initial entities appear without flooding or crashing the client.
- The existing Runner world is not regenerated or modified as part of the fix.
- A Java client can still join and play normally.
- LevelChunk serialization has a deterministic regression fixture.
- Outgoing packet ordering and backpressure are covered by tests.

## Recommended Implementation Order

1. Run the controlled binary/runtime matrix.
2. Restore exact Original Bedrock parity in one focused change.
3. Repeat the matrix and capture packet/fragment diagnostics.
4. Implement the ordered bounded writer if queuing remains implicated.
5. Stage initial entity traffic only if the trace shows it is necessary.
6. Investigate individual chunk serialization only if the failure follows Runner chunk data.

Keep the SubChunk protocol migration out of the crash fix. Reintroduce it later as a separate feature after the inline known-working path is stable and covered by regression tests.
