use std::collections::{HashMap, HashSet};

use pumpkin_data::scoreboard::ScoreboardDisplaySlot;
use pumpkin_protocol::{
    BClientPacket, NumberFormat,
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
    display_slots: [Option<String>; 19],
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
            display_slots: std::array::from_fn(|_| None),
            bedrock_objective_history: HashSet::new(),
            bedrock_score_ids: HashMap::new(),
            // Zero is reserved by several Bedrock identity paths. Positive,
            // monotonically allocated IDs remain stable across score updates.
            next_bedrock_score_id: 1,
        }
    }
}

impl Scoreboard {
    fn bedrock_score_id(&mut self, bedrock_objective_id: &str, entity_name: &str) -> i64 {
        let key = (bedrock_objective_id.to_string(), entity_name.to_string());
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

    fn bedrock_score_name(&self, score: &ScoreboardScore<'_>) -> String {
        score.display_name.as_ref().map_or_else(
            || self.bedrock_custom_name(score.entity_name),
            |display_name| display_name.clone().get_text(),
        )
    }

    const fn slot_index(slot: &ScoreboardDisplaySlot) -> usize {
        match slot {
            ScoreboardDisplaySlot::List => 0,
            ScoreboardDisplaySlot::Sidebar => 1,
            ScoreboardDisplaySlot::BelowName => 2,
            ScoreboardDisplaySlot::TeamBlack => 3,
            ScoreboardDisplaySlot::TeamDarkBlue => 4,
            ScoreboardDisplaySlot::TeamDarkGreen => 5,
            ScoreboardDisplaySlot::TeamDarkAqua => 6,
            ScoreboardDisplaySlot::TeamDarkRed => 7,
            ScoreboardDisplaySlot::TeamDarkPurple => 8,
            ScoreboardDisplaySlot::TeamGold => 9,
            ScoreboardDisplaySlot::TeamGray => 10,
            ScoreboardDisplaySlot::TeamDarkGray => 11,
            ScoreboardDisplaySlot::TeamBlue => 12,
            ScoreboardDisplaySlot::TeamGreen => 13,
            ScoreboardDisplaySlot::TeamAqua => 14,
            ScoreboardDisplaySlot::TeamRed => 15,
            ScoreboardDisplaySlot::TeamLightPurple => 16,
            ScoreboardDisplaySlot::TeamYellow => 17,
            ScoreboardDisplaySlot::TeamWhite => 18,
        }
    }

    const fn slot_from_index(index: usize) -> ScoreboardDisplaySlot {
        match index {
            0 => ScoreboardDisplaySlot::List,
            1 => ScoreboardDisplaySlot::Sidebar,
            2 => ScoreboardDisplaySlot::BelowName,
            3 => ScoreboardDisplaySlot::TeamBlack,
            4 => ScoreboardDisplaySlot::TeamDarkBlue,
            5 => ScoreboardDisplaySlot::TeamDarkGreen,
            6 => ScoreboardDisplaySlot::TeamDarkAqua,
            7 => ScoreboardDisplaySlot::TeamDarkRed,
            8 => ScoreboardDisplaySlot::TeamDarkPurple,
            9 => ScoreboardDisplaySlot::TeamGold,
            10 => ScoreboardDisplaySlot::TeamGray,
            11 => ScoreboardDisplaySlot::TeamDarkGray,
            12 => ScoreboardDisplaySlot::TeamBlue,
            13 => ScoreboardDisplaySlot::TeamGreen,
            14 => ScoreboardDisplaySlot::TeamAqua,
            15 => ScoreboardDisplaySlot::TeamRed,
            16 => ScoreboardDisplaySlot::TeamLightPurple,
            17 => ScoreboardDisplaySlot::TeamYellow,
            _ => ScoreboardDisplaySlot::TeamWhite,
        }
    }

    const fn bedrock_slot_name(index: usize) -> &'static str {
        match index {
            0 => "list",
            2 => "belowname",
            _ => "sidebar",
        }
    }

    fn bedrock_objective_id(index: usize, objective_name: &str) -> String {
        format!("{objective_name}:{index}")
    }

