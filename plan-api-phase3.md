# Phase 3 — Entity Damage & Death Event Parity Plan

This document is the detailed expansion of **Phase 3** from [`plan-api.md`](./plan-api.md). It covers the combat, death, projectile, and hunger events that mcMMO depends on and that are **not yet exposed** to native DLL plugins in Pumpkin.

**Goal of Phase 3:** implement and fire `EntityDamageEvent`, `EntityDamageByEntityEvent`, `EntityDeathEvent`, `PlayerDeathEvent`, `FoodLevelChangeEvent`, `ProjectileLaunchEvent`, and `ProjectileHitEvent` so that the mcMMO combat skills (Unarmed, Swords, Axes, Archery), Taming, Acrobatics, and Herbalism diet can be ported.

---

## Phase 3 Event Checklist

| # | Bukkit/Spigot event (mcMMO) | Pumpkin event | Status |
|---|-----------------------------|---------------|--------|
| 1 | `EntityDamageEvent` | `EntityDamageEvent` | ❌ Not implemented |
| 2 | `EntityDamageByEntityEvent` | `EntityDamageByEntityEvent` | ❌ Not implemented |
| 3 | `EntityDeathEvent` | `EntityDeathEvent` | ❌ Not implemented |
| 4 | `PlayerDeathEvent` | `PlayerDeathEvent` | ❌ Not implemented |
| 5 | `FoodLevelChangeEvent` | `FoodLevelChangeEvent` | ❌ Not implemented |
| 6 | `ProjectileLaunchEvent` | `ProjectileLaunchEvent` | ❌ Not implemented |
| 7 | `ProjectileHitEvent` | `ProjectileHitEvent` | ❌ Not implemented |

---

## 1. EntityDamageEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/EntityListener.java:581-754`

mcMMO uses `EntityDamageEvent` at **HIGHEST** priority to:

- Process registered interactions (`InteractionManager.processEvent`).
- Skip NPCs and invalid entities.
- Handle player-specific logic: god mode, recently-hurt timestamp.
- Handle tamed pet environmental damage/fall damage (Taming skill).
- Apply Acrobatics roll/dodge for fall damage.

```java
@EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
public void onEntityDamage(EntityDamageEvent event) {
    /* WORLD BLACKLIST CHECK */
    if (WorldBlacklist.isWorldBlacklisted(event.getEntity().getWorld())) {
        return;
    }

    InteractionManager.processEvent(event, pluginRef, InteractType.ON_ENTITY_DAMAGE);

    if (event.getEntity() instanceof LivingEntity livingEntity) {
        if (CombatUtils.hasIgnoreDamageMetadata(livingEntity)) {
            return;
        }
    }

    double damage = event.getFinalDamage();
    if (damage <= 0) return;

    Entity entity = event.getEntity();
    if (!entity.isValid() || !(entity instanceof LivingEntity livingEntity)) return;

    DamageCause cause = event.getCause();

    if (livingEntity instanceof Player player) {
        final McMMOPlayer mmoPlayer = UserManager.getPlayer(player);
        if (mmoPlayer.getGodMode()) {
            event.setCancelled(true);
            return;
        }
        if (event.getFinalDamage() >= 1) {
            mmoPlayer.actualizeRecentlyHurt();
        }
    } else if (livingEntity instanceof Tameable pet) {
        // Taming skill environmental/fall protection
    }

    // Acrobatics fall damage roll/dodge
    if (cause == DamageCause.FALL && entity instanceof Player player) {
        final McMMOPlayer mmoPlayer = UserManager.getPlayer(player);
        if (mmoPlayer != null) {
            AcrobaticsManager acrobaticsManager = mmoPlayer.getAcrobaticsManager();
            if (!acrobaticsManager.hasFallen()) {
                event.setDamage(EntityDamageEvent.DamageModifier.BASE, 0);
                event.setCancelled(acrobaticsManager.rollCheck(event.getDamage()));
            }
        }
    }
}
```

Key fields mcMMO reads:

- `event.getEntity()` — the entity taking damage.
- `event.getCause()` — `DamageCause` enum (FALL, ENTITY_ATTACK, FIRE, etc.).
- `event.getDamage()` / `event.getFinalDamage()` — raw and final damage.
- `event.setDamage(...)` / `event.setCancelled(...)` — Acrobatics roll/dodge.

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/entity/entity_damage.rs`

Event shape:

```rust
use pumpkin_data::damage::DamageType;
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

use crate::entity::EntityBase;

/// Fired when a living entity is damaged.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityDamageEvent {
    /// The entity being damaged.
    pub entity: Arc<dyn EntityBase>,
    /// The type of damage (fall, fire, entity attack, etc.).
    pub damage_type: DamageType,
    /// The raw damage amount before reductions/absorption.
    pub damage: f32,
    /// The final damage that will be applied after this event.
    pub final_damage: f32,
}

