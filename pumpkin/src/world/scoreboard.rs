use std::collections::{HashMap, HashSet};

use pumpkin_data::scoreboard::ScoreboardDisplaySlot;
use pumpkin_protocol::{
    BClientPacket, ClientPacket, NumberFormat,
    bedrock::client::scoreboard::{
        CRemoveObjective as BRemoveObjective, CSetDisplayObjective as BSetDisplayObjective,
        CSetScore as BSetScore, ScoreEntry as BScoreEntry,
    },
    codec::var_int::VarInt,
    java::client::play::{
        CDisplayObjective, CSetPlayerTeam, CUpdateObjectives, CUpdateScore, Mode, RenderType,
        TeamMethod, TeamParameters,
    },
};
use pumpkin_util::text::{TextComponent, color::NamedColor};
use tracing::warn;

use super::World;
use crate::{entity::player::Player, net::bedrock::state::BedrockPacketGroup};

pub struct Scoreboard {
    objectives: HashMap<String, ScoreboardObjective<'static>>,
    teams: HashMap<String, Team>,
    scores: HashMap<String, HashMap<String, ScoreboardScore<'static>>>,
    /// Objective names ever exposed to Bedrock. Snapshot replay removes this
    /// set before rebuilding current state so removals missed during death or
    /// dimension transitions cannot survive client-side.
    bedrock_objective_history: HashSet<String>,
    bedrock_score_ids: HashMap<(String, String), i64>,
    next_bedrock_score_id: i64,
}

impl Default for Scoreboard {
    fn default() -> Self {
        Self {
            objectives: HashMap::new(),
            teams: HashMap::new(),
            scores: HashMap::new(),
            bedrock_objective_history: HashSet::new(),
            bedrock_score_ids: HashMap::new(),
            // Zero is reserved by several Bedrock identity paths. Positive,
            // monotonically allocated IDs remain stable across score updates.
            next_bedrock_score_id: 1,
        }
    }
}

impl Scoreboard {
    fn bedrock_score_id(&mut self, objective_name: &str, entity_name: &str) -> i64 {
        let key = (objective_name.to_string(), entity_name.to_string());
        if let Some(id) = self.bedrock_score_ids.get(&key) {
            return *id;
        }
        let id = self.next_bedrock_score_id;
        self.next_bedrock_score_id = self.next_bedrock_score_id.saturating_add(1);
        self.bedrock_score_ids.insert(key, id);
        id
    }

    fn bedrock_custom_name(&self, entity_name: &str) -> String {
        self.teams
            .values()
            .find(|team| team.players.iter().any(|player| player == entity_name))
            .map_or_else(
                || entity_name.to_string(),
                |team| {
                    format!(
                        "{}{}{}",
                        team.player_prefix.clone().get_text(),
                        entity_name,
                        team.player_suffix.clone().get_text()
                    )
                },
            )
    }

    async fn broadcast_bedrock_score_refresh(&self, world: &World, entity_names: &[String]) {
        if entity_names.is_empty() {
            return;
        }
        let mut entries = Vec::new();
        for (objective_name, scores) in &self.scores {
            for entity_name in entity_names {
                let Some(score) = scores.get(entity_name) else {
                    continue;
                };
                let Some(scoreboard_id) = self
                    .bedrock_score_ids
                    .get(&(objective_name.clone(), entity_name.clone()))
                else {
                    continue;
                };
                entries.push(BScoreEntry {
                    scoreboard_id: *scoreboard_id,
                    objective_name: objective_name.clone(),
                    score: score.value,
                    entry_type: VarInt(3),
                    entity_unique_id: 0,
                    custom_name: self.bedrock_custom_name(entity_name),
                });
            }
        }
        if entries.is_empty() {
            return;
        }

        let packet = BSetScore {
            action: VarInt(0),
            entries,
        };
        for player in world.players.load().iter() {
            if let Some(client) = player.client.bedrock()
                && client.allows_packet_group(BedrockPacketGroup::PersistentUi)
            {
                client.enqueue_packet(&packet).await;
            }
        }
    }

    async fn broadcast_editioned<J: ClientPacket, B: BClientPacket>(
        world: &World,
        je_packet: &J,
        be_packet: &B,
    ) {
        let players = world.players.load();
        for player in players.iter() {
            match player.client.as_ref() {
                crate::net::ClientPlatform::Java(client) => {
                    client.enqueue_packet(je_packet).await;
                }
                crate::net::ClientPlatform::Bedrock(client)
                    if client.allows_packet_group(BedrockPacketGroup::PersistentUi) =>
                {
                    client.enqueue_packet(be_packet).await;
                }
                crate::net::ClientPlatform::Bedrock(_) => {}
            }
        }
    }