    fn displayed_instances_for(&self, objective_name: &str) -> Vec<(usize, String)> {
        self.display_slots
            .iter()
            .enumerate()
            .filter_map(|(index, displayed)| {
                displayed
                    .as_deref()
                    .filter(|displayed| *displayed == objective_name)
                    .map(|_| (index, Self::bedrock_objective_id(index, objective_name)))
            })
            .collect()
    }

    async fn broadcast_bedrock<B: BClientPacket>(world: &World, packet: &B) {
        for player in world.players.load().iter() {
            if let Some(client) = player.client.bedrock()
                && client.allows_packet_group(BedrockPacketGroup::PersistentUi)
            {
                client.enqueue_packet(packet).await;
            }
        }
    }

    async fn broadcast_bedrock_scores_for_objective(
        &mut self,
        world: &World,
        objective_name: &str,
    ) {
        let Some(scores) = self.scores.get(objective_name) else {
            return;
        };
        let scores = scores.values().cloned().collect::<Vec<_>>();
        let instances = self.displayed_instances_for(objective_name);
        let mut entries = Vec::new();
        for (_, bedrock_objective_id) in instances {
            for score in &scores {
                let scoreboard_id = self.bedrock_score_id(&bedrock_objective_id, score.entity_name);
                entries.push(BScoreEntry {
                    scoreboard_id,
                    objective_name: bedrock_objective_id.clone(),
                    score: score.value,
                    entry_type: VarInt(3),
                    entity_unique_id: 0,
                    custom_name: self.bedrock_score_name(score),
                });
            }
        }
        if !entries.is_empty() {
            Self::broadcast_bedrock(
                world,
                &BSetScore {
                    action: VarInt(0),
                    entries,
                },
            )
            .await;
        }
    }