impl EntityDamageEvent {
    pub fn new(
        entity: Arc<dyn EntityBase>,
        damage_type: DamageType,
        damage: f32,
        final_damage: f32,
    ) -> Self {
        Self {
            entity,
            damage_type,
            damage,
            final_damage,
            cancelled: false,
        }
    }
}
```

**Register module:** add `pub mod entity_damage;` and `pub use entity_damage::EntityDamageEvent;` to `pumpkin/src/plugin/api/events/entity/mod.rs`.

**Fire the event:** `pumpkin/src/entity/living.rs:1974-2328`, inside `damage_with_context`.

Insert after `effective_amount` is computed and shield blocking is checked, but **before** the hurt cooldown logic and health modification:

```rust
// Total damage after reductions
let effective_amount = amount * (1.0 - resistance_reduction);

// --- NEW: EntityDamageEvent ---
let damage_event = EntityDamageEvent::new(
    caller_arc.clone(), // Arc<dyn EntityBase> of the victim
    damage_type,
    amount,
    effective_amount,
);
let server = world.server.upgrade().unwrap();
let damage_event = server.plugin_manager.fire(damage_event).await;

if damage_event.cancelled {
    return false;
}

let amount = damage_event.damage;
let effective_amount = damage_event.final_damage;
// --- END NEW ---
```

Because `caller` is `&dyn EntityBase`, you will need an `Arc<dyn EntityBase>` clone. The `EntityBase` trait is implemented for entity wrapper types (e.g., `Arc<ZombieEntity>`). If the caller already has an `Arc`, pass a clone. If not, add an `Arc::clone` from the stored world entity lookup.

### Required behavior for mcMMO parity

- Must fire for all living entities taking damage.
- Must expose the victim, damage type, raw damage, and final damage.
- Must be cancellable (god mode, Acrobatics dodge).
- Must allow plugins to mutate `damage` and `final_damage`.

### Gaps / action items

- mcMMO uses Bukkit's `DamageCause` enum. Pumpkin uses `DamageType`. Provide a mapping or expose both the Pumpkin `DamageType` and a simplified `DamageCause` enum for plugin portability.
- Shield blocking currently returns `false` before the event would fire. Decide whether to fire the event before or after shield blocking. mcMMO wants to see damage that was not blocked, so firing **after** shield blocking is acceptable.

---

## 2. EntityDamageByEntityEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/EntityListener.java:321-456`

mcMMO uses `EntityDamageByEntityEvent` at **HIGHEST** priority to:

- Skip ignored-damage metadata and invincible entities.
- Handle Blast Mining explosions (TNT primed by a player).
- Run party/same-player friendly-fire checks.
- Run Unarmed arrow deflect.
- Call `CombatUtils.processCombatAttack` for Swords, Axes, Unarmed, Archery, Taming bonuses.

```java
@EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
public void onEntityDamageByEntity(EntityDamageByEntityEvent event) {
    if (event.getEntity() instanceof LivingEntity livingEntity) {
        if (CombatUtils.hasIgnoreDamageMetadata(livingEntity)) return;
    }

    double damage = event.getFinalDamage();
    Entity defender = event.getEntity();
    Entity attacker = event.getDamager();

    // TNT Blast Mining
    if (attacker instanceof TNTPrimed tntAttacker && defender instanceof Player) {
        if (BlastMining.processBlastMiningExplosion(event, tntAttacker, (Player) defender)) {
            return;
        }
    }

    // Friendly fire checks
    if (defender instanceof Player defendingPlayer) {
        if (attacker instanceof Projectile projectile) {
            if (projectile.getShooter() instanceof Player attackingPlayer) {
                if (checkIfInPartyOrSamePlayer(event, defendingPlayer, attackingPlayer)) return;
                // Unarmed deflect
                if (unarmedManager.canDeflect() && projectile instanceof Arrow) {
                    event.setCancelled(true);
                    return;
                }
            }
        } else if (attacker instanceof Player attackingPlayer) {
            if (checkIfInPartyOrSamePlayer(event, defendingPlayer, attackingPlayer)) return;
        }
    }

    // Resolve projectile shooter to the living attacker
    if (attacker instanceof Projectile projectile) {
        ProjectileSource shooter = projectile.getShooter();
        if (shooter instanceof LivingEntity) {
            attacker = (LivingEntity) shooter;
        }
    }

    CombatUtils.processCombatAttack(event, attacker, target);
    CombatUtils.handleHealthbars(attacker, target, event.getFinalDamage(), pluginRef);
}
```

Key fields mcMMO reads:

- `event.getEntity()` — defender.
- `event.getDamager()` — direct attacker (may be a projectile).
- `event.getFinalDamage()` / `event.getDamage()` — mutable damage.
- `event.setCancelled(...)`.

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/entity/entity_damage_by_entity.rs`

Event shape:

```rust
use pumpkin_data::damage::DamageType;
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