    pub async fn add_objective(&mut self, world: &World, objective: ScoreboardObjective<'static>) {
        if self.objectives.contains_key(objective.name) {
            warn!(
                "Tried to create an objective which already exists: {}",
                &objective.name
            );
            return;
        }

        let je_update = CUpdateObjectives::new(
            objective.name.to_string(),
            Mode::Add,
            objective.display_name.clone(),
            objective.render_type,
            objective.number_format.clone(),
        );

        let be_update = BSetDisplayObjective {
            display_slot: "sidebar".to_string(), // Default to sidebar
            objective_name: objective.name.to_string(),
            display_name: objective.display_name.clone().get_text(),
            criteria_name: "dummy".to_string(),
            sort_order: VarInt(0),
        };

        self.bedrock_objective_history
            .insert(objective.name.to_string());

        Self::broadcast_editioned(world, &je_update, &be_update).await;

        let je_display =
            CDisplayObjective::new(ScoreboardDisplaySlot::Sidebar, objective.name.to_string());
        // Bedrock's SetDisplayObjective already sets the slot.

        world.broadcast_packet_all(&je_display);

        self.objectives
            .insert(objective.name.to_string(), objective);
    }

    pub async fn remove_objective(&mut self, world: &World, name: &str) {
        if !self.objectives.contains_key(name) {
            warn!(
                "Tried to remove an objective which does not exist: {}",
                name
            );
            return;
        }

        let je_packet = CUpdateObjectives::new(
            name.to_string(),
            Mode::Remove,
            TextComponent::empty(),
            RenderType::Integer,
            None,
        );

        let be_packet = BRemoveObjective {
            objective_name: name.to_string(),
        };

        Self::broadcast_editioned(world, &je_packet, &be_packet).await;

        self.objectives.remove(name);
        self.scores.remove(name);
        self.bedrock_score_ids
            .retain(|(objective_name, _), _| objective_name != name);
    }

    pub async fn update_score(&mut self, world: &World, score: ScoreboardScore<'static>) {
        if !self.objectives.contains_key(score.objective_name) {
            warn!(
                "Tried to place a score into an objective which does not exist: {}",
                &score.objective_name
            );
            return;
        }

        let je_packet = CUpdateScore::new(
            score.entity_name.to_string(),
            score.objective_name.to_string(),
            score.value,
            score.display_name.clone(),
            score.number_format.clone(),
        );

        let scoreboard_id = self.bedrock_score_id(score.objective_name, score.entity_name);
        let custom_name = self.bedrock_custom_name(score.entity_name);
        let be_packet = BSetScore {
            action: VarInt(0), // Change
            entries: vec![BScoreEntry {
                scoreboard_id,
                objective_name: score.objective_name.to_string(),
                score: score.value,
                entry_type: VarInt(3), // Fake player/Literal
                entity_unique_id: 0,
                custom_name,
            }],
        };

        Self::broadcast_editioned(world, &je_packet, &be_packet).await;

        self.scores
            .entry(score.objective_name.to_string())
            .or_default()
            .insert(score.entity_name.to_string(), score);
    }

    pub async fn remove_score(&mut self, world: &World, entity_name: &str, objective_name: &str) {
        let je_packet =
            CUpdateScore::new_remove(entity_name.to_string(), objective_name.to_string());

        let scoreboard_id = self.bedrock_score_id(objective_name, entity_name);
        let be_packet = BSetScore {
            action: VarInt(1), // Remove
            entries: vec![BScoreEntry {
                scoreboard_id,
                objective_name: objective_name.to_string(),
                score: VarInt(0),
                entry_type: VarInt(3),
                entity_unique_id: 0,
                custom_name: entity_name.to_string(),
            }],
        };

        Self::broadcast_editioned(world, &je_packet, &be_packet).await;

        if let Some(objective_scores) = self.scores.get_mut(objective_name) {
            objective_scores.remove(entity_name);
        }
        self.bedrock_score_ids
            .remove(&(objective_name.to_string(), entity_name.to_string()));
    }

