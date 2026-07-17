use pumpkin_data::{
    dimension::Dimension,
    game_rules::{
        GameRule as PumpkinGameRule, GameRuleRegistry, GameRuleValue as PumpkinGameRuleValue,
    },
};
use pumpkin_protocol::bedrock::client::gamerules_changed::{
    GameRule as BedrockGameRule, GameRuleValue as BedrockGameRuleValue, StartGameRules,
};

/// Server-owned lifecycle for a Bedrock play session.
///
/// Bedrock accepts different packet families at different points in the
/// connection. Keeping this state separate from Java's load timeout prevents
/// gameplay, UI, and recovery packets from leaking across join, dimension, or
/// death boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BedrockSessionState {
    Initializing = 0,
    Playing = 1,
    ChangingDimension = 2,
    Dead = 3,
    Respawning = 4,
    Disconnected = 5,
}

impl BedrockSessionState {
    #[must_use]
    pub const fn from_wire(value: u8) -> Self {
        match value {
            0 => Self::Initializing,
            1 => Self::Playing,
            2 => Self::ChangingDimension,
            3 => Self::Dead,
            4 => Self::Respawning,
            _ => Self::Disconnected,
        }
    }

    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        self as u8 == next as u8
            || matches!(
                (self, next),
                (Self::Initializing | Self::ChangingDimension, Self::Playing)
                    | (Self::Playing, Self::ChangingDimension | Self::Dead)
                    | (Self::Dead, Self::Respawning)
                    | (Self::Respawning, Self::Playing | Self::ChangingDimension)
                    | (_, Self::Disconnected)
            )
    }

    #[must_use]
    pub const fn allows(self, group: BedrockPacketGroup) -> bool {
        matches!(
            (self, group),
            (Self::Initializing, BedrockPacketGroup::Bootstrap)
                | (
                    Self::Playing,
                    BedrockPacketGroup::Gameplay | BedrockPacketGroup::PersistentUi
                )
                | (
                    Self::ChangingDimension | Self::Respawning,
                    BedrockPacketGroup::WorldTransition
                )
                | (Self::Dead, BedrockPacketGroup::Death)
        )
    }
}

/// Packet families used to test and document lifecycle gates. Individual
/// packet codecs stay unaware of session policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BedrockPacketGroup {
    Bootstrap,
    Gameplay,
    PersistentUi,
    WorldTransition,
    Death,
}

#[must_use]
pub const fn recovery_replay_pending(
    state: BedrockSessionState,
    recovery_epoch: u64,
    replayed_epoch: u64,
) -> bool {
    matches!(state, BedrockSessionState::Playing)
        && recovery_epoch != 0
        && replayed_epoch < recovery_epoch
}

/// Initial join is the only lifecycle that emits `PlayerSpawn` from chunk
/// progress. Dimension changes complete it explicitly after the client ack.
#[must_use]
pub const fn should_send_initial_player_spawn(
    state: BedrockSessionState,
    spawned: bool,
    sent_chunks: usize,
) -> bool {
    matches!(state, BedrockSessionState::Initializing) && !spawned && sent_chunks > 4
}

/// Maps Pumpkin's Java-style dimension registry entries to Bedrock's fixed
/// dimension IDs.
#[must_use]
pub fn dimension_id(dimension: &Dimension) -> i32 {
    match dimension.minecraft_name {
        "minecraft:the_nether" => 1,
        "minecraft:the_end" => 2,
        // Bedrock has no separate overworld-caves dimension. Custom dimensions
        // currently use the overworld representation until custom Bedrock
        // dimensions are implemented.
        _ => 0,
    }
}

/// Returns the Bedrock-visible subset of Pumpkin's gamerules.
#[must_use]
pub fn game_rules(registry: &GameRuleRegistry) -> StartGameRules {
    StartGameRules::new(
        PumpkinGameRule::all()
            .iter()
            .filter_map(|rule| game_rule(rule, registry.get(rule)))
            .collect(),
    )
}

/// Maps one Pumpkin gamerule to its Bedrock name and typed value.
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "the typed gamerule view is a small copyable value returned by the registry"
)]
pub fn game_rule(
    rule: &PumpkinGameRule,
    value: PumpkinGameRuleValue<&i64, &bool>,
) -> Option<BedrockGameRule> {
    let name = match rule {
        PumpkinGameRule::AdvanceTime => "dodaylightcycle",
        PumpkinGameRule::AdvanceWeather => "doweathercycle",
        PumpkinGameRule::BlockDrops => "dotiledrops",
        PumpkinGameRule::DrowningDamage => "drowningdamage",
        PumpkinGameRule::EntityDrops => "doentitydrops",
        PumpkinGameRule::FallDamage => "falldamage",
        PumpkinGameRule::FireDamage => "firedamage",
        PumpkinGameRule::FreezeDamage => "freezedamage",
        PumpkinGameRule::ImmediateRespawn => "doimmediaterespawn",
        PumpkinGameRule::KeepInventory => "keepinventory",
        PumpkinGameRule::MobDrops => "domobloot",
        PumpkinGameRule::MobGriefing => "mobgriefing",
        PumpkinGameRule::NaturalHealthRegeneration => "naturalregeneration",
        PumpkinGameRule::Pvp => "pvp",
        PumpkinGameRule::ReducedDebugInfo => "showcoordinates",
        PumpkinGameRule::RespawnRadius => "spawnradius",
        PumpkinGameRule::SendCommandFeedback => "sendcommandfeedback",
        PumpkinGameRule::ShowDeathMessages => "showdeathmessages",
        PumpkinGameRule::SpawnMobs => "domobspawning",
        PumpkinGameRule::TntExplodes => "tntexplodes",
        _ => return None,
    };

    let value = match rule {
        // Match Geyser's server-authoritative HUD and death handling. The
        // Pumpkin values still control gameplay; these client-visible values
        // prevent Bedrock from regenerating health or clearing inventory on
        // its own before Pumpkin sends the resulting state.
        PumpkinGameRule::NaturalHealthRegeneration => BedrockGameRuleValue::Bool(false),
        PumpkinGameRule::KeepInventory => BedrockGameRuleValue::Bool(true),
        PumpkinGameRule::RespawnRadius => BedrockGameRuleValue::Int(0),
        _ => match value {
            PumpkinGameRuleValue::Int(value) => BedrockGameRuleValue::Int(
                (*value).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            ),
            PumpkinGameRuleValue::Bool(value) => {
                let value = if matches!(rule, PumpkinGameRule::ReducedDebugInfo) {
                    !*value
                } else {
                    *value
                };
                BedrockGameRuleValue::Bool(value)
            }
        },
    };

    Some(BedrockGameRule::new(name, value))
}