use crate::entity::EntityBase;

/// Fired when a living entity is damaged by another entity.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityDamageByEntityEvent {
    /// The entity being damaged.
    pub entity: Arc<dyn EntityBase>,
    /// The direct damager (e.g., a zombie, an arrow, a TNT entity).
    pub damager: Arc<dyn EntityBase>,
    /// The underlying attacker if `damager` is a projectile (e.g., the player who shot the arrow).
    pub attacker: Option<Arc<dyn EntityBase>>,
    /// The damage type.
    pub damage_type: DamageType,
    /// Raw damage.
    pub damage: f32,
    /// Final damage to apply.
    pub final_damage: f32,
}

impl EntityDamageByEntityEvent {
    pub fn new(
        entity: Arc<dyn EntityBase>,
        damager: Arc<dyn EntityBase>,
        attacker: Option<Arc<dyn EntityBase>>,
        damage_type: DamageType,
        damage: f32,
        final_damage: f32,
    ) -> Self {
        Self {
            entity,
            damager,
            attacker,
            damage_type,
            damage,
            final_damage,
            cancelled: false,
        }
    }
}
```

**Register module:** add `pub mod entity_damage_by_entity;` and `pub use entity_damage_by_entity::EntityDamageByEntityEvent;` to `pumpkin/src/plugin/api/events/entity/mod.rs`.

**Fire the event:** `pumpkin/src/entity/living.rs:1974-2328`, inside `damage_with_context`.

This event should be fired **instead of** `EntityDamageEvent` when `source` or `cause` is another entity. The simplest approach is:

1. Compute `effective_amount`.
2. Resolve `damager` (direct source entity) and `attacker` (underlying living attacker).
3. If there is an entity damager, fire `EntityDamageByEntityEvent`; otherwise fire `EntityDamageEvent`.
4. Apply the mutated damage/cancellation.

```rust
let (damager, attacker) = resolve_damager(source, cause);

let event = if let Some(damager) = damager.clone() {
    server.plugin_manager.fire(EntityDamageByEntityEvent::new(
        victim_arc.clone(),
        damager,
        attacker.clone(),
        damage_type,
        amount,
        effective_amount,
    )).await
} else {
    server.plugin_manager.fire(EntityDamageEvent::new(
        victim_arc.clone(),
        damage_type,
        amount,
        effective_amount,
    )).await
};

if event.cancelled {
    return false;
}
```

Because Rust events are strongly typed, you cannot use a single `event` variable for both types. Use an `if/else` and then continue with the resolved `amount`/`effective_amount`.

### Required behavior for mcMMO parity

- Must fire when the direct damage source is an entity.
- Must expose defender, direct damager, underlying attacker, damage type, and damage values.
- Must allow cancellation and damage mutation.

### Gaps / action items

- `source` vs `cause` semantics in Pumpkin:
  - `source` = direct source (e.g., arrow entity).
  - `cause` = underlying cause (e.g., player who shot the arrow).
  - mcMMO's `getDamager()` maps to `source`; the shooter resolution maps to `cause` when it is a living entity.
- Friendly-fire and deflect checks should be done by the plugin, not the server.

---

## 3. EntityDeathEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/EntityListener.java:757-804`

mcMMO registers **two** death handlers:

1. **LOWEST priority** (`onEntityDeathLowest`): clean transient metadata for slimes/magma cubes.
2. **Default priority** (`onEntityDeath`): handle transient entity tracker cleanup and Archery arrow retrieval.

```java
@EventHandler(priority = EventPriority.LOWEST)
public void onEntityDeathLowest(EntityDeathEvent event) {
    final LivingEntity entity = event.getEntity();
    if (TRANSFORMABLE_ENTITIES.contains(entity.getType())) {
        return;
    }
    mcMMO.getTransientMetadataTools().cleanLivingEntityMetadata(entity);
}

@EventHandler
public void onEntityDeath(EntityDeathEvent event) {
    final LivingEntity entity = event.getEntity();
    if (mcMMO.getTransientEntityTracker().isTransient(entity)) {
        mcMMO.getTransientEntityTracker().killSummonAndCleanMobFlags(entity, null, false);
    }
    if (WorldBlacklist.isWorldBlacklisted(event.getEntity().getWorld())) return;
    if (ExperienceConfig.getInstance().isNPCInteractionPrevented()
            && Misc.isNPCEntityExcludingVillagers(entity)) return;
    Archery.arrowRetrievalCheck(entity);
}
```

Key fields mcMMO reads:

- `event.getEntity()` — the dead living entity.
- `event.getDrops()` — mutable item drops.
- `event.getDroppedExp()` — mutable dropped XP.

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/entity/entity_death.rs`

Event shape:

```rust
use pumpkin_data::damage::DamageType;
use pumpkin_macros::Event;
use std::sync::Arc;

use crate::entity::EntityBase;
use crate::item::ItemStack;