    /// Replays the current Bedrock-visible scoreboard after the client has
    /// completed a join or recovery boundary. Bedrock has no Java-style team
    /// packet, so team prefixes and suffixes are folded into fake-player score
    /// names while the stable score identity remains unchanged.
    pub async fn send_snapshot_to(&self, player: &Player) {
        let Some(client) = player.client.bedrock() else {
            return;
        };
        if !client.allows_packet_group(BedrockPacketGroup::PersistentUi) {
            return;
        }

        let mut known_objectives: Vec<_> = self.bedrock_objective_history.iter().collect();
        known_objectives.sort_unstable();
        for objective_name in known_objectives {
            client
                .send_game_packet(&BRemoveObjective {
                    objective_name: objective_name.clone(),
                })
                .await;
        }

        let mut objective_names: Vec<_> = self.objectives.keys().collect();
        objective_names.sort_unstable();
        for objective_name in objective_names {
            let objective = &self.objectives[objective_name];
            client
                .send_game_packet(&BSetDisplayObjective {
                    display_slot: "sidebar".to_string(),
                    objective_name: objective.name.to_string(),
                    display_name: objective.display_name.clone().get_text(),
                    criteria_name: "dummy".to_string(),
                    sort_order: VarInt(0),
                })
                .await;
        }

        let mut entries = Vec::new();
        for objective_name in self.objectives.keys() {
            let Some(scores) = self.scores.get(objective_name) else {
                continue;
            };
            let mut entity_names: Vec<_> = scores.keys().collect();
            entity_names.sort_unstable();
            for entity_name in entity_names {
                let score = &scores[entity_name];
                let Some(scoreboard_id) = self
                    .bedrock_score_ids
                    .get(&(objective_name.clone(), entity_name.clone()))
                else {
                    warn!(
                        objective = %objective_name,
                        entity = %entity_name,
                        "Skipping Bedrock scoreboard snapshot entry without a stable ID"
                    );
                    continue;
                };
                entries.push(BScoreEntry {
                    scoreboard_id: *scoreboard_id,
                    objective_name: objective_name.clone(),
                    score: score.value,
                    entry_type: VarInt(3),
                    entity_unique_id: 0,
                    custom_name: self.bedrock_custom_name(entity_name),
                });
            }
        }
        if !entries.is_empty() {
            client
                .send_game_packet(&BSetScore {
                    action: VarInt(0),
                    entries,
                })
                .await;
        }
    }

    pub async fn add_team(&mut self, world: &World, team: Team) {
        if self.teams.contains_key(&team.name) {
            warn!(
                "Tried to create Team which does already exist, {}",
                team.name
            );
            return;
        }

        let parameters = TeamParameters {
            display_name: &team.display_name,
            options: team.options,
            nametag_visibility: team.nametag_visibility.to_str(),
            collision_rule: team.collision_rule.to_str(),
            color: team.color as i32,
            player_prefix: &team.player_prefix,
            player_suffix: &team.player_suffix,
        };

        world.broadcast_packet_all(&CSetPlayerTeam {
            team_name: team.name.clone(),
            method: TeamMethod::Create,
            parameters: Some(parameters),
            players: team.players.clone().into(),
        });

        let players = team.players.clone();
        self.teams.insert(team.name.clone(), team);
        self.broadcast_bedrock_score_refresh(world, &players).await;
    }

    pub async fn update_team(&mut self, world: &World, team: Team) {
        if !self.teams.contains_key(&team.name) {
            warn!("Tried to update Team which does not exist, {}", team.name);
            return;
        }

        let parameters = TeamParameters {
            display_name: &team.display_name,
            options: team.options,
            nametag_visibility: team.nametag_visibility.to_str(),
            collision_rule: team.collision_rule.to_str(),
            color: team.color as i32,
            player_prefix: &team.player_prefix,
            player_suffix: &team.player_suffix,
        };

        world.broadcast_packet_all(&CSetPlayerTeam {
            team_name: team.name.clone(),
            method: TeamMethod::Update,
            parameters: Some(parameters),
            players: Box::new([]),
        });

        let mut players = self
            .teams
            .get(&team.name)
            .map_or_else(Vec::new, |old| old.players.clone());
        players.extend(team.players.iter().cloned());
        players.sort_unstable();
        players.dedup();
        self.teams.insert(team.name.clone(), team);
        self.broadcast_bedrock_score_refresh(world, &players).await;
    }

