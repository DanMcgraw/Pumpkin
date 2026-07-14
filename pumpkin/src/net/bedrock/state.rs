use pumpkin_data::{
    dimension::Dimension,
    game_rules::{
        GameRule as PumpkinGameRule, GameRuleRegistry, GameRuleValue as PumpkinGameRuleValue,
    },
};
use pumpkin_protocol::bedrock::client::gamerules_changed::{
    GameRule as BedrockGameRule, GameRuleValue as BedrockGameRuleValue, StartGameRules,
};

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

    use super::{dimension_id, game_rule, game_rules};

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
        assert!(rules.rules.iter().any(|rule| {
            rule.name == "spawnradius" && rule.value == GameRuleValue::Int(0)
        }));
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