/// Fired when a living entity dies.
#[derive(Event, Clone)]
pub struct EntityDeathEvent {
    pub entity: Arc<dyn EntityBase>,
    pub damage_type: DamageType,
    pub killer: Option<Arc<dyn EntityBase>>,
    pub drops: Vec<ItemStack>,
    pub dropped_exp: i32,
}

impl EntityDeathEvent {
    pub fn new(
        entity: Arc<dyn EntityBase>,
        damage_type: DamageType,
        killer: Option<Arc<dyn EntityBase>>,
        drops: Vec<ItemStack>,
        dropped_exp: i32,
    ) -> Self {
        Self {
            entity,
            damage_type,
            killer,
            drops,
            dropped_exp,
        }
    }
}
```

**Register module:** add `pub mod entity_death;` and `pub use entity_death::EntityDeathEvent;` to `pumpkin/src/plugin/api/events/entity/mod.rs`.

**Fire the event:** `pumpkin/src/entity/living.rs:1298-1378`, inside `on_death`.

Insert after computing loot and XP but before dropping them:

```rust
// Drop loot
let mut drops = Vec::new();
self.drop_loot(params.clone()).await; // currently drops immediately; refactor to return Vec<ItemStack>

let dropped_exp = if params.killed_by_player.unwrap_or(false) && world.level_info.load().game_rules.mob_drops {
    dyn_self.get_experience_reward(cause)
} else {
    0
};

// --- NEW: EntityDeathEvent ---
let killer = cause.map(|c| Arc::clone(&world.get_entity_by_id(c.get_entity().entity_id).unwrap()));
let death_event = EntityDeathEvent::new(
    dyn_self.clone(),
    damage_type,
    killer,
    drops,
    dropped_exp as i32,
);
let death_event = server.plugin_manager.fire(death_event).await;

// Apply mutated drops and XP
for stack in death_event.drops {
    world.drop_stack(&block_pos, stack).await;
}
if death_event.dropped_exp > 0 {
    ExperienceOrbEntity::spawn(&world, self.entity.pos.load(), death_event.dropped_exp as u32).await;
}
// --- END NEW ---
```

This requires refactoring `drop_loot` (and the subclass overrides) to return `Vec<ItemStack>` instead of spawning item entities directly.

### Required behavior for mcMMO parity

- Must fire when any living entity dies.
- Must expose the entity, killer, drops, and dropped XP.
- Should allow plugins to mutate drops and XP.
- Should be non-cancellable (Bukkit's is not cancellable either).

### Gaps / action items

- Refactor `LivingEntity::drop_loot` to return `Vec<ItemStack>` instead of spawning entities.
- `PlayerDeathEvent` (below) is a specialization; fire it for players and also fire `EntityDeathEvent`, or make `PlayerDeathEvent` include the same fields and fire only that for players. Bukkit fires both; for mcMMO parity, fire both.

---

## 4. PlayerDeathEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java:192-249`

mcMMO registers **two** player death handlers:

1. **MONITOR priority** (`onPlayerDeathMonitor`): apply hardcore stat loss and vampirism.
2. **NORMAL priority** (`onPlayerDeathNormal`): remove ability buffs from inventory.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onPlayerDeathMonitor(PlayerDeathEvent event) {
    Player killedPlayer = event.getEntity();
    Player killer = killedPlayer.getKiller();

    if (HardcoreManager.isStatLossEnabled() || HardcoreManager.isVampirismEnabled()) {
        if (statLossEnabled) HardcoreManager.invokeStatPenalty(killedPlayer);
        if (killer != null && vampirismEnabled) HardcoreManager.invokeVampirism(killer, killedPlayer);
    }
}

@EventHandler(priority = EventPriority.NORMAL, ignoreCancelled = false)
public void onPlayerDeathNormal(PlayerDeathEvent event) {
    SkillUtils.removeAbilityBoostsFromInventory(playerDeathEvent.getEntity());
}
```

Key fields mcMMO reads:

- `event.getEntity()` — dead player.
- `event.getKiller()` — the player/mob credited with the kill.
- `event.getDrops()` — mutable drops.
- `event.getDroppedExp()` — mutable dropped XP.
- `event.setKeepInventory(...)` / `event.setKeepLevel(...)`.

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/player/player_death.rs`

Event shape:

```rust
use pumpkin_data::damage::DamageType;
use pumpkin_macros::Event;
use std::sync::Arc;

use crate::entity::player::Player;
use crate::entity::EntityBase;
use crate::item::ItemStack;
use super::PlayerEvent;

/// Fired when a player dies.
#[derive(Event, Clone)]
pub struct PlayerDeathEvent {
    pub player: Arc<Player>,
    pub damage_type: DamageType,
    pub killer: Option<Arc<dyn EntityBase>>,
    pub drops: Vec<ItemStack>,
    pub dropped_exp: i32,
    pub keep_inventory: bool,
    pub keep_level: bool,
}

impl PlayerDeathEvent {
    pub fn new(
        player: Arc<Player>,
        damage_type: DamageType,
        killer: Option<Arc<dyn EntityBase>>,
        drops: Vec<ItemStack>,
        dropped_exp: i32,
    ) -> Self {
        Self {
            player,
            damage_type,
            killer,
            drops,
            dropped_exp,
            keep_inventory: false,
            keep_level: false,
        }
    }
}

impl PlayerEvent for PlayerDeathEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
```