    async fn broadcast_bedrock_score_refresh(&self, world: &World, entity_names: &[String]) {
        if entity_names.is_empty() {
            return;
        }
        let mut entries = Vec::new();
        for (index, objective_name) in self.display_slots.iter().enumerate() {
            let Some(objective_name) = objective_name else {
                continue;
            };
            let Some(scores) = self.scores.get(objective_name) else {
                continue;
            };
            let bedrock_objective_id = Self::bedrock_objective_id(index, objective_name);
            for entity_name in entity_names {
                let Some(score) = scores.get(entity_name) else {
                    continue;
                };
                let Some(scoreboard_id) = self
                    .bedrock_score_ids
                    .get(&(bedrock_objective_id.clone(), entity_name.clone()))
                else {
                    continue;
                };
                entries.push(BScoreEntry {
                    scoreboard_id: *scoreboard_id,
                    objective_name: bedrock_objective_id.clone(),
                    score: score.value,
                    entry_type: VarInt(3),
                    entity_unique_id: 0,
                    custom_name: self.bedrock_score_name(score),
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

    pub async fn add_objective(
        &mut self,
        world: &World,
        objective: ScoreboardObjective<'static>,
    ) -> bool {
        if self.objectives.contains_key(objective.name) {
            warn!(
                "Tried to create an objective which already exists: {}",
                &objective.name
            );
            return false;
        }

        let je_update = CUpdateObjectives::new(
            objective.name.to_string(),
            Mode::Add,
            objective.display_name.clone(),
            objective.render_type,
            objective.number_format.clone(),
        );

        world.broadcast_packet_all(&je_update);

        self.objectives
            .insert(objective.name.to_string(), objective);
        true
    }

    pub async fn update_objective(
        &mut self,
        world: &World,
        objective: ScoreboardObjective<'static>,
    ) -> bool {
        if !self.objectives.contains_key(objective.name) {
            return false;
        }

        world.broadcast_packet_all(&CUpdateObjectives::new(
            objective.name.to_string(),
            Mode::Update,
            objective.display_name.clone(),
            objective.render_type,
            objective.number_format.clone(),
        ));
        self.objectives
            .insert(objective.name.to_string(), objective.clone());

        for (index, bedrock_objective_id) in self.displayed_instances_for(objective.name) {
            Self::broadcast_bedrock(
                world,
                &BRemoveObjective {
                    objective_name: bedrock_objective_id.clone(),
                },
            )
            .await;
            Self::broadcast_bedrock(
                world,
                &BSetDisplayObjective {
                    display_slot: Self::bedrock_slot_name(index).to_string(),
                    objective_name: bedrock_objective_id,
                    display_name: objective.display_name.clone().get_text(),
                    criteria_name: "dummy".to_string(),
                    sort_order: VarInt(1),
                },
            )
            .await;
        }
        self.broadcast_bedrock_scores_for_objective(world, objective.name)
            .await;
        true
    }

    pub async fn remove_objective(&mut self, world: &World, name: &str) -> bool {
        if !self.objectives.contains_key(name) {
            warn!(
                "Tried to remove an objective which does not exist: {}",
                name
            );
            return false;
        }

        let je_packet = CUpdateObjectives::new(
            name.to_string(),
            Mode::Remove,
            TextComponent::empty(),
            RenderType::Integer,
            None,
        );

        world.broadcast_packet_all(&je_packet);

        let prefix = format!("{name}:");
        let bedrock_ids = self
            .bedrock_objective_history
            .iter()
            .filter(|id| id.starts_with(&prefix))
            .cloned()
            .collect::<Vec<_>>();
        for objective_name in bedrock_ids {
            Self::broadcast_bedrock(world, &BRemoveObjective { objective_name }).await;
        }

        self.objectives.remove(name);
        self.scores.remove(name);
        self.bedrock_score_ids
            .retain(|(objective_name, _), _| !objective_name.starts_with(&prefix));
        self.bedrock_objective_history
            .retain(|objective_name| !objective_name.starts_with(&prefix));
        for displayed in &mut self.display_slots {
            if displayed.as_deref() == Some(name) {
                *displayed = None;
            }
        }
        true
    }

    pub async fn update_score(&mut self, world: &World, score: ScoreboardScore<'static>) -> bool {
        if !self.objectives.contains_key(score.objective_name) {
            warn!(
                "Tried to place a score into an objective which does not exist: {}",
                &score.objective_name
            );
            return false;
        }

        let je_packet = CUpdateScore::new(
            score.entity_name.to_string(),
            score.objective_name.to_string(),
            score.value,
            score.display_name.clone(),
            score.number_format.clone(),
        );

        world.broadcast_packet_all(&je_packet);

        let mut entries = Vec::new();
        for (_, bedrock_objective_id) in self.displayed_instances_for(score.objective_name) {
            let scoreboard_id = self.bedrock_score_id(&bedrock_objective_id, score.entity_name);
            entries.push(BScoreEntry {
                scoreboard_id,
                objective_name: bedrock_objective_id,
                score: score.value,
                entry_type: VarInt(3),
                entity_unique_id: 0,
                custom_name: self.bedrock_score_name(&score),
            });
        }
        if !entries.is_empty() {
            Self::broadcast_bedrock(
                world,
                &BSetScore {
                    action: VarInt(0),
                    entries,
                },
            )
            .await;
        }

        self.scores
            .entry(score.objective_name.to_string())
            .or_default()
            .insert(score.entity_name.to_string(), score);
        true
    }

    pub async fn remove_score(
        &mut self,
        world: &World,
        entity_name: &str,
        objective_name: &str,
    ) -> bool {
        if !self
            .scores
            .get(objective_name)
            .is_some_and(|scores| scores.contains_key(entity_name))
        {
            return false;
        }
        let je_packet =
            CUpdateScore::new_remove(entity_name.to_string(), objective_name.to_string());
        world.broadcast_packet_all(&je_packet);

        let mut entries = Vec::new();
        for (_, bedrock_objective_id) in self.displayed_instances_for(objective_name) {
            if let Some(scoreboard_id) = self
                .bedrock_score_ids
                .get(&(bedrock_objective_id.clone(), entity_name.to_string()))
            {
                entries.push(BScoreEntry {
                    scoreboard_id: *scoreboard_id,
                    objective_name: bedrock_objective_id,
                    score: VarInt(0),
                    entry_type: VarInt(3),
                    entity_unique_id: 0,
                    custom_name: entity_name.to_string(),
                });
            }
        }
        if !entries.is_empty() {
            Self::broadcast_bedrock(
                world,
                &BSetScore {
                    action: VarInt(1),
                    entries,
                },
            )
            .await;
        }

        if let Some(objective_scores) = self.scores.get_mut(objective_name) {
            objective_scores.remove(entity_name);
        }
        for (_, bedrock_objective_id) in self.displayed_instances_for(objective_name) {
            self.bedrock_score_ids
                .remove(&(bedrock_objective_id, entity_name.to_string()));
        }
        true
    }

    pub async fn remove_all_scores(&mut self, world: &World, entity_name: &str) -> usize {
        let objective_names = self
            .scores
            .iter()
            .filter(|(_, scores)| scores.contains_key(entity_name))
            .map(|(objective_name, _)| objective_name.clone())
            .collect::<Vec<_>>();
        let count = objective_names.len();
        for objective_name in objective_names {
            self.remove_score(world, entity_name, &objective_name).await;
        }
        count
    }

    pub async fn set_display_slot(
        &mut self,
        world: &World,
        slot: ScoreboardDisplaySlot,
        objective_name: Option<&str>,
    ) -> bool {
        if let Some(objective_name) = objective_name
            && !self.objectives.contains_key(objective_name)
        {
            return false;
        }
        let index = Self::slot_index(&slot);
        if self.display_slots[index].as_deref() == objective_name {
            return false;
        }

        let old_display_id = self.display_slots[index]
            .as_deref()
            .map(|old| Self::bedrock_objective_id(index, old));
        self.display_slots[index] = objective_name.map(ToOwned::to_owned);
        world.broadcast_packet_all(&CDisplayObjective::new(
            slot,
            objective_name.unwrap_or_default().to_string(),
        ));

        if let Some(objective_name) = old_display_id {
            Self::broadcast_bedrock(world, &BRemoveObjective { objective_name }).await;
        }
        if let Some(objective_name) = objective_name {
            let objective = self
                .objectives
                .get(objective_name)
                .expect("objective was validated")
                .clone();
            let bedrock_objective_id = Self::bedrock_objective_id(index, objective_name);
            self.bedrock_objective_history
                .insert(bedrock_objective_id.clone());
            Self::broadcast_bedrock(
                world,
                &BSetDisplayObjective {
                    display_slot: Self::bedrock_slot_name(index).to_string(),
                    objective_name: bedrock_objective_id,
                    display_name: objective.display_name.get_text(),
                    criteria_name: "dummy".to_string(),
                    sort_order: VarInt(1),
                },
            )
            .await;
            self.broadcast_bedrock_scores_for_objective(world, objective_name)
                .await;
        }
        true
    }

    #[must_use]
    pub fn objective(&self, name: &str) -> Option<&ScoreboardObjective<'static>> {
        self.objectives.get(name)
    }

    #[must_use]
    pub fn objective_names(&self) -> Vec<String> {
        let mut names = self.objectives.keys().cloned().collect::<Vec<_>>();
        names.sort_unstable();
        names
    }

    #[must_use]
    pub fn score(
        &self,
        entity_name: &str,
        objective_name: &str,
    ) -> Option<&ScoreboardScore<'static>> {
        self.scores.get(objective_name)?.get(entity_name)
    }

    #[must_use]
    pub fn tracked_entities(&self) -> Vec<String> {
        let mut names = self
            .scores
            .values()
            .flat_map(|scores| scores.keys().cloned())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        names.sort_unstable();
        names
    }

    #[must_use]
    pub fn scores_for_entity(&self, entity_name: &str) -> Vec<ScoreboardScore<'static>> {
        let mut scores = self
            .scores
            .values()
            .filter_map(|scores| scores.get(entity_name).cloned())
            .collect::<Vec<_>>();
        scores.sort_unstable_by_key(|score| score.objective_name);
        scores
    }

    #[must_use]
    pub fn displayed_objective(&self, slot: &ScoreboardDisplaySlot) -> Option<&str> {
        self.display_slots[Self::slot_index(slot)].as_deref()
    }

    /// Sends all current scoreboard state to a Java player joining after the
    /// objectives, teams, or scores were created.
    pub async fn send_java_snapshot_to(&self, player: &Player) {
        let Some(client) = player.client.java() else {
            return;
        };

        for objective_name in self.objective_names() {
            let objective = &self.objectives[&objective_name];
            client
                .enqueue_packet(&CUpdateObjectives::new(
                    objective_name,
                    Mode::Add,
                    objective.display_name.clone(),
                    objective.render_type,
                    objective.number_format.clone(),
                ))
                .await;
        }

        let mut team_names = self.teams.keys().collect::<Vec<_>>();
        team_names.sort_unstable();
        for team_name in team_names {
            let team = &self.teams[team_name];
            client
                .enqueue_packet(&CSetPlayerTeam {
                    team_name: team.name.clone(),
                    method: TeamMethod::Create,
                    parameters: Some(TeamParameters {
                        display_name: &team.display_name,
                        options: team.options,
                        nametag_visibility: team.nametag_visibility.to_str(),
                        collision_rule: team.collision_rule.to_str(),
                        color: team.color as i32,
                        player_prefix: &team.player_prefix,
                        player_suffix: &team.player_suffix,
                    }),
                    players: team.players.clone().into(),
                })
                .await;
        }

        let mut objective_names = self.scores.keys().collect::<Vec<_>>();
        objective_names.sort_unstable();
        for objective_name in objective_names {
            let scores = &self.scores[objective_name];
            let mut entity_names = scores.keys().collect::<Vec<_>>();
            entity_names.sort_unstable();
            for entity_name in entity_names {
                let score = &scores[entity_name];
                client
                    .enqueue_packet(&CUpdateScore::new(
                        score.entity_name.to_string(),
                        score.objective_name.to_string(),
                        score.value,
                        score.display_name.clone(),
                        score.number_format.clone(),
                    ))
                    .await;
            }
        }

        for (index, objective_name) in self.display_slots.iter().enumerate() {
            if let Some(objective_name) = objective_name {
                client
                    .enqueue_packet(&CDisplayObjective::new(
                        Self::slot_from_index(index),
                        objective_name.clone(),
                    ))
                    .await;
            }
        }
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

        for (index, objective_name) in self.display_slots.iter().enumerate() {
            let Some(objective_name) = objective_name else {
                continue;
            };
            let objective = &self.objectives[objective_name];
            let bedrock_objective_id = Self::bedrock_objective_id(index, objective_name);
            client
                .send_game_packet(&BSetDisplayObjective {
                    display_slot: Self::bedrock_slot_name(index).to_string(),
                    objective_name: bedrock_objective_id,
                    display_name: objective.display_name.clone().get_text(),
                    criteria_name: "dummy".to_string(),
                    sort_order: VarInt(1),
                })
                .await;
        }

        let mut entries = Vec::new();
        for (index, objective_name) in self.display_slots.iter().enumerate() {
            let Some(objective_name) = objective_name else {
                continue;
            };
            let Some(scores) = self.scores.get(objective_name) else {
                continue;
            };
            let bedrock_objective_id = Self::bedrock_objective_id(index, objective_name);
            let mut entity_names: Vec<_> = scores.keys().collect();
            entity_names.sort_unstable();
            for entity_name in entity_names {
                let score = &scores[entity_name];
                let Some(scoreboard_id) = self
                    .bedrock_score_ids
                    .get(&(bedrock_objective_id.clone(), entity_name.clone()))
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
                    objective_name: bedrock_objective_id.clone(),
                    score: score.value,
                    entry_type: VarInt(3),
                    entity_unique_id: 0,
                    custom_name: self.bedrock_score_name(score),
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

#[derive(Clone)]
pub struct ScoreboardObjective<'a> {
    pub name: &'a str,
    pub display_name: TextComponent,
    pub render_type: RenderType,
    pub number_format: Option<NumberFormat>,
    pub criteria: &'a str,
    pub display_auto_update: bool,
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
            criteria: "dummy",
            display_auto_update: false,
        }
    }

    #[must_use]
    pub const fn with_criteria(mut self, criteria: &'a str) -> Self {
        self.criteria = criteria;
        self
    }

    #[must_use]
    pub fn writable(&self) -> bool {
        matches!(self.criteria, "dummy" | "trigger")
    }
}

#[derive(Clone)]
pub struct ScoreboardScore<'a> {
    pub entity_name: &'a str,
    pub objective_name: &'a str,
    pub value: VarInt,
    pub display_name: Option<TextComponent>,
    pub number_format: Option<NumberFormat>,
    pub locked: bool,
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
            locked: true,
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
