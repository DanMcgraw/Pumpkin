# Phase 1 — Foundation Event Parity Plan

This document is the detailed expansion of **Phase 1** from [`plan-api.md`](./plan-api.md). It covers the events that are **already exposed** to native DLL plugins in Pumpkin and that mcMMO already depends on.

**Goal of Phase 1:** verify that these events are fired in the correct places, carry enough data, and behave closely enough to their Bukkit/Spigot equivalents that mcMMO-style plugins can use them without waiting for later phases.

---

## Phase 1 Event Checklist

| # | Bukkit/Spigot event (mcMMO) | Pumpkin event | Status |
|---|-----------------------------|---------------|--------|
| 1 | `PlayerJoinEvent` | `PlayerJoinEvent` | ✅ Exposed |
| 2 | `PlayerQuitEvent` | `PlayerLeaveEvent` | ✅ Exposed (note name difference) |
| 3 | `AsyncPlayerChatEvent` | `PlayerChatEvent` | ⚠️ Exposed but synchronous |
| 4 | `PlayerCommandPreprocessEvent` | `PlayerCommandSendEvent` | ⚠️ Exposed but different semantics |
| 5 | `PlayerInteractEvent` | `PlayerInteractEvent` | ✅ Exposed |
| 6 | `PlayerInteractEntityEvent` | `PlayerInteractEntityEvent` | ✅ Exposed |
| 7 | `PlayerFishEvent` | `PlayerFishEvent` | ⚠️ Exposed but minimal state coverage |
| 8 | `PlayerTeleportEvent` | `PlayerTeleportEvent` | ✅ Exposed |
| 9 | `PlayerChangedWorldEvent` | `PlayerChangeWorldEvent` | ✅ Exposed |
| 10 | `PlayerRespawnEvent` | `PlayerRespawnEvent` | ✅ Exposed |
| 11 | `InventoryClickEvent` | `InventoryClickEvent` | ✅ Exposed |
| 12 | `BlockBreakEvent` | `BlockBreakEvent` | ✅ Exposed |
| 13 | `BlockPlaceEvent` | `BlockPlaceEvent` | ✅ Exposed |
| 14 | `BlockGrowEvent` | `BlockGrowEvent` | ⚠️ Exposed but crop-only |
| 15 | `CreatureSpawnEvent` | `EntitySpawnEvent` | ⚠️ Exposed but generic |

---

## 1. PlayerJoinEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java:637-653`

mcMMO uses `PlayerJoinEvent` to:

- Schedule async player profile loading after a 3-second delay (`PlayerProfileLoadingTask`).
- Display the mcMMO MOTD if enabled and the player has permission.
- Notify the player if an XP multiplier event is active.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onPlayerJoin(PlayerJoinEvent event) {
    Player player = event.getPlayer();
    mcMMO.p.getFoliaLib().getScheduler()
            .runLaterAsync(new PlayerProfileLoadingTask(player), 60);

    if (mcMMO.p.getGeneralConfig().getMOTDEnabled() && Permissions.motd(player)) {
        Motd.displayAll(player);
    }

    if (plugin.isXPEventEnabled() && mcMMO.p.getGeneralConfig().playerJoinEventInfo()) {
        player.sendMessage(LocaleLoader.getString("XPRate.Event",
                ExperienceConfig.getInstance().getExperienceGainsGlobalMultiplier()));
    }
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/player/player_join.rs`
**Fired in:** `pumpkin/src/world/mod.rs:2485-2492` (Bedrock path) and `:3120-3129` (Java path)

```rust
let event = PlayerJoinEvent::new(player.clone(), msg_comp);
let event = server.plugin_manager.fire(event).await;

if !event.cancelled {
    self.broadcast_system_message(&event.join_message, false).await;
    info!("{}", event.join_message.to_pretty_console());
}
```

### Required behavior for mcMMO parity

- The event must fire **after** the player object exists but **before** the join message is broadcast.
- Plugins should be able to read the player and the join message, and optionally cancel the broadcast.
- mcMMO does not cancel this event, so cancellation support is a nice-to-have but not critical.

### Gaps / action items

- Verify both Java and Bedrock spawn paths fire the event. ✅ Currently true.
- Consider whether the event should be cancellable to prevent spawning entirely. Currently it only cancels the broadcast.

---

## 2. PlayerQuitEvent → PlayerLeaveEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java:609-627`

mcMMO uses `PlayerQuitEvent` to:

- Save the player profile (sync save if the server is shutting down).
- Clean transient metadata from the player.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onPlayerQuit(PlayerQuitEvent event) {
    Player player = event.getPlayer();
    final McMMOPlayer mmoPlayer = UserManager.getPlayer(player);
    mmoPlayer.logout(mcMMO.isServerShutdownExecuted());
    mcMMO.getTransientMetadataTools().cleanLivingEntityMetadata(event.getPlayer());
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/player/player_leave.rs`
**Fired in:** `pumpkin/src/world/mod.rs:4167-4182`

```rust
let event = PlayerLeaveEvent::new(player.clone(), msg_comp);
let event = self.server.upgrade().unwrap().plugin_manager.fire(event).await;

if !event.cancelled {
    for player in self.players.load().iter() {
        player.send_system_message(&event.leave_message).await;
    }
    info!("{}", event.leave_message.to_pretty_console());
}
```

### Required behavior for mcMMO parity

- Fire after the player object is still valid but before removal is complete.
- Allow plugins to perform cleanup/saving.
- mcMMO does not cancel quit, so cancellation is only for suppressing the leave message.

### Gaps / action items

- The name differs (`PlayerLeaveEvent` vs `PlayerQuitEvent`). This is fine for a port but document it in plugin migration guides.
- Ensure the player data is still accessible when the event fires. ✅ Currently true.

---

## 3. AsyncPlayerChatEvent → PlayerChatEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java:1093-1127`

mcMMO uses `AsyncPlayerChatEvent` to:

- Route messages to party/admin chat channels.
- Cancel the vanilla chat event when a custom channel handles the message.

```java
@EventHandler(priority = EventPriority.HIGH, ignoreCancelled = true)
public void onPlayerChat(AsyncPlayerChatEvent event) {
    Player player = event.getPlayer();
    final McMMOPlayer mmoPlayer = UserManager.getOfflinePlayer(player);

    if (plugin.getChatManager().isChatChannelEnabled(mmoPlayer.getChatChannel())) {
        if (mmoPlayer.getChatChannel() != ChatChannel.NONE) {
            if (plugin.getChatManager().isMessageAllowed(mmoPlayer)) {
                plugin.getChatManager().processPlayerMessage(mmoPlayer, event.getMessage(),
                        event.isAsynchronous());
                event.setCancelled(true);
            } else {
                plugin.getChatManager().setOrToggleChatChannel(mmoPlayer, ...);
            }
        }
    }
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/player/player_chat.rs`
**Fired in:** `pumpkin/src/net/java/play.rs:1349-1353` and `pumpkin/src/net/bedrock/play.rs:906`

```rust
send_cancellable! {{
    server;
    PlayerChatEvent::new(player.clone(), chat_message.message.to_string(), vec![]);

    'after: {
        info!("<chat> {}: {}", gameprofile.name, event.message);
        // ... broadcast chat ...
    }
}};
```

### Required behavior for mcMMO parity

- mcMMO needs to read the player, message, and recipients; optionally cancel.
- mcMMO uses `event.isAsynchronous()` when processing. Pumpkin's event is fired synchronously on the network task.

### Gaps / action items

- **Synchrony difference:** Bukkit fires `AsyncPlayerChatEvent` on an async thread. If mcMMO chat processing performs blocking I/O (e.g., database lookups for offline party members), running it synchronously in Pumpkin may lag the server. Evaluate whether to:
  - Add an async chat variant.
  - Move heavy processing off the event handler via channels/tasks.
- The `recipients` list is present but empty by default. mcMMO may want to mutate it. Verify it is respected by the broadcast logic.

---

## 4. PlayerCommandPreprocessEvent → PlayerCommandSendEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java:1140-1163`

mcMMO uses `PlayerCommandPreprocessEvent` to:

- Localize skill command names for non-English locales.
- Rewrite `/mineria` → `/mining` etc. before the server parses the command.

```java
@EventHandler(priority = EventPriority.LOW, ignoreCancelled = true)
public void onPlayerCommandPreprocess(PlayerCommandPreprocessEvent event) {
    if (!mcMMO.p.getGeneralConfig().getLocale().equalsIgnoreCase("en_US")) {
        String message = event.getMessage();
        String command = message.substring(1).split(" ")[0];
        String lowerCaseCommand = command.toLowerCase(Locale.ENGLISH);

        for (PrimarySkillType skill : PrimarySkillType.values()) {
            String skillName = skill.toString().toLowerCase(Locale.ENGLISH);
            String localizedName = mcMMO.p.getSkillTools().getLocalizedSkillName(skill)
                    .toLowerCase(Locale.ENGLISH);

            if (lowerCaseCommand.equals(localizedName)) {
                event.setMessage(message.replace(command, skillName));
                break;
            }
        }
    }
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/player/player_command_send.rs`
**Fired in:** `pumpkin/src/net/java/play.rs:685` and `pumpkin/src/net/bedrock/play.rs:1079`

### Required behavior for mcMMO parity

- mcMMO needs to read and optionally rewrite the raw command string before it is dispatched.
- Pumpkin's `PlayerCommandSendEvent` may be intended for command-completion or sending available commands rather than preprocessing. Verify the payload contains the raw command text and that cancellation/rewriting works.

### Gaps / action items

- **Confirm semantics:** If `PlayerCommandSendEvent` is not the right hook for command preprocessing, add a `PlayerCommandPreprocessEvent` in a later phase.
- Ensure the event fires for both Java and Bedrock command packets and that plugins can mutate the command string.

---

## 5. PlayerInteractEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java:684-824` (LOWEST), `:843-889` (LOW), `:896-999` (MONITOR)

mcMMO uses `PlayerInteractEvent` for many things:

- LOWEST priority: Repair/Salvage anvil interaction, Blast Mining detonation.
- LOW priority: Deny item use while an anvil confirmation is pending.
- MONITOR priority: Ability activation (Axes, Excavation, Mining, Swords, Unarmed, Woodcutting), Herbalism Green Thumb, Chimaera Wing, and spam-fishing detection.

```java
@EventHandler(priority = EventPriority.LOWEST, ignoreCancelled = true)
public void onPlayerInteractLowest(PlayerInteractEvent event) {
    // Repair/Salvage anvil logic
    // Blast Mining remote detonation
}

@EventHandler(priority = EventPriority.MONITOR)
public void onPlayerInteractMonitor(PlayerInteractEvent event) {
    // Ability activation checks
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/player/player_interact_event.rs`
**Fired in:** `pumpkin/src/net/java/play.rs:1300-1318`, `:2182-2200`, `:2417-2448`

```rust
let event = if let Some((hit_pos, _hit_dir)) = hit_result {
    PlayerInteractEvent::new(
        player,
        InteractAction::LeftClickBlock,
        player.world().get_block(&hit_pos),
        Some(hit_pos),
    )
} else {
    PlayerInteractEvent::new(player, InteractAction::LeftClickAir, &Block::AIR, None)
};

send_cancellable! {{
    server;
    event;
    'after: {
        player.swing_hand(hand, false).await;
    }
}};
```

### Required behavior for mcMMO parity

- Must distinguish left/right click and block/air.
- Must include the clicked block, block position, and player.
- Must support multiple handler priorities (LOWEST, LOW, MONITOR).
- mcMMO uses `event.getHand()` to know main/off hand and `event.getAction()` for `RIGHT_CLICK_BLOCK`, `LEFT_CLICK_BLOCK`, etc.

### Gaps / action items

- Add `hand: Hand` to `PlayerInteractEvent` if not already present. mcMMO uses this for fishing-rod spam detection and tool checks.
- Verify that `Action.PHYSICAL` (pressure plates) is skipped or handled correctly. mcMMO returns early on physical interactions.
- Ensure that `useInteractedBlock()` / `useItemInHand()` results are exposed if Pumpkin supports them.

---

## 6. PlayerInteractEntityEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java:1171-1203`

mcMMO uses `PlayerInteractEntityEvent` to:

- Remove ability buffs from items placed into item frames.
- Possibly used for taming interactions elsewhere.

```java
@EventHandler(priority = EventPriority.HIGH, ignoreCancelled = true)
public void onPlayerInteractEntity(PlayerInteractEntityEvent event) {
    if (event.getRightClicked() instanceof ItemFrame frame) {
        // Remove ability buffs from items placed in item frames
    }
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/player/player_interact_entity_event.rs`
**Fired in:** `pumpkin/src/net/java/play.rs:1749-1800`

```rust
send_cancellable! {{
    server;
    PlayerInteractEntityEvent::new(
        player,
        Arc::clone(&target),
        action.clone(),
        interact.target_position,
        sneaking,
    );

    'after: {
        match event.action {
            ActionType::Attack => { /* attack handling */ }
            ActionType::Interact | ActionType::InteractAt => { /* interact handling */ }
        }
    }
}};
```

### Required behavior for mcMMO parity

- Must include the player, target entity, interaction type, and whether the player is sneaking.
- mcMMO uses `getRightClicked()` (the entity). Pumpkin exposes `target: Arc<dyn EntityBase>`.

### Gaps / action items

- mcMMO distinguishes attack vs interact. Pumpkin's `ActionType` enum covers this. ✅
- If mcMMO needs to know the hand used, verify it is exposed or add it.

---

## 7. PlayerFishEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java:318-498`

mcMMO uses `PlayerFishEvent` for:

- HIGH priority: Anti-exploit, ice fishing, treasure replacement, vanilla XP boost.
- MONITOR priority: Master Angler, fishing skill processing, scarcity/anti-exploit, shake.

```java
@EventHandler(priority = EventPriority.HIGH, ignoreCancelled = true)
public void onPlayerFishHighest(PlayerFishEvent event) {
    switch (event.getState()) {
        case CAUGHT_FISH:
            // Replace treasure, apply vanilla XP boost
            break;
        case IN_GROUND:
            // Ice fishing
            break;
    }
}

@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onPlayerFishMonitor(PlayerFishEvent event) {
    // Master Angler, processFishing, shake
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/player/fish.rs`
**Fired in:** `pumpkin/src/net/java/play.rs:2536-2546`

```rust
let fish_event = PlayerFishEvent::new(
    player.clone(),
    None,
    uuid::Uuid::nil(),
    String::new(),
    PlayerFishState::Fishing,
    hand,
    0,
);
let fish_event = server.plugin_manager.fire(fish_event).await;
!fish_event.cancelled
```

Currently only the `Fishing` state is fired (when starting to fish).

### Required behavior for mcMMO parity

- All `PlayerFishEvent.State` values must be supported: `FISHING`, `CAUGHT_FISH`, `CAUGHT_ENTITY`, `IN_GROUND`, `FAILED_ATTEMPT`, `REEL_IN`, `BITE`.
- Must expose the caught entity, hook, experience to drop, and hand.

### Gaps / action items

- **Major gap:** only `Fishing` state is emitted. Expand firing to catch success, entity catch, ground hit, bite, reel-in, and failure.
- Add the fishing hook entity reference and caught entity reference when applicable.
- Ensure the `exp_to_drop` field is populated and respected.

---

## 8. PlayerTeleportEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java:96-134`

mcMMO uses `PlayerTeleportEvent` to:

- Set the player's last teleportation timestamp to prevent Acrobatics exploitation.
- Update scoreboards when moving into/out of blacklisted worlds.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onPlayerTeleport(PlayerTeleportEvent event) {
    Player player = event.getPlayer();
    UserManager.getPlayer(player).actualizeTeleportATS();
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/player/player_teleport.rs`
**Fired in:** `pumpkin/src/entity/player.rs:2582-2612`

```rust
send_cancellable! {{
    server;
    PlayerTeleportEvent {
        player: self.clone(),
        from: self.living_entity.entity.pos.load(),
        to: position,
        cancelled: false,
    };

    'after: {
        let position = event.to;
        // ... perform teleport ...
    }
}};
```

### Required behavior for mcMMO parity

- Must fire before teleportation and allow cancellation.
- Must include from/to positions.

### Gaps / action items

- mcMMO uses `event.getFrom()` and `event.getTo()` as `Location` (world + coords). Pumpkin currently only stores `Vector3<f64>`. If mcMMO needs world comparison, add world fields.
- mcMMO only monitors at MONITOR priority, so cancellation support is not required for mcMMO but is useful for other plugins.

---

## 9. PlayerChangedWorldEvent → PlayerChangeWorldEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java:259-276`

mcMMO uses `PlayerChangedWorldEvent` to:

- Remove god mode if not allowed in the destination world.
- Remove the player from parties if not allowed in the destination world.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onPlayerWorldChange(PlayerChangedWorldEvent event) {
    Player player = event.getPlayer();
    final McMMOPlayer mmoPlayer = UserManager.getPlayer(player);
    mmoPlayer.checkGodMode();
    mmoPlayer.checkParty();
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/player/player_change_world.rs`
**Fired in:** `pumpkin/src/entity/player.rs:2478-2508`

```rust
send_cancellable! {{
    server;
    PlayerChangeWorldEvent {
        player: self.clone(),
        previous_world: current_world.clone(),
        new_world: new_world.clone(),
        position,
        yaw,
        pitch,
        cancelled: false,
    };

    'after: {
        // ... perform world transfer ...
    }
}};
```

### Required behavior for mcMMO parity

- Must expose previous world, new world, and destination position/rotation.
- Must fire before the actual dimension change so plugins can cancel or redirect.

### Gaps / action items

- mcMMO uses `event.getFrom()` and `event.getTo()` (world objects). Pumpkin exposes `previous_world` and `new_world`. ✅
- Verify the event fires for all dimension transitions: portals, commands, respawn, plugins.

---

## 10. PlayerRespawnEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java:663-677`

mcMMO uses `PlayerRespawnEvent` to:

- Set the player's last respawn timestamp to prevent exploitation.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onPlayerRespawn(PlayerRespawnEvent event) {
    Player player = event.getPlayer();
    UserManager.getPlayer(player).actualizeRespawnATS();
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/player/player_respawn.rs`
**Fired in:** `pumpkin/src/world/mod.rs:3396-3409`

```rust
if let Some(server) = self.server.upgrade() {
    let _ = server
        .plugin_manager
        .fire(PlayerRespawnEvent::new(
            player.clone(),
            self.clone(),
            target_world.clone(),
            position,
            yaw,
            pitch,
            alive,
        ))
        .await;
}
```

### Required behavior for mcMMO parity

- Fire after the respawn destination is determined.
- Provide player, previous world, respawn world, position, rotation.

### Gaps / action items

- mcMMO only monitors this event. The current non-cancellable design matches mcMMO's use. ✅
- If other plugins need to modify respawn location, consider making fields mutable.

---

## 11. InventoryClickEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/InventoryListener.java:180-313`, `:455-473`

mcMMO uses `InventoryClickEvent` to:

- Process furnace/brewing stand ownership.
- Handle custom alchemy ingredient transfers.
- Remove ability buffs from clicked items.

```java
@EventHandler(priority = EventPriority.NORMAL, ignoreCancelled = true)
public void onInventoryClickEventNormal(InventoryClickEvent event) {
    // Furnace/BrewingStand ownership
    // Alchemy transfers
}

@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onInventoryClickEvent(InventoryClickEvent event) {
    if (event.getCurrentItem() == null) return;
    SkillUtils.removeAbilityBuff(event.getCurrentItem());
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/player/inventory_interact.rs`
**Fired in:** `pumpkin/src/entity/player.rs:3856-3873`

```rust
send_cancellable! {{
    server;
    InventoryClickEvent::new(
        self,
        screen_handler.window_type(),
        click_type,
        slot,
        raw_slot,
        clicked_item,
        cursor_item,
        i32::from(hotbar_button),
    );
    'after: {}
    'cancelled: {
        screen_handler.cancel().await;
        return;
    }
}};
```

### Required behavior for mcMMO parity

- Must expose player, clicked item, cursor item, slot, raw slot, click type, hotbar button.
- Must support cancellation.
- mcMMO casts `event.getWhoClicked()` to `Player`; Pumpkin passes `Arc<Player>` directly.

### Gaps / action items

- Verify `ClickType` mapping covers all Bukkit `ClickType` values (LEFT, RIGHT, SHIFT_LEFT, SHIFT_RIGHT, NUMBER_KEY, etc.).
- Add `InventoryDragEvent` support for alchemy ingredient dragging (Phase 4).

---

## 12. BlockBreakEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/BlockListener.java:418-543` (MONITOR), `:550-606` (HIGHEST)

mcMMO uses `BlockBreakEvent` for:

- MONITOR: Mining, Woodcutting, Excavation XP and abilities; Herbalism; Alchemy brew cancellation.
- HIGHEST: Hylian Luck (sword breaking blocks).

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onBlockBreak(BlockBreakEvent event) {
    // Mining, Woodcutting, Excavation, Herbalism checks
}

@EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
public void onBlockBreakHigher(BlockBreakEvent event) {
    // Hylian Luck
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/block/block_break.rs`
**Fired in:** `pumpkin/src/world/mod.rs:4667-4681`

```rust
let event = BlockBreakEvent::new(
    cause.clone(),
    broken_block,
    *position,
    0,
    !flags.contains(BlockFlags::SKIP_DROPS),
);

let event = self.server.upgrade().unwrap().plugin_manager.fire::<BlockBreakEvent>(event).await;

if !event.cancelled {
    let mut flags = flags;
    if event.drop { flags.remove(BlockFlags::SKIP_DROPS); }
    else { flags.insert(BlockFlags::SKIP_DROPS); }
    // ... break block ...
}
```

### Required behavior for mcMMO parity

- Must include player, block, position, and whether drops should occur.
- Must be cancellable.
- mcMMO reads `event.getPlayer()`, `event.getBlock()`, and uses metadata on the block for natural/unnatural tracking.

### Gaps / action items

- Pumpkin exposes `exp` and `drop` fields. mcMMO does not modify XP through this event directly, so this is fine.
- mcMMO relies on block metadata for natural/unnatural tracking. Pumpkin does not have a direct equivalent; a port will need to store this state elsewhere (e.g., in a world-backed block tracker).

---

## 13. BlockPlaceEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/BlockListener.java:329-368`

mcMMO uses `BlockPlaceEvent` to:

- Mark placed blocks as unnatural so they do not grant skill XP.
- Track Repair/Salvage anvil placements.

```java
@EventHandler(priority = EventPriority.MONITOR)
public void onBlockPlace(BlockPlaceEvent event) {
    Block block = event.getBlock().getState().getBlock();
    if (BlockUtils.isWithinWorldBounds(block)) {
        if (!(event instanceof BlockMultiPlaceEvent)) {
            BlockUtils.setUnnaturalBlock(block);
        }
    }

    Player player = event.getPlayer();
    final McMMOPlayer mmoPlayer = UserManager.getPlayer(player);

    if (blockState.getType() == repairAnvilMaterial) {
        mmoPlayer.getRepairManager().placedAnvilCheck();
    } else if (blockState.getType() == salvageAnvilMaterial) {
        mmoPlayer.getSalvageManager().placedAnvilCheck();
    }
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/block/block_place.rs`
**Fired in:** `pumpkin/src/block/registry.rs:593-605`

```rust
let event = crate::plugin::block::block_place::BlockPlaceEvent::new(
    player.clone(),
    placed_block,
    clicked_block,
    final_block_pos,
    true,
);
let event = server.plugin_manager.fire::<crate::plugin::block::block_place::BlockPlaceEvent>(event).await;
if event.cancelled {
    return Ok(None);
}
```

### Required behavior for mcMMO parity

- Must include player, placed block, block placed against, position, and build permission.
- Must be cancellable.

### Gaps / action items

- mcMMO uses `event.getBlockReplacedState()`. Pumpkin does not currently expose the replaced block state. Add it if needed.
- `BlockMultiPlaceEvent` (doors, beds, etc.) is not yet exposed; track in Phase 6.

---

## 14. BlockGrowEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/BlockListener.java:397-411`

mcMMO uses `BlockGrowEvent` to:

- Mark naturally grown blocks as eligible for Herbalism XP.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onBlockGrow(BlockGrowEvent event) {
    Block block = event.getBlock();
    if (BlockUtils.isWithinWorldBounds(block)) {
        mcMMO.getUserBlockTracker().setEligible(block);
    }
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/block/block_grow.rs`
**Fired in:** `pumpkin/src/block/blocks/plant/crop/mod.rs:59-71`

```rust
let event = BlockGrowEvent::new(
    world.clone(),
    block,
    state,
    Block::from_state_id(new_state_id),
    new_state_id,
    *pos,
);
let event = server.plugin_manager.fire(event).await;
if event.cancelled {
    return;
}
new_state_id = event.new_state_id;
```

### Required behavior for mcMMO parity

- Fire whenever a block grows naturally.
- Expose the block, old/new state, and position.

### Gaps / action items

- **Limited coverage:** Currently only crop random-tick growth fires this event. mcMMO expects it for saplings, cactus, sugar cane, kelp, bamboo, mushrooms, vines, etc.
- Expand `BlockGrowEvent` firing to all growth paths in Pumpkin.

---

## 15. CreatureSpawnEvent → EntitySpawnEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/EntityListener.java:811-850`

mcMMO uses `CreatureSpawnEvent` to:

- Tag mobs spawned by nether portals, spawners, spawn eggs, etc. with metadata.
- Apply mob health scaling and other flags.

```java
@EventHandler(priority = EventPriority.MONITOR)
public void onCreatureSpawn(CreatureSpawnEvent event) {
    LivingEntity livingEntity = event.getEntity();
    switch (event.getSpawnReason()) {
        case NETHER_PORTAL:
            trackSpawnedAndPassengers(livingEntity, MobMetaFlagType.NETHER_PORTAL_MOB);
            break;
        case SPAWNER:
            // ...
        case EGG:
            // ...
    }
}
```

### Pumpkin implementation

**Definition:** `pumpkin/src/plugin/api/events/entity/entity_spawn.rs`
**Fired in:** `pumpkin/src/world/mod.rs:4309-4319` and `:5814`

```rust
let event = EntitySpawnEvent::new(self.clone(), entity.clone());
let event = self.server.upgrade().expect("server is gone").plugin_manager.fire(event).await;
if event.cancelled {
    self.remove_entity(entity.as_ref()).await;
}
```

### Required behavior for mcMMO parity

- mcMMO needs the spawned entity and its spawn reason.
- Must fire for all living entity spawns.

### Gaps / action items

- Pumpkin's `EntitySpawnEvent` is generic for all entities (not just creatures). ✅ This is acceptable.
- **Spawn reason missing:** mcMMO uses `CreatureSpawnEvent.SpawnReason` (NETHER_PORTAL, SPAWNER, EGG, etc.). Pumpkin does not expose a spawn reason. Add a `spawn_reason: String` or enum field.
- Verify the event fires for player spawns, item drops, projectiles, etc. If mcMMO only cares about living entities, a plugin can filter by entity type.

---

## Step-by-Step Testing Guide

### Setup

1. Build Pumpkin with DLL plugin support enabled (`[lib]` crate exposed, `PLUGIN_API_VERSION` matches).
2. Create a test DLL plugin in a new folder, e.g., `pumpkin/examples/phase1-test-plugin/`.
3. The plugin should register handlers for all 15 Phase 1 events and log each firing to a file.

### Test DLL plugin skeleton

```rust
use std::sync::Arc;
use pumpkin::plugin::{
    api::{context::Context, Plugin},
    EventHandler, EventPriority,
};
use pumpkin::plugin::player::*;
use pumpkin::plugin::block::*;
use pumpkin::plugin::entity::*;

struct Phase1TestPlugin;

impl Plugin for Phase1TestPlugin {
    fn on_load(&mut self, server: Arc<Context>) -> PluginFuture<'_, Result<(), String>> {
        Box::pin(async move {
            server.register_event(Arc::new(PlayerJoinLogger), EventPriority::MONITOR, false).await;
            server.register_event(Arc::new(PlayerLeaveLogger), EventPriority::MONITOR, false).await;
            // ... register all 15 events ...
            Ok(())
        })
    }
}

struct PlayerJoinLogger;
impl EventHandler<PlayerJoinEvent> for PlayerJoinLogger {
    fn handle<'a>(&'a self, _server: &'a Arc<Server>, event: &'a PlayerJoinEvent) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            log_event!("PlayerJoinEvent", "player={}", event.player.gameprofile.name);
        })
    }
}
```

### Manual test script

| Step | Action | Expected event(s) logged |
|------|--------|--------------------------|
| 1 | Start server | `ServerTickStartEvent`, `ServerTickEndEvent` (server events, optional) |
| 2 | Join with a Java client | `PlayerJoinEvent` |
| 3 | Type a chat message | `PlayerChatEvent` |
| 4 | Type `/help` | `PlayerCommandSendEvent` |
| 5 | Left-click a block | `PlayerInteractEvent` with `LeftClickBlock` |
| 6 | Right-click a block | `PlayerInteractEvent` with `RightClickBlock` |
| 7 | Left-click air | `PlayerInteractEvent` with `LeftClickAir` |
| 8 | Right-click an entity | `PlayerInteractEntityEvent` |
| 9 | Cast and reel a fishing rod | `PlayerFishEvent` with `Fishing` state |
| 10 | Teleport with `/tp` | `PlayerTeleportEvent` |
| 11 | Go through a Nether portal | `PlayerChangeWorldEvent` |
| 12 | Die and respawn | `PlayerRespawnEvent` |
| 13 | Click in inventory | `InventoryClickEvent` |
| 14 | Break a stone block | `BlockBreakEvent` |
| 15 | Place a dirt block | `BlockPlaceEvent` |
| 16 | Wait for wheat to grow | `BlockGrowEvent` |
| 17 | Spawn a pig with an egg | `EntitySpawnEvent` |
| 18 | Disconnect | `PlayerLeaveEvent` |

### Automated smoke test (optional)

Add a Rust integration test under `pumpkin/tests/events.rs` that:

1. Creates a `PluginManager`.
2. Registers a logging handler for each Phase 1 event.
3. Fires each event struct directly.
4. Asserts the handler was called and the event fields are correct.
5. Asserts cancellation is respected for cancellable events.

---

## Sample `output.log`

The following is an example of what a debug plugin would write after performing the test script above.

```text
[2026-07-08T22:54:57Z INFO  phase1_test_plugin] PlayerJoinEvent: player=Steve
[2026-07-08T22:55:02Z INFO  phase1_test_plugin] PlayerChatEvent: player=Steve, message="hello world"
[2026-07-08T22:55:05Z INFO  phase1_test_plugin] PlayerCommandSendEvent: player=Steve, command="help"
[2026-07-08T22:55:08Z INFO  phase1_test_plugin] PlayerInteractEvent: player=Steve, action=LeftClickBlock, pos=Some(BlockPos { x: 10, y: 64, z: -20 })
[2026-07-08T22:55:09Z INFO  phase1_test_plugin] PlayerInteractEvent: player=Steve, action=RightClickBlock, pos=Some(BlockPos { x: 10, y: 64, z: -20 })
[2026-07-08T22:55:10Z INFO  phase1_test_plugin] PlayerInteractEvent: player=Steve, action=LeftClickAir, pos=None
[2026-07-08T22:55:12Z INFO  phase1_test_plugin] PlayerInteractEntityEvent: player=Steve, action=Interact
[2026-07-08T22:55:15Z INFO  phase1_test_plugin] PlayerFishEvent: player=Steve, state=Fishing
[2026-07-08T22:55:18Z INFO  phase1_test_plugin] PlayerFishEvent: player=Steve, state=CaughtFish
[2026-07-08T22:55:22Z INFO  phase1_test_plugin] PlayerTeleportEvent: player=Steve, from=(0.0, 64.0, 0.0), to=(100.0, 64.0, 100.0)
[2026-07-08T22:55:28Z INFO  phase1_test_plugin] PlayerChangeWorldEvent: player=Steve, from=overworld, to=the_nether
[2026-07-08T22:55:35Z INFO  phase1_test_plugin] PlayerRespawnEvent: player=Steve, world=overworld, pos=(0.0, 64.0, 0.0)
[2026-07-08T22:55:40Z INFO  phase1_test_plugin] InventoryClickEvent: player=Steve, slot=21, click_type=Left
[2026-07-08T22:55:45Z INFO  phase1_test_plugin] BlockBreakEvent: player=Steve, block=stone, pos=BlockPos { x: 12, y: 64, z: -18 }
[2026-07-08T22:55:47Z INFO  phase1_test_plugin] BlockPlaceEvent: player=Steve, block=dirt, pos=BlockPos { x: 12, y: 64, z: -18 }
[2026-07-08T22:56:10Z INFO  phase1_test_plugin] BlockGrowEvent: block=wheat, old_age=3, new_age=4, pos=BlockPos { x: 20, y: 64, z: 5 }
[2026-07-08T22:56:15Z INFO  phase1_test_plugin] EntitySpawnEvent: entity=pig, spawn_reason=SpawnEgg
[2026-07-08T22:56:20Z INFO  phase1_test_plugin] PlayerLeaveEvent: player=Steve
```

**Note:** The `PlayerFishEvent` `CaughtFish` and `EntitySpawnEvent` `spawn_reason` log lines assume the gaps listed above are filled. Until then, those lines will not appear.

---

## Phase 1 Completion Criteria

Phase 1 is considered complete when:

1. All 15 events above are registered and fire from the documented code paths.
2. A test DLL plugin confirms each event fires during the manual test script.
3. The following gaps are resolved:
   - `PlayerFishEvent` supports all fishing states, not just `Fishing`.
   - `BlockGrowEvent` fires for all growth types (saplings, cactus, sugar cane, kelp, etc.).
   - `EntitySpawnEvent` includes a spawn reason.
   - `PlayerCommandSendEvent` semantics are verified or a `PlayerCommandPreprocessEvent` is added.
   - `PlayerInteractEvent` exposes the hand used.
4. The automated smoke test passes.

---

## References

- Parent plan: [`plan-api.md`](./plan-api.md)
- mcMMO source: `../mcMMO/src/main/java/com/gmail/nossr50/listeners/`
- Pumpkin event definitions: `pumpkin/src/plugin/api/events/`
- Pumpkin event dispatch: `pumpkin/src/plugin/mod.rs`

---

*Document generated for Phase 1 of the Pumpkin / mcMMO event parity effort.*