**Register module:** add `pub mod player_death;` to `pumpkin/src/plugin/api/events/player/mod.rs`.

**Fire the event:** `pumpkin/src/entity/living.rs:1298-1378`, inside `on_death`, when the entity is a player.

Add at the same location as `EntityDeathEvent`, branching for players:

```rust
if self.entity.entity_type == &EntityType::PLAYER {
    if let Some(player) = dyn_self.cast_any().downcast_ref::<Player>() {
        let player_arc = Arc::new(player.clone()); // or fetch from world
        let player_death_event = PlayerDeathEvent::new(
            player_arc,
            damage_type,
            killer.clone(),
            drops.clone(),
            dropped_exp as i32,
        );
        let player_death_event = server.plugin_manager.fire(player_death_event).await;
        drops = player_death_event.drops;
        dropped_exp = player_death_event.dropped_exp as u32;
        keep_inventory = player_death_event.keep_inventory;
        keep_level = player_death_event.keep_level;
    }
}
```

**Note:** Player death handling in Pumpkin may be spread across `Player::on_death` or the respawn code. Verify the exact player death path and fire the event there.

### Required behavior for mcMMO parity

- Must fire when a player dies.
- Must expose player, killer, drops, dropped XP.
- Must allow `keep_inventory` and `keep_level` to be toggled.

### Gaps / action items

- Determine whether Pumpkin currently drops player items/XP on death. If not, implement that first.
- Respect `keep_inventory` and `keep_level` in the respawn logic.

---

## 5. FoodLevelChangeEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/EntityListener.java:972-1070`

mcMMO uses `FoodLevelChangeEvent` at **LOW** priority to:

- Apply Herbalism's Farmer's Diet subskill, which increases food restoration for plant-based foods.

```java
@EventHandler(priority = EventPriority.LOW, ignoreCancelled = true)
public void onFoodLevelChange(FoodLevelChangeEvent event) {
    Entity entity = event.getEntity();
    if (!(entity instanceof Player player)) return;
    if (UserManager.getPlayer(player) == null) return;

    int currentFoodLevel = player.getFoodLevel();
    int newFoodLevel = event.getFoodLevel();
    int foodChange = newFoodLevel - currentFoodLevel;
    if (foodChange <= 0) return;

    // Determine food in hand
    Material foodInHand;
    if (mcMMO.getMaterialMapStore().isFood(player.getInventory().getItemInMainHand().getType())) {
        foodInHand = player.getInventory().getItemInMainHand().getType();
    } else if (mcMMO.getMaterialMapStore().isFood(player.getInventory().getItemInOffHand().getType())) {
        foodInHand = player.getInventory().getItemInOffHand().getType();
    } else {
        return;
    }

    if (Permissions.isSubSkillEnabled(player, SubSkillType.HERBALISM_FARMERS_DIET)) {
        event.setFoodLevel(UserManager.getPlayer(player).getHerbalismManager().farmersDiet(newFoodLevel));
    }
}
```

Key fields mcMMO reads:

- `event.getEntity()` — the player.
- `event.getFoodLevel()` — new food level.
- `event.setFoodLevel(...)` — modify restoration.

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/player/food_level_change.rs`

Event shape:

```rust
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

use crate::entity::player::Player;
use super::PlayerEvent;

/// Fired when a player's food level changes.
#[cancellable]
#[derive(Event, Clone)]
pub struct FoodLevelChangeEvent {
    pub player: Arc<Player>,
    pub food_level: u8,
}

impl FoodLevelChangeEvent {
    pub fn new(player: Arc<Player>, food_level: u8) -> Self {
        Self {
            player,
            food_level,
            cancelled: false,
        }
    }
}