    pub async fn remove_team(&mut self, world: &World, name: &str) {
        if !self.teams.contains_key(name) {
            warn!("Tried to remove Team which does not exist, {}", name);
            return;
        }

        world.broadcast_packet_all(&CSetPlayerTeam {
            team_name: name.to_string(),
            method: TeamMethod::Remove,
            parameters: None,
            players: Box::new([]),
        });

        let players = self
            .teams
            .remove(name)
            .map_or_else(Vec::new, |team| team.players);
        self.broadcast_bedrock_score_refresh(world, &players).await;
    }

    pub async fn add_player_to_team(&mut self, world: &World, team_name: &str, player: String) {
        let Some(team) = self.teams.get_mut(team_name) else {
            warn!(
                "Tried to add player to Team which does not exist, {}",
                team_name
            );
            return;
        };

        if team.players.contains(&player) {
            return;
        }

        world.broadcast_packet_all(&CSetPlayerTeam {
            team_name: team_name.to_string(),
            method: TeamMethod::AddPlayers,
            parameters: None,
            players: vec![player.clone()].into(),
        });

        team.players.push(player.clone());
        self.broadcast_bedrock_score_refresh(world, &[player]).await;
    }

    pub async fn remove_player_from_team(&mut self, world: &World, team_name: &str, player: &str) {
        let Some(team) = self.teams.get_mut(team_name) else {
            warn!(
                "Tried to remove player from Team which does not exist, {}",
                team_name
            );
            return;
        };

        if !team.players.contains(&player.to_string()) {
            return;
        }

        world.broadcast_packet_all(&CSetPlayerTeam {
            team_name: team_name.to_string(),
            method: TeamMethod::RemovePlayers,
            parameters: None,
            players: vec![player.to_string()].into(),
        });

        team.players.retain(|p| p != player);
        self.broadcast_bedrock_score_refresh(world, &[player.to_string()])
            .await;
    }
}

pub struct ScoreboardObjective<'a> {
    pub name: &'a str,
    pub display_name: TextComponent,
    pub render_type: RenderType,
    pub number_format: Option<NumberFormat>,
}

impl<'a> ScoreboardObjective<'a> {
    #[must_use]
    pub const fn new(
        name: &'a str,
        display_name: TextComponent,
        render_type: RenderType,
        number_format: Option<NumberFormat>,
    ) -> Self {
        Self {
            name,
            display_name,
            render_type,
            number_format,
        }
    }
}

pub struct ScoreboardScore<'a> {
    pub entity_name: &'a str,
    pub objective_name: &'a str,
    pub value: VarInt,
    pub display_name: Option<TextComponent>,
    pub number_format: Option<NumberFormat>,
}

impl<'a> ScoreboardScore<'a> {
    #[must_use]
    pub const fn new(
        entity_name: &'a str,
        objective_name: &'a str,
        value: VarInt,
        display_name: Option<TextComponent>,
        number_format: Option<NumberFormat>,
    ) -> Self {
        Self {
            entity_name,
            objective_name,
            value,
            display_name,
            number_format,
        }
    }
}

pub enum NameTagVisibility {
    Always,
    Never,
    HideForOtherTeams,
    HideForOwnTeam,
}

impl NameTagVisibility {
    #[must_use]
    pub const fn to_str(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Never => "never",
            Self::HideForOtherTeams => "hideForOtherTeams",
            Self::HideForOwnTeam => "hideForOwnTeam",
        }
    }
}

pub enum CollisionRule {
    Always,
    Never,
    PushOtherTeams,
    PushOwnTeam,
}

impl CollisionRule {
    #[must_use]
    pub const fn to_str(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Never => "never",
            Self::PushOtherTeams => "pushOtherTeams",
            Self::PushOwnTeam => "pushOwnTeam",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Scoreboard;

    #[test]
    fn bedrock_score_ids_are_stable_and_never_pointer_derived() {
        let mut scoreboard = Scoreboard::default();
        let first = scoreboard.bedrock_score_id("objective", "player");
        let repeated = scoreboard.bedrock_score_id("objective", "player");
        let other_objective = scoreboard.bedrock_score_id("other", "player");
        let other_player = scoreboard.bedrock_score_id("objective", "other");

        assert_eq!(first, repeated);
        assert!(first > 0);
        assert_ne!(first, other_objective);
        assert_ne!(first, other_player);
    }
}

pub struct Team {
    pub name: String,
    pub display_name: TextComponent,
    pub options: i8,
    pub nametag_visibility: NameTagVisibility,
    pub collision_rule: CollisionRule,
    pub color: NamedColor,
    pub player_prefix: TextComponent,
    pub player_suffix: TextComponent,
    pub players: Vec<String>,
}