#[cfg(test)]
mod tests {
    use pumpkin_data::{
        dimension::Dimension,
        game_rules::{GameRule as PumpkinGameRule, GameRuleRegistry},
    };
    use pumpkin_protocol::bedrock::client::gamerules_changed::GameRuleValue;

    use super::{
        BedrockPacketGroup, BedrockSessionState, dimension_id, game_rule, game_rules,
        recovery_replay_pending, should_send_initial_player_spawn,
    };

    #[test]
    fn lifecycle_rejects_packets_outside_their_session_boundary() {
        assert!(BedrockSessionState::Initializing.allows(BedrockPacketGroup::Bootstrap));
        assert!(!BedrockSessionState::Initializing.allows(BedrockPacketGroup::Gameplay));
        assert!(BedrockSessionState::Playing.allows(BedrockPacketGroup::Gameplay));
        assert!(BedrockSessionState::Playing.allows(BedrockPacketGroup::PersistentUi));
        assert!(BedrockSessionState::ChangingDimension.allows(BedrockPacketGroup::WorldTransition));
        assert!(BedrockSessionState::Dead.allows(BedrockPacketGroup::Death));
        assert!(!BedrockSessionState::Disconnected.allows(BedrockPacketGroup::Gameplay));
    }

    #[test]
    fn chunk_progress_only_completes_initial_spawn() {
        assert!(should_send_initial_player_spawn(
            BedrockSessionState::Initializing,
            false,
            5,
        ));
        assert!(!should_send_initial_player_spawn(
            BedrockSessionState::ChangingDimension,
            false,
            5,
        ));
        assert!(!should_send_initial_player_spawn(
            BedrockSessionState::Initializing,
            true,
            5,
        ));
    }

    #[test]
    fn lifecycle_only_accepts_defined_transitions() {
        assert!(BedrockSessionState::Initializing.can_transition_to(BedrockSessionState::Playing));
        assert!(BedrockSessionState::Playing.can_transition_to(BedrockSessionState::Dead));
        assert!(BedrockSessionState::Dead.can_transition_to(BedrockSessionState::Respawning));
        assert!(BedrockSessionState::Respawning.can_transition_to(BedrockSessionState::Playing));
        assert!(!BedrockSessionState::Initializing.can_transition_to(BedrockSessionState::Dead));
        assert!(!BedrockSessionState::Dead.can_transition_to(BedrockSessionState::Playing));
    }

    #[test]
    fn recovery_generation_is_replayed_only_once_while_playing() {
        assert!(recovery_replay_pending(BedrockSessionState::Playing, 2, 1));
        assert!(!recovery_replay_pending(BedrockSessionState::Playing, 2, 2));
        assert!(!recovery_replay_pending(
            BedrockSessionState::Respawning,
            2,
            1
        ));
        assert!(!recovery_replay_pending(BedrockSessionState::Playing, 0, 0));
    }

    #[test]
    fn maps_vanilla_dimensions_to_bedrock_ids() {
        assert_eq!(dimension_id(&Dimension::OVERWORLD), 0);
        assert_eq!(dimension_id(&Dimension::OVERWORLD_CAVES), 0);
        assert_eq!(dimension_id(&Dimension::THE_NETHER), 1);
        assert_eq!(dimension_id(&Dimension::THE_END), 2);
    }

    #[test]
    fn builds_initial_typed_game_rules() {
        let registry = GameRuleRegistry::default();
        let rules = game_rules(&registry);

        assert!(rules.rules.iter().any(|rule| {
            rule.name == "dodaylightcycle" && rule.value == GameRuleValue::Bool(true)
        }));
        assert!(
            rules
                .rules
                .iter()
                .any(|rule| { rule.name == "spawnradius" && rule.value == GameRuleValue::Int(0) })
        );
    }

    #[test]
    fn inverts_reduced_debug_info_for_show_coordinates() {
        let registry = GameRuleRegistry::default();
        let rule = PumpkinGameRule::ReducedDebugInfo;
        let mapped = game_rule(&rule, registry.get(&rule)).unwrap();

        assert_eq!(mapped.name, "showcoordinates");
        assert_eq!(
            mapped.value,
            GameRuleValue::Bool(!registry.reduced_debug_info)
        );
    }

    #[test]
    fn forces_server_authoritative_bedrock_player_rules() {
        let registry = GameRuleRegistry::default();

        for (rule, expected) in [
            (
                PumpkinGameRule::NaturalHealthRegeneration,
                GameRuleValue::Bool(false),
            ),
            (PumpkinGameRule::KeepInventory, GameRuleValue::Bool(true)),
            (PumpkinGameRule::RespawnRadius, GameRuleValue::Int(0)),
        ] {
            let mapped = game_rule(&rule, registry.get(&rule)).unwrap();
            assert_eq!(mapped.value, expected);
        }
    }
}