impl PlayerEvent for FoodLevelChangeEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}
```

**Register module:** add `pub mod food_level_change;` to `pumpkin/src/plugin/api/events/player/mod.rs`.

**Fire the event:** `pumpkin/src/entity/hunger.rs:49-56`, inside `HungerManager::tick`.

Replace the direct hunger modification with an event:

```rust
if exhaustion > EXHAUSTION_COST {
    exhaustion -= EXHAUSTION_COST;
    if saturation > 0.0 {
        saturation = (saturation - 1.0).max(0.0);
    } else if difficulty != Difficulty::Peaceful {
        let new_level = level.saturating_sub(1);
        let event = FoodLevelChangeEvent::new(player.clone(), new_level);
        let server = player.world().server.upgrade().unwrap();
        let event = server.plugin_manager.fire(event).await;
        if !event.cancelled {
            level = event.food_level;
        }
    }
    needs_sync = true;
}
```

Also fire when eating in `HungerManager::eat`:

```rust
let event = FoodLevelChangeEvent::new(player_arc.clone(), new_level);
let event = server.plugin_manager.fire(event).await;
if !event.cancelled {
    self.level.store(event.food_level);
    player.send_health().await;
}
```

### Required behavior for mcMMO parity

- Must fire when hunger increases (eating) or decreases (exhaustion).
- Must expose player and new food level.
- Must allow mutation and cancellation.

### Gaps / action items

- Decide whether the event should fire for saturation changes too. mcMMO only cares about `foodLevel`, so start with that.
- The `eat` method currently updates food synchronously; refactor to async so the event can be awaited.

---

## 6. ProjectileLaunchEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/EntityListener.java:191-236`

mcMMO uses `ProjectileLaunchEvent` at **MONITOR** priority to:

- Set arrow metadata (`BOW_FORCE`, `ARROW_DISTANCE`) for Archery skill calculations.
- Apply Arrow Retrieval metadata based on RNG.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onProjectileLaunch(ProjectileLaunchEvent event) {
    if (WorldBlacklist.isWorldBlacklisted(event.getEntity().getWorld())) return;

    if (event.getEntity().getShooter() instanceof Player player) {
        if (event.getEntity() instanceof Arrow arrow) {
            CombatUtils.delayArrowMetaCleanup(arrow);

            if (!arrow.hasMetadata(METADATA_KEY_BOW_FORCE)) {
                arrow.setMetadata(METADATA_KEY_BOW_FORCE, new FixedMetadataValue(pluginRef, 1.0));
            }
            if (!arrow.hasMetadata(METADATA_KEY_ARROW_DISTANCE)) {
                arrow.setMetadata(METADATA_KEY_ARROW_DISTANCE, new FixedMetadataValue(pluginRef, arrow.getLocation()));
            }

            if (ItemUtils.doesPlayerHaveEnchantmentInHands(player, PIERCING)) return;

            if (ProbabilityUtil.isSkillRNGSuccessful(SubSkillType.ARCHERY_ARROW_RETRIEVAL, UserManager.getPlayer(player))) {
                arrow.setMetadata(METADATA_KEY_TRACKED_ARROW, MetadataConstants.MCMMO_METADATA_VALUE);
            }
        }
    }
}
```

Key fields mcMMO reads:

- `event.getEntity()` — the launched projectile.
- `event.getEntity().getShooter()` — the shooter (usually a player).

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/entity/projectile_launch.rs`

Event shape:

```rust
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

use crate::entity::EntityBase;

/// Fired when a projectile is launched.
#[cancellable]
#[derive(Event, Clone)]
pub struct ProjectileLaunchEvent {
    pub projectile: Arc<dyn EntityBase>,
    pub shooter: Option<Arc<dyn EntityBase>>,
}

impl ProjectileLaunchEvent {
    pub fn new(projectile: Arc<dyn EntityBase>, shooter: Option<Arc<dyn EntityBase>>) -> Self {
        Self {
            projectile,
            shooter,
            cancelled: false,
        }
    }
}
```

**Register module:** add `pub mod projectile_launch;` and `pub use projectile_launch::ProjectileLaunchEvent;` to `pumpkin/src/plugin/api/events/entity/mod.rs`.

**Fire the event:** After every projectile spawn in Pumpkin. Key locations:

- `pumpkin/src/item/items/bow.rs:197-198` — bow arrow spawn.
- `pumpkin/src/item/items/crossbow.rs` — crossbow arrows.
- `pumpkin/src/item/items/trident.rs` — thrown trident.
- `pumpkin/src/item/items/snowball.rs`, `egg.rs`, etc. — thrown items.
- `pumpkin/src/entity/projectile/egg.rs:146` (already has `PlayerEggThrowEvent`; add `ProjectileLaunchEvent` nearby).

Example insertion in `bow.rs`:

```rust
let arrow_arc: Arc<dyn EntityBase> = Arc::new(arrow);

// --- NEW: ProjectileLaunchEvent ---
let launch_event = ProjectileLaunchEvent::new(
    arrow_arc.clone(),
    Some(Arc::clone(player) as Arc<dyn EntityBase>),
);
let launch_event = world.server.upgrade().unwrap().plugin_manager.fire(launch_event).await;
if launch_event.cancelled {
    return;
}
// --- END NEW ---

world.spawn_entity(arrow_arc).await;
```

### Required behavior for mcMMO parity

- Must fire when any projectile is spawned.
- Must expose projectile and shooter.
- Must be cancellable.

### Gaps / action items

- Pumpkin does not store a `shooter` reference on all projectiles. Bow arrows already store `owner_id`; ensure all projectiles store an `owner_id` or `shooter` arc.
- Fire the event from every projectile-launching code path.

---

## 7. ProjectileHitEvent

### mcMMO usage

**File:** `mcMMO/src/main/java/com/gmail/nossr50/listeners/EntityListener.java:1212-1232`

mcMMO uses `ProjectileHitEvent` at **MONITOR** priority to:

- Process Crossbows skill effects when a crossbow arrow hits.

```java
@EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
public void onProjectileHitEvent(ProjectileHitEvent event) {
    if (WorldBlacklist.isWorldBlacklisted(event.getEntity().getWorld())) return;

    if (event.getEntity() instanceof Arrow arrow) {
        if (arrow.isShotFromCrossbow()) {
            Crossbows.processCrossbows(event, pluginRef, arrow);
        }
    }
}
```

Key fields mcMMO reads:

- `event.getEntity()` — the projectile.
- `event.getHitEntity()` — entity that was hit (if any).
- `event.getHitBlock()` — block that was hit (if any).

### Pumpkin implementation plan

**New file:** `pumpkin/src/plugin/api/events/entity/projectile_hit.rs`

Event shape:

```rust
use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

use crate::entity::EntityBase;

/// Fired when a projectile hits an entity or block.
#[cancellable]
#[derive(Event, Clone)]
pub struct ProjectileHitEvent {
    pub projectile: Arc<dyn EntityBase>,
    pub hit_entity: Option<Arc<dyn EntityBase>>,
    pub hit_block: Option<&'static Block>,
    pub hit_block_pos: Option<BlockPos>,
}

impl ProjectileHitEvent {
    pub fn new(
        projectile: Arc<dyn EntityBase>,
        hit_entity: Option<Arc<dyn EntityBase>>,
        hit_block: Option<&'static Block>,
        hit_block_pos: Option<BlockPos>,
    ) -> Self {
        Self {
            projectile,
            hit_entity,
            hit_block,
            hit_block_pos,
            cancelled: false,
        }
    }
}
```

**Register module:** add `pub mod projectile_hit;` and `pub use projectile_hit::ProjectileHitEvent;` to `pumpkin/src/plugin/api/events/entity/mod.rs`.

**Fire the event:** In each projectile's `on_hit` implementation.

Key locations:

- `pumpkin/src/entity/projectile/arrow.rs:348` — arrow hit.
- `pumpkin/src/entity/projectile/trident.rs` — trident hit.
- `pumpkin/src/entity/projectile/ender_pearl.rs:75` — ender pearl hit.
- `pumpkin/src/entity/projectile/snowball.rs` — snowball hit.
- `pumpkin/src/entity/projectile/egg.rs` — egg hit.

Example insertion in `arrow.rs::on_hit`:

```rust
fn on_hit(&self, hit: ProjectileHit) -> EntityBaseFuture<'_, ()> {
    Box::pin(async move {
        let entity = self.get_entity();
        let world = entity.world.load();

        let (hit_entity, hit_block, hit_block_pos) = match &hit {
            ProjectileHit::Entity(e) => (Some(Arc::clone(e)), None, None),
            ProjectileHit::Block(pos, block) => (None, Some(*block), Some(*pos)),
        };

        // --- NEW: ProjectileHitEvent ---
        let hit_event = ProjectileHitEvent::new(
            caller_arc.clone(), // the arrow as Arc<dyn EntityBase>
            hit_entity.clone(),
            hit_block,
            hit_block_pos,
        );
        let server = world.server.upgrade().unwrap();
        let hit_event = server.plugin_manager.fire(hit_event).await;
        if hit_event.cancelled {
            return;
        }
        // --- END NEW ---

        // existing hit handling ...
    })
}
```

### Required behavior for mcMMO parity

- Must fire when a projectile hits an entity or block.
- Must expose projectile, hit entity, and hit block/position.
- Must be cancellable.

### Gaps / action items

- Ensure all projectiles have a centralized `on_hit` or `process_tick` collision path where this event can be fired.
- `ProjectileHit` enum must expose both entity and block variants.

---

## Implementation Order Within Phase 3

1. **EntityDamageEvent** and **EntityDamageByEntityEvent** — core combat system; unlock all melee/ranged skills.
2. **EntityDeathEvent** and **PlayerDeathEvent** — unlock combat XP, hardcore mode, arrow retrieval.
3. **FoodLevelChangeEvent** — unlock Herbalism Farmer's Diet.
4. **ProjectileLaunchEvent** and **ProjectileHitEvent** — unlock Archery and Crossbows.

---

## Step-by-Step Testing Guide

### Setup

1. Build Pumpkin with the new events.
2. Create a test DLL plugin that registers handlers for all 7 Phase 3 events and logs each firing.
3. Ensure the plugin can cancel events and mutate damage/food level/drops.

### Manual test script

| Step | Action | Expected event(s) logged |
|------|--------|--------------------------|
| 1 | Fall from a height | `EntityDamageEvent: cause=Fall` |
| 2 | Punch a zombie | `EntityDamageByEntityEvent: damager=player, entity=zombie` |
| 3 | Shoot a zombie with a bow | `EntityDamageByEntityEvent: damager=arrow, attacker=player` |
| 4 | Cancel `EntityDamageByEntityEvent` and punch a zombie | No damage dealt |
| 5 | Mutate `EntityDamageEvent` final_damage to 0 and fall | No damage taken |
| 6 | Kill a zombie | `EntityDeathEvent: entity=zombie, killer=player` |
| 7 | Die as a player | `PlayerDeathEvent: player=Steve` |
| 8 | Cancel `EntityDeathEvent` drops | Zombie drops no items |
| 9 | Eat food | `FoodLevelChangeEvent: food_level=...` |
| 10 | Cancel `FoodLevelChangeEvent` and eat | Food level does not increase |
| 11 | Shoot an arrow | `ProjectileLaunchEvent: projectile=arrow, shooter=player` |
| 12 | Cancel `ProjectileLaunchEvent` and shoot | Arrow does not spawn |
| 13 | Hit an entity/block with an arrow | `ProjectileHitEvent: projectile=arrow, hit_entity=...` |
| 14 | Cancel `ProjectileHitEvent` and shoot | Arrow passes through target |

### Automated test

Add a Rust test in `pumpkin/tests/phase3_events.rs` that:

1. Creates a `PluginManager` and mock entities.
2. Fires each event directly.
3. Verifies handlers receive the correct event data.
4. Verifies cancellation and mutation are respected.

---

## Sample `output.log`

```text
[2026-07-08T23:05:01Z INFO  phase3_test_plugin] EntityDamageEvent: entity=Steve, cause=Fall, damage=5.0, final_damage=5.0
[2026-07-08T23:05:03Z INFO  phase3_test_plugin] EntityDamageByEntityEvent: entity=zombie, damager=Steve, attacker=Steve, damage=4.5, final_damage=4.5
[2026-07-08T23:05:05Z INFO  phase3_test_plugin] ProjectileLaunchEvent: projectile=arrow, shooter=Steve
[2026-07-08T23:05:06Z INFO  phase3_test_plugin] ProjectileHitEvent: projectile=arrow, hit_entity=zombie
[2026-07-08T23:05:06Z INFO  phase3_test_plugin] EntityDamageByEntityEvent: entity=zombie, damager=arrow, attacker=Steve, damage=9.0, final_damage=9.0
[2026-07-08T23:05:08Z INFO  phase3_test_plugin] EntityDeathEvent: entity=zombie, killer=Steve, dropped_exp=5
[2026-07-08T23:05:12Z INFO  phase3_test_plugin] PlayerDeathEvent: player=Steve, killer=zombie
[2026-07-08T23:05:18Z INFO  phase3_test_plugin] FoodLevelChangeEvent: player=Steve, food_level=17
```

---

## Phase 3 Completion Criteria

Phase 3 is complete when:

1. All 7 events are defined, registered, and fire from the documented code paths.
2. A test DLL plugin confirms each event fires during the manual test script.
3. Damage events allow cancellation and mutation of damage values.
4. Death events expose drops and dropped XP, and allow plugins to mutate them.
5. `PlayerDeathEvent` supports `keep_inventory` and `keep_level`.
6. `FoodLevelChangeEvent` fires on both hunger loss and eating, and allows mutation/cancellation.
7. Projectile events fire for arrows, tridents, snowballs, eggs, ender pearls, and crossbow projectiles.
8. The automated smoke test passes.

---

## References

- Parent plan: [`plan-api.md`](./plan-api.md)
- Phase 1 detail: [`plan-api-phase1.md`](./plan-api-phase1.md)
- Phase 2 detail: [`plan-api-phase2.md`](./plan-api-phase2.md)
- mcMMO source: `../mcMMO/src/main/java/com/gmail/nossr50/listeners/EntityListener.java`, `../mcMMO/src/main/java/com/gmail/nossr50/listeners/PlayerListener.java`
- Pumpkin event definitions: `pumpkin/src/plugin/api/events/entity/`, `pumpkin/src/plugin/api/events/player/`
- Pumpkin damage pipeline: `pumpkin/src/entity/living.rs:1974-2328`
- Pumpkin death pipeline: `pumpkin/src/entity/living.rs:1298-1378`
- Pumpkin hunger manager: `pumpkin/src/entity/hunger.rs`
- Pumpkin bow shooting: `pumpkin/src/item/items/bow.rs:147-212`
- Pumpkin player melee attack: `pumpkin/src/entity/player.rs:914-1073`
- Pumpkin arrow projectile: `pumpkin/src/entity/projectile/arrow.rs`

---

*Document generated for Phase 3 of the Pumpkin / mcMMO event parity effort.*
