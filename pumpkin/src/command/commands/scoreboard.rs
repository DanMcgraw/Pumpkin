use std::sync::Arc;

use pumpkin_protocol::{NumberFormat, codec::var_int::VarInt, java::client::play::RenderType};
use pumpkin_util::text::TextComponent;

use crate::{
    command::{
        CommandExecutor, CommandResult, CommandSender,
        args::{
            ConsumedArgs, FindArg,
            bool::BoolArgConsumer,
            bounded_num::BoundedNumArgumentConsumer,
            score_holder::ScoreHoldersArgumentConsumer,
            scoreboard::{
                ScoreboardArgumentConsumer, ScoreboardArgumentKind, find_style, parse_display_slot,
            },
            simple::SimpleArgConsumer,
            textcomponent::TextComponentArgConsumer,
        },
        dispatcher::CommandError,
        tree::{
            CommandTree,
            builder::{argument, literal},
        },
    },
    server::Server,
    world::{
        World,
        scoreboard::{Scoreboard, ScoreboardObjective, ScoreboardScore},
    },
};

const NAMES: [&str; 1] = ["scoreboard"];
const DESCRIPTION: &str = "Manages scoreboard objectives and player scores.";

const ARG_OBJECTIVE: &str = "objective";
const ARG_CRITERIA: &str = "criteria";
const ARG_DISPLAY_NAME: &str = "displayName";
const ARG_SLOT: &str = "slot";
const ARG_TARGETS: &str = "targets";
const ARG_TARGET: &str = "target";
const ARG_SCORE: &str = "score";
const ARG_NAME: &str = "name";
const ARG_CONTENTS: &str = "contents";
const ARG_VALUE: &str = "value";
const ARG_STYLE: &str = "style";
const ARG_TARGET_OBJECTIVE: &str = "targetObjective";
const ARG_OPERATION: &str = "operation";
const ARG_SOURCE: &str = "source";
const ARG_SOURCE_OBJECTIVE: &str = "sourceObjective";

fn scoreboard_argument(kind: ScoreboardArgumentKind) -> ScoreboardArgumentConsumer {
    ScoreboardArgumentConsumer(kind)
}

fn command_world(sender: &CommandSender, server: &Server) -> Result<Arc<World>, CommandError> {
    sender
        .world()
        .or_else(|| server.worlds.load().first().cloned())
        .ok_or_else(|| CommandError::CommandFailed(TextComponent::text("No world is loaded")))
}

fn translated_error(key: &'static str, with: impl Into<Vec<TextComponent>>) -> CommandError {
    CommandError::CommandFailed(TextComponent::translate_cross(key, key, with))
}

async fn send_translated(
    sender: &CommandSender,
    key: &'static str,
    with: impl Into<Vec<TextComponent>>,
) {
    sender
        .send_message(TextComponent::translate_cross(key, key, with))
        .await;
}

fn component_list(components: impl IntoIterator<Item = TextComponent>) -> TextComponent {
    let mut result = TextComponent::empty();
    for (index, component) in components.into_iter().enumerate() {
        if index > 0 {
            result = result.add_child(TextComponent::text(", "));
        }
        result = result.add_child(component);
    }
    result
}

fn leak(value: &str) -> &'static str {
    Box::leak(value.to_string().into_boxed_str())
}

fn simple_arg<'a>(args: &'a ConsumedArgs, name: &str) -> Result<&'a str, CommandError> {
    SimpleArgConsumer::find_arg(args, name)
}

fn objective_or_error(
    scoreboard: &Scoreboard,
    name: &str,
) -> Result<ScoreboardObjective<'static>, CommandError> {
    scoreboard.objective(name).cloned().ok_or_else(|| {
        translated_error(
            "arguments.objective.notFound",
            [TextComponent::text(name.to_string())],
        )
    })
}

fn writable_objective_or_error(
    scoreboard: &Scoreboard,
    name: &str,
) -> Result<ScoreboardObjective<'static>, CommandError> {
    let objective = objective_or_error(scoreboard, name)?;
    if !objective.writable() {
        return Err(translated_error(
            "arguments.objective.readonly",
            [TextComponent::text(name.to_string())],
        ));
    }
    Ok(objective)
}

fn score_holders<'a>(args: &'a ConsumedArgs, name: &str) -> Result<&'a [String], CommandError> {
    ScoreHoldersArgumentConsumer::find_arg(args, name)
}

fn expanded_holders(
    scoreboard: &Scoreboard,
    holders: &[String],
) -> Result<Vec<String>, CommandError> {
    let mut expanded = if holders.iter().any(|holder| holder == "*") {
        scoreboard.tracked_entities()
    } else {
        holders.to_vec()
    };
    expanded.sort_unstable();
    expanded.dedup();
    if expanded.is_empty() {
        return Err(translated_error("argument.scoreHolder.empty", []));
    }
    Ok(expanded)
}

fn score_with_value(
    scoreboard: &Scoreboard,
    holder: &str,
    objective: &ScoreboardObjective<'static>,
    value: i32,
) -> ScoreboardScore<'static> {
    if let Some(existing) = scoreboard.score(holder, objective.name) {
        let mut score = existing.clone();
        score.value = VarInt(value);
        score
    } else {
        ScoreboardScore::new(leak(holder), objective.name, VarInt(value), None, None)
    }
}

fn bounded_score(min: Option<i32>) -> BoundedNumArgumentConsumer<i32> {
    let consumer = BoundedNumArgumentConsumer::new().name(ARG_SCORE);
    if let Some(min) = min {
        consumer.min(min)
    } else {
        consumer
    }
}

fn valid_criteria(criteria: &str) -> bool {
    matches!(
        criteria,
        "dummy"
            | "trigger"
            | "deathCount"
            | "playerKillCount"
            | "totalKillCount"
            | "health"
            | "food"
            | "air"
            | "armor"
            | "xp"
            | "level"
    ) || (criteria.starts_with("minecraft.") && criteria.contains(':'))
}

struct ListObjectivesExecutor;

impl CommandExecutor for ListObjectivesExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let world = command_world(sender, server)?;
            let scoreboard = world.scoreboard.lock().await;
            let objectives = scoreboard
                .objective_names()
                .into_iter()
                .filter_map(|name| scoreboard.objective(&name).cloned())
                .collect::<Vec<_>>();
            if objectives.is_empty() {
                send_translated(sender, "commands.scoreboard.objectives.list.empty", []).await;
            } else {
                send_translated(
                    sender,
                    "commands.scoreboard.objectives.list.success",
                    [
                        TextComponent::text(objectives.len().to_string()),
                        component_list(
                            objectives
                                .iter()
                                .map(|objective| objective.display_name.clone()),
                        ),
                    ],
                )
                .await;
            }
            Ok(objectives.len() as i32)
        })
    }
}

struct AddObjectiveExecutor {
    custom_display_name: bool,
}

impl CommandExecutor for AddObjectiveExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let name = simple_arg(args, ARG_OBJECTIVE)?;
            let criteria = simple_arg(args, ARG_CRITERIA)?;
            if !valid_criteria(criteria) {
                return Err(translated_error(
                    "argument.criteria.invalid",
                    [TextComponent::text(criteria.to_string())],
                ));
            }
            if name.is_empty() || name.chars().count() > 16 {
                return Err(CommandError::CommandFailed(TextComponent::text(
                    "Objective names must contain 1 to 16 characters",
                )));
            }
            let world = command_world(sender, server)?;
            let mut scoreboard = world.scoreboard.lock().await;
            if scoreboard.objective(name).is_some() {
                return Err(translated_error(
                    "commands.scoreboard.objectives.add.duplicate",
                    [],
                ));
            }
            let display_name = if self.custom_display_name {
                TextComponentArgConsumer::find_arg(args, ARG_DISPLAY_NAME)?
            } else {
                TextComponent::text(name.to_string())
            };
            let render_type = if criteria == "health" {
                RenderType::Hearts
            } else {
                RenderType::Integer
            };
            let objective =
                ScoreboardObjective::new(leak(name), display_name.clone(), render_type, None)
                    .with_criteria(leak(criteria));
            scoreboard.add_objective(&world, objective).await;
            let count = scoreboard.objective_names().len() as i32;
            drop(scoreboard);
            send_translated(
                sender,
                "commands.scoreboard.objectives.add.success",
                [display_name],
            )
            .await;
            Ok(count)
        })
    }
}

#[derive(Clone)]
enum ObjectiveModify {
    DisplayName,
    RenderType(RenderType),
    DisplayAutoUpdate,
    NumberFormat(Option<NumberFormat>),
    FixedNumberFormat,
    StyledNumberFormat,
}

struct ModifyObjectiveExecutor(ObjectiveModify);

impl CommandExecutor for ModifyObjectiveExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let name = simple_arg(args, ARG_OBJECTIVE)?;
            let world = command_world(sender, server)?;
            let mut scoreboard = world.scoreboard.lock().await;
            let mut objective = objective_or_error(&scoreboard, name)?;
            let key = match &self.0 {
                ObjectiveModify::DisplayName => {
                    objective.display_name =
                        TextComponentArgConsumer::find_arg(args, ARG_DISPLAY_NAME)?;
                    "commands.scoreboard.objectives.modify.displayname"
                }
                ObjectiveModify::RenderType(render_type) => {
                    objective.render_type = *render_type;
                    "commands.scoreboard.objectives.modify.rendertype"
                }
                ObjectiveModify::DisplayAutoUpdate => {
                    objective.display_auto_update = BoolArgConsumer::find_arg(args, ARG_VALUE)?;
                    if objective.display_auto_update {
                        "commands.scoreboard.objectives.modify.displayAutoUpdate.enable"
                    } else {
                        "commands.scoreboard.objectives.modify.displayAutoUpdate.disable"
                    }
                }
                ObjectiveModify::NumberFormat(format) => {
                    objective.number_format = format.clone();
                    if format.is_some() {
                        "commands.scoreboard.objectives.modify.objectiveFormat.set"
                    } else {
                        "commands.scoreboard.objectives.modify.objectiveFormat.clear"
                    }
                }
                ObjectiveModify::FixedNumberFormat => {
                    objective.number_format = Some(NumberFormat::Fixed(
                        TextComponentArgConsumer::find_arg(args, ARG_CONTENTS)?,
                    ));
                    "commands.scoreboard.objectives.modify.objectiveFormat.set"
                }
                ObjectiveModify::StyledNumberFormat => {
                    objective.number_format =
                        Some(NumberFormat::Styled(find_style(args, ARG_STYLE)?));
                    "commands.scoreboard.objectives.modify.objectiveFormat.set"
                }
            };
            let display = objective.display_name.clone();
            scoreboard.update_objective(&world, objective).await;
            drop(scoreboard);
            let message_args = match &self.0 {
                ObjectiveModify::RenderType(_) => vec![display],
                ObjectiveModify::NumberFormat(_)
                | ObjectiveModify::FixedNumberFormat
                | ObjectiveModify::StyledNumberFormat => {
                    vec![TextComponent::text(name.to_string())]
                }
                ObjectiveModify::DisplayName | ObjectiveModify::DisplayAutoUpdate => {
                    vec![TextComponent::text(name.to_string()), display]
                }
            };
            send_translated(sender, key, message_args).await;
            Ok(0)
        })
    }
}

struct RemoveObjectiveExecutor;

impl CommandExecutor for RemoveObjectiveExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let name = simple_arg(args, ARG_OBJECTIVE)?;
            let world = command_world(sender, server)?;
            let mut scoreboard = world.scoreboard.lock().await;
            let objective = objective_or_error(&scoreboard, name)?;
            scoreboard.remove_objective(&world, name).await;
            let count = scoreboard.objective_names().len() as i32;
            drop(scoreboard);
            send_translated(
                sender,
                "commands.scoreboard.objectives.remove.success",
                [objective.display_name],
            )
            .await;
            Ok(count)
        })
    }
}

struct SetDisplayExecutor {
    clear: bool,
}

impl CommandExecutor for SetDisplayExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let slot_name = simple_arg(args, ARG_SLOT)?;
            let slot = parse_display_slot(slot_name).ok_or_else(|| {
                CommandError::CommandFailed(TextComponent::text("Unknown scoreboard display slot"))
            })?;
            let world = command_world(sender, server)?;
            let mut scoreboard = world.scoreboard.lock().await;
            if self.clear {
                if scoreboard.displayed_objective(&slot).is_none() {
                    return Err(translated_error(
                        "commands.scoreboard.objectives.display.alreadyEmpty",
                        [],
                    ));
                }
                scoreboard.set_display_slot(&world, slot, None).await;
                drop(scoreboard);
                send_translated(
                    sender,
                    "commands.scoreboard.objectives.display.cleared",
                    [TextComponent::text(slot_name.to_string())],
                )
                .await;
            } else {
                let name = simple_arg(args, ARG_OBJECTIVE)?;
                let objective = objective_or_error(&scoreboard, name)?;
                if scoreboard.displayed_objective(&slot) == Some(name) {
                    return Err(translated_error(
                        "commands.scoreboard.objectives.display.alreadySet",
                        [],
                    ));
                }
                scoreboard.set_display_slot(&world, slot, Some(name)).await;
                drop(scoreboard);
                send_translated(
                    sender,
                    "commands.scoreboard.objectives.display.set",
                    [
                        TextComponent::text(slot_name.to_string()),
                        objective.display_name,
                    ],
                )
                .await;
            }
            Ok(0)
        })
    }
}

struct ListPlayersExecutor {
    target: bool,
}

impl CommandExecutor for ListPlayersExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let world = command_world(sender, server)?;
            let scoreboard = world.scoreboard.lock().await;
            if !self.target {
                let holders = scoreboard.tracked_entities();
                if holders.is_empty() {
                    send_translated(sender, "commands.scoreboard.players.list.empty", []).await;
                } else {
                    send_translated(
                        sender,
                        "commands.scoreboard.players.list.success",
                        [
                            TextComponent::text(holders.len().to_string()),
                            component_list(holders.iter().cloned().map(TextComponent::text)),
                        ],
                    )
                    .await;
                }
                return Ok(holders.len() as i32);
            }

            let holders = expanded_holders(&scoreboard, score_holders(args, ARG_TARGET)?)?;
            if holders.len() != 1 {
                return Err(CommandError::CommandFailed(TextComponent::text(
                    "Only one score holder may be listed",
                )));
            }
            let holder = &holders[0];
            let scores = scoreboard.scores_for_entity(holder);
            if scores.is_empty() {
                send_translated(
                    sender,
                    "commands.scoreboard.players.list.entity.empty",
                    [TextComponent::text(holder.clone())],
                )
                .await;
            } else {
                send_translated(
                    sender,
                    "commands.scoreboard.players.list.entity.success",
                    [
                        TextComponent::text(holder.clone()),
                        TextComponent::text(scores.len().to_string()),
                    ],
                )
                .await;
                for score in &scores {
                    let objective = scoreboard
                        .objective(score.objective_name)
                        .expect("scores only exist for known objectives");
                    send_translated(
                        sender,
                        "commands.scoreboard.players.list.entity.entry",
                        [
                            objective.display_name.clone(),
                            TextComponent::text(score.value.0.to_string()),
                        ],
                    )
                    .await;
                }
            }
            Ok(scores.len() as i32)
        })
    }
}

struct GetScoreExecutor;

impl CommandExecutor for GetScoreExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let world = command_world(sender, server)?;
            let scoreboard = world.scoreboard.lock().await;
            let holders = expanded_holders(&scoreboard, score_holders(args, ARG_TARGET)?)?;
            if holders.len() != 1 {
                return Err(CommandError::CommandFailed(TextComponent::text(
                    "Only one score holder may be queried",
                )));
            }
            let objective_name = simple_arg(args, ARG_OBJECTIVE)?;
            let objective = objective_or_error(&scoreboard, objective_name)?;
            let holder = &holders[0];
            let score = scoreboard.score(holder, objective_name).ok_or_else(|| {
                translated_error(
                    "commands.scoreboard.players.get.null",
                    [
                        TextComponent::text(objective_name.to_string()),
                        TextComponent::text(holder.clone()),
                    ],
                )
            })?;
            let value = score.value.0;
            send_translated(
                sender,
                "commands.scoreboard.players.get.success",
                [
                    TextComponent::text(holder.clone()),
                    TextComponent::text(value.to_string()),
                    objective.display_name,
                ],
            )
            .await;
            Ok(value)
        })
    }
}

#[derive(Clone, Copy)]
enum ScoreMutation {
    Set,
    Add,
    Remove,
}

struct MutateScoreExecutor(ScoreMutation);

impl CommandExecutor for MutateScoreExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let amount = BoundedNumArgumentConsumer::<i32>::find_arg(args, ARG_SCORE)??;
            let objective_name = simple_arg(args, ARG_OBJECTIVE)?;
            let world = command_world(sender, server)?;
            let mut scoreboard = world.scoreboard.lock().await;
            let objective = writable_objective_or_error(&scoreboard, objective_name)?;
            let holders = expanded_holders(&scoreboard, score_holders(args, ARG_TARGETS)?)?;
            let mut result = 0_i32;
            for holder in &holders {
                let old = scoreboard
                    .score(holder, objective_name)
                    .map_or(0, |score| score.value.0);
                let value = match self.0 {
                    ScoreMutation::Set => amount,
                    ScoreMutation::Add => old.wrapping_add(amount),
                    ScoreMutation::Remove => old.wrapping_sub(amount),
                };
                result = result.wrapping_add(value);
                let score = score_with_value(&scoreboard, holder, &objective, value);
                scoreboard.update_score(&world, score).await;
            }
            let first = TextComponent::text(holders[0].clone());
            let key = match (self.0, holders.len() == 1) {
                (ScoreMutation::Set, true) => "commands.scoreboard.players.set.success.single",
                (ScoreMutation::Set, false) => "commands.scoreboard.players.set.success.multiple",
                (ScoreMutation::Add, true) => "commands.scoreboard.players.add.success.single",
                (ScoreMutation::Add, false) => "commands.scoreboard.players.add.success.multiple",
                (ScoreMutation::Remove, true) => {
                    "commands.scoreboard.players.remove.success.single"
                }
                (ScoreMutation::Remove, false) => {
                    "commands.scoreboard.players.remove.success.multiple"
                }
            };
            drop(scoreboard);
            let count_or_first = if holders.len() == 1 {
                first
            } else {
                TextComponent::text(holders.len().to_string())
            };
            let message_args = match self.0 {
                ScoreMutation::Set => vec![
                    objective.display_name,
                    count_or_first,
                    TextComponent::text(amount.to_string()),
                ],
                ScoreMutation::Add | ScoreMutation::Remove if holders.len() == 1 => vec![
                    TextComponent::text(amount.to_string()),
                    objective.display_name,
                    count_or_first,
                    TextComponent::text(result.to_string()),
                ],
                ScoreMutation::Add | ScoreMutation::Remove => vec![
                    TextComponent::text(amount.to_string()),
                    objective.display_name,
                    count_or_first,
                ],
            };
            send_translated(sender, key, message_args).await;
            Ok(result)
        })
    }
}

struct ResetScoreExecutor {
    objective: bool,
}

impl CommandExecutor for ResetScoreExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let world = command_world(sender, server)?;
            let mut scoreboard = world.scoreboard.lock().await;
            let holders = expanded_holders(&scoreboard, score_holders(args, ARG_TARGETS)?)?;
            let objective = if self.objective {
                Some(objective_or_error(
                    &scoreboard,
                    simple_arg(args, ARG_OBJECTIVE)?,
                )?)
            } else {
                None
            };
            for holder in &holders {
                if let Some(objective) = &objective {
                    scoreboard
                        .remove_score(&world, holder, objective.name)
                        .await;
                } else {
                    scoreboard.remove_all_scores(&world, holder).await;
                }
            }
            let key = match (objective.is_some(), holders.len() == 1) {
                (false, true) => "commands.scoreboard.players.reset.all.single",
                (false, false) => "commands.scoreboard.players.reset.all.multiple",
                (true, true) => "commands.scoreboard.players.reset.specific.single",
                (true, false) => "commands.scoreboard.players.reset.specific.multiple",
            };
            let mut message_args = vec![if holders.len() == 1 {
                TextComponent::text(holders[0].clone())
            } else {
                TextComponent::text(holders.len().to_string())
            }];
            if let Some(objective) = objective {
                message_args.insert(0, objective.display_name);
            }
            drop(scoreboard);
            send_translated(sender, key, message_args).await;
            Ok(holders.len() as i32)
        })
    }
}

struct EnableTriggerExecutor;

impl CommandExecutor for EnableTriggerExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let objective_name = simple_arg(args, ARG_OBJECTIVE)?;
            let world = command_world(sender, server)?;
            let mut scoreboard = world.scoreboard.lock().await;
            let objective = objective_or_error(&scoreboard, objective_name)?;
            if objective.criteria != "trigger" {
                return Err(translated_error(
                    "commands.scoreboard.players.enable.invalid",
                    [],
                ));
            }
            let holders = expanded_holders(&scoreboard, score_holders(args, ARG_TARGETS)?)?;
            let mut changed = 0;
            for holder in &holders {
                let mut score = score_with_value(
                    &scoreboard,
                    holder,
                    &objective,
                    scoreboard
                        .score(holder, objective_name)
                        .map_or(0, |score| score.value.0),
                );
                if !score.locked {
                    continue;
                }
                score.locked = false;
                scoreboard.update_score(&world, score).await;
                changed += 1;
            }
            if changed == 0 {
                return Err(translated_error(
                    "commands.scoreboard.players.enable.failed",
                    [],
                ));
            }
            let key = if holders.len() == 1 {
                "commands.scoreboard.players.enable.success.single"
            } else {
                "commands.scoreboard.players.enable.success.multiple"
            };
            drop(scoreboard);
            send_translated(
                sender,
                key,
                [
                    objective.display_name,
                    if holders.len() == 1 {
                        TextComponent::text(holders[0].clone())
                    } else {
                        TextComponent::text(holders.len().to_string())
                    },
                ],
            )
            .await;
            Ok(changed)
        })
    }
}

#[derive(Clone)]
enum ScoreDisplayMutation {
    Name(Option<TextComponent>),
    NameFromArgument,
    NumberFormat(Option<NumberFormat>),
    FixedNumberFormat,
    StyledNumberFormat,
}

struct ScoreDisplayExecutor(ScoreDisplayMutation);

impl CommandExecutor for ScoreDisplayExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let objective_name = simple_arg(args, ARG_OBJECTIVE)?;
            let world = command_world(sender, server)?;
            let mut scoreboard = world.scoreboard.lock().await;
            let objective = objective_or_error(&scoreboard, objective_name)?;
            let holders = expanded_holders(&scoreboard, score_holders(args, ARG_TARGETS)?)?;
            let supplied_name = if matches!(self.0, ScoreDisplayMutation::NameFromArgument) {
                Some(TextComponentArgConsumer::find_arg(args, ARG_NAME)?)
            } else {
                None
            };
            let (is_name, clearing) = match &self.0 {
                ScoreDisplayMutation::Name(value) => (true, value.is_none()),
                ScoreDisplayMutation::NameFromArgument => (true, false),
                ScoreDisplayMutation::NumberFormat(value) => (false, value.is_none()),
                ScoreDisplayMutation::FixedNumberFormat
                | ScoreDisplayMutation::StyledNumberFormat => (false, false),
            };
            for holder in &holders {
                let mut score = score_with_value(
                    &scoreboard,
                    holder,
                    &objective,
                    scoreboard
                        .score(holder, objective_name)
                        .map_or(0, |score| score.value.0),
                );
                match &self.0 {
                    ScoreDisplayMutation::Name(value) => score.display_name.clone_from(value),
                    ScoreDisplayMutation::NameFromArgument => {
                        score.display_name.clone_from(&supplied_name);
                    }
                    ScoreDisplayMutation::NumberFormat(value) => {
                        score.number_format.clone_from(value);
                    }
                    ScoreDisplayMutation::FixedNumberFormat => {
                        score.number_format = Some(NumberFormat::Fixed(
                            TextComponentArgConsumer::find_arg(args, ARG_CONTENTS)?,
                        ));
                    }
                    ScoreDisplayMutation::StyledNumberFormat => {
                        score.number_format =
                            Some(NumberFormat::Styled(find_style(args, ARG_STYLE)?));
                    }
                }
                scoreboard.update_score(&world, score).await;
            }
            let key = match (is_name, clearing, holders.len() == 1) {
                (true, true, true) => {
                    "commands.scoreboard.players.display.name.clear.success.single"
                }
                (true, true, false) => {
                    "commands.scoreboard.players.display.name.clear.success.multiple"
                }
                (true, false, true) => {
                    "commands.scoreboard.players.display.name.set.success.single"
                }
                (true, false, false) => {
                    "commands.scoreboard.players.display.name.set.success.multiple"
                }
                (false, true, true) => {
                    "commands.scoreboard.players.display.numberFormat.clear.success.single"
                }
                (false, true, false) => {
                    "commands.scoreboard.players.display.numberFormat.clear.success.multiple"
                }
                (false, false, true) => {
                    "commands.scoreboard.players.display.numberFormat.set.success.single"
                }
                (false, false, false) => {
                    "commands.scoreboard.players.display.numberFormat.set.success.multiple"
                }
            };
            drop(scoreboard);
            let holder_or_count = if holders.len() == 1 {
                TextComponent::text(holders[0].clone())
            } else {
                TextComponent::text(holders.len().to_string())
            };
            let message_args = if is_name && !clearing {
                vec![
                    supplied_name.expect("name display mutation has a supplied name"),
                    holder_or_count,
                    objective.display_name,
                ]
            } else {
                vec![holder_or_count, objective.display_name]
            };
            send_translated(sender, key, message_args).await;
            Ok(holders.len() as i32)
        })
    }
}

fn apply_operation(
    operation: &str,
    target: i32,
    source: i32,
) -> Result<(i32, Option<i32>), CommandError> {
    Ok(match operation {
        "=" => (source, None),
        "+=" => (target.wrapping_add(source), None),
        "-=" => (target.wrapping_sub(source), None),
        "*=" => (target.wrapping_mul(source), None),
        "/=" if source != 0 => (target.div_euclid(source), None),
        "%=" if source != 0 => (target.rem_euclid(source), None),
        "<" => (target.min(source), None),
        ">" => (target.max(source), None),
        "><" => (source, Some(target)),
        "/=" | "%=" => return Err(translated_error("arguments.operation.div0", [])),
        _ => return Err(translated_error("arguments.operation.invalid", [])),
    })
}

struct OperationExecutor;

impl CommandExecutor for OperationExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let target_objective_name = simple_arg(args, ARG_TARGET_OBJECTIVE)?;
            let source_objective_name = simple_arg(args, ARG_SOURCE_OBJECTIVE)?;
            let operation = simple_arg(args, ARG_OPERATION)?;
            let world = command_world(sender, server)?;
            let mut scoreboard = world.scoreboard.lock().await;
            let target_objective = writable_objective_or_error(&scoreboard, target_objective_name)?;
            let source_objective = objective_or_error(&scoreboard, source_objective_name)?;
            let targets = expanded_holders(&scoreboard, score_holders(args, ARG_TARGETS)?)?;
            let sources = expanded_holders(&scoreboard, score_holders(args, ARG_SOURCE)?)?;
            let mut result = 0_i32;
            for target in &targets {
                for source in &sources {
                    let target_value = scoreboard
                        .score(target, target_objective_name)
                        .map_or(0, |score| score.value.0);
                    let source_value = scoreboard
                        .score(source, source_objective_name)
                        .map_or(0, |score| score.value.0);
                    if scoreboard.score(source, source_objective_name).is_none() {
                        let source_score =
                            score_with_value(&scoreboard, source, &source_objective, source_value);
                        scoreboard.update_score(&world, source_score).await;
                    }
                    let (new_target, new_source) =
                        apply_operation(operation, target_value, source_value)?;
                    let target_score =
                        score_with_value(&scoreboard, target, &target_objective, new_target);
                    scoreboard.update_score(&world, target_score).await;
                    if let Some(new_source) = new_source {
                        let source_score =
                            score_with_value(&scoreboard, source, &source_objective, new_source);
                        scoreboard.update_score(&world, source_score).await;
                    }
                }
                result = result.wrapping_add(
                    scoreboard
                        .score(target, target_objective_name)
                        .map_or(0, |score| score.value.0),
                );
            }
            let key = if targets.len() == 1 {
                "commands.scoreboard.players.operation.success.single"
            } else {
                "commands.scoreboard.players.operation.success.multiple"
            };
            drop(scoreboard);
            let message_args = if targets.len() == 1 {
                vec![
                    target_objective.display_name,
                    TextComponent::text(targets[0].clone()),
                    TextComponent::text(result.to_string()),
                ]
            } else {
                vec![
                    target_objective.display_name,
                    TextComponent::text(targets.len().to_string()),
                ]
            };
            send_translated(sender, key, message_args).await;
            Ok(result)
        })
    }
}

fn objective_number_format_tree() -> crate::command::tree::builder::NonLeafNodeBuilder {
    literal("numberformat")
        .execute(ModifyObjectiveExecutor(ObjectiveModify::NumberFormat(None)))
        .then(
            literal("blank").execute(ModifyObjectiveExecutor(ObjectiveModify::NumberFormat(
                Some(NumberFormat::Blank),
            ))),
        )
        .then(
            literal("fixed").then(
                argument(ARG_CONTENTS, TextComponentArgConsumer)
                    .execute(ModifyObjectiveExecutor(ObjectiveModify::FixedNumberFormat)),
            ),
        )
        .then(
            literal("styled").then(
                argument(
                    ARG_STYLE,
                    scoreboard_argument(ScoreboardArgumentKind::Style),
                )
                .execute(ModifyObjectiveExecutor(ObjectiveModify::StyledNumberFormat)),
            ),
        )
}

fn player_number_format_tree() -> crate::command::tree::builder::NonLeafNodeBuilder {
    argument(
        ARG_OBJECTIVE,
        scoreboard_argument(ScoreboardArgumentKind::Objective),
    )
    .execute(ScoreDisplayExecutor(ScoreDisplayMutation::NumberFormat(
        None,
    )))
    .then(
        literal("blank").execute(ScoreDisplayExecutor(ScoreDisplayMutation::NumberFormat(
            Some(NumberFormat::Blank),
        ))),
    )
    .then(
        literal("fixed").then(argument(ARG_CONTENTS, TextComponentArgConsumer).execute(
            ScoreDisplayExecutor(ScoreDisplayMutation::FixedNumberFormat),
        )),
    )
    .then(
        literal("styled").then(
            argument(
                ARG_STYLE,
                scoreboard_argument(ScoreboardArgumentKind::Style),
            )
            .execute(ScoreDisplayExecutor(
                ScoreDisplayMutation::StyledNumberFormat,
            )),
        ),
    )
}

pub fn init_command_tree() -> CommandTree {
    CommandTree::new(NAMES, DESCRIPTION)
        .then(
            literal("objectives")
                .then(literal("list").execute(ListObjectivesExecutor))
                .then(
                    literal("add").then(
                        argument(ARG_OBJECTIVE, SimpleArgConsumer).then(
                            argument(
                                ARG_CRITERIA,
                                scoreboard_argument(ScoreboardArgumentKind::Criteria),
                            )
                            .execute(AddObjectiveExecutor {
                                custom_display_name: false,
                            })
                            .then(
                                argument(ARG_DISPLAY_NAME, TextComponentArgConsumer).execute(
                                    AddObjectiveExecutor {
                                        custom_display_name: true,
                                    },
                                ),
                            ),
                        ),
                    ),
                )
                .then(
                    literal("modify").then(
                        argument(
                            ARG_OBJECTIVE,
                            scoreboard_argument(ScoreboardArgumentKind::Objective),
                        )
                        .then(
                            literal("displayname").then(
                                argument(ARG_DISPLAY_NAME, TextComponentArgConsumer)
                                    .execute(ModifyObjectiveExecutor(ObjectiveModify::DisplayName)),
                            ),
                        )
                        .then(
                            literal("rendertype")
                                .then(literal("integer").execute(ModifyObjectiveExecutor(
                                    ObjectiveModify::RenderType(RenderType::Integer),
                                )))
                                .then(literal("hearts").execute(ModifyObjectiveExecutor(
                                    ObjectiveModify::RenderType(RenderType::Hearts),
                                ))),
                        )
                        .then(literal("displayautoupdate").then(
                            argument(ARG_VALUE, BoolArgConsumer).execute(ModifyObjectiveExecutor(
                                ObjectiveModify::DisplayAutoUpdate,
                            )),
                        ))
                        .then(objective_number_format_tree()),
                    ),
                )
                .then(
                    literal("remove").then(
                        argument(
                            ARG_OBJECTIVE,
                            scoreboard_argument(ScoreboardArgumentKind::Objective),
                        )
                        .execute(RemoveObjectiveExecutor),
                    ),
                )
                .then(
                    literal("setdisplay").then(
                        argument(ARG_SLOT, scoreboard_argument(ScoreboardArgumentKind::Slot))
                            .execute(SetDisplayExecutor { clear: true })
                            .then(
                                argument(
                                    ARG_OBJECTIVE,
                                    scoreboard_argument(ScoreboardArgumentKind::Objective),
                                )
                                .execute(SetDisplayExecutor { clear: false }),
                            ),
                    ),
                ),
        )
        .then(
            literal("players")
                .then(
                    literal("list")
                        .execute(ListPlayersExecutor { target: false })
                        .then(
                            argument(ARG_TARGET, ScoreHoldersArgumentConsumer::single())
                                .execute(ListPlayersExecutor { target: true }),
                        ),
                )
                .then(
                    literal("set").then(
                        argument(ARG_TARGETS, ScoreHoldersArgumentConsumer::multiple()).then(
                            argument(
                                ARG_OBJECTIVE,
                                scoreboard_argument(ScoreboardArgumentKind::Objective),
                            )
                            .then(
                                argument(ARG_SCORE, bounded_score(None))
                                    .execute(MutateScoreExecutor(ScoreMutation::Set)),
                            ),
                        ),
                    ),
                )
                .then(
                    literal("get").then(
                        argument(ARG_TARGET, ScoreHoldersArgumentConsumer::single()).then(
                            argument(
                                ARG_OBJECTIVE,
                                scoreboard_argument(ScoreboardArgumentKind::Objective),
                            )
                            .execute(GetScoreExecutor),
                        ),
                    ),
                )
                .then(
                    literal("add").then(
                        argument(ARG_TARGETS, ScoreHoldersArgumentConsumer::multiple()).then(
                            argument(
                                ARG_OBJECTIVE,
                                scoreboard_argument(ScoreboardArgumentKind::Objective),
                            )
                            .then(
                                argument(ARG_SCORE, bounded_score(Some(0)))
                                    .execute(MutateScoreExecutor(ScoreMutation::Add)),
                            ),
                        ),
                    ),
                )
                .then(
                    literal("remove").then(
                        argument(ARG_TARGETS, ScoreHoldersArgumentConsumer::multiple()).then(
                            argument(
                                ARG_OBJECTIVE,
                                scoreboard_argument(ScoreboardArgumentKind::Objective),
                            )
                            .then(
                                argument(ARG_SCORE, bounded_score(Some(0)))
                                    .execute(MutateScoreExecutor(ScoreMutation::Remove)),
                            ),
                        ),
                    ),
                )
                .then(
                    literal("reset").then(
                        argument(ARG_TARGETS, ScoreHoldersArgumentConsumer::multiple())
                            .execute(ResetScoreExecutor { objective: false })
                            .then(
                                argument(
                                    ARG_OBJECTIVE,
                                    scoreboard_argument(ScoreboardArgumentKind::Objective),
                                )
                                .execute(ResetScoreExecutor { objective: true }),
                            ),
                    ),
                )
                .then(
                    literal("enable").then(
                        argument(ARG_TARGETS, ScoreHoldersArgumentConsumer::multiple()).then(
                            argument(
                                ARG_OBJECTIVE,
                                scoreboard_argument(ScoreboardArgumentKind::Objective),
                            )
                            .execute(EnableTriggerExecutor),
                        ),
                    ),
                )
                .then(
                    literal("display")
                        .then(
                            literal("name").then(
                                argument(ARG_TARGETS, ScoreHoldersArgumentConsumer::multiple())
                                    .then(
                                        argument(
                                            ARG_OBJECTIVE,
                                            scoreboard_argument(ScoreboardArgumentKind::Objective),
                                        )
                                        .execute(ScoreDisplayExecutor(ScoreDisplayMutation::Name(
                                            None,
                                        )))
                                        .then(
                                            argument(ARG_NAME, TextComponentArgConsumer).execute(
                                                ScoreDisplayExecutor(
                                                    ScoreDisplayMutation::NameFromArgument,
                                                ),
                                            ),
                                        ),
                                    ),
                            ),
                        )
                        .then(
                            literal("numberformat").then(
                                argument(ARG_TARGETS, ScoreHoldersArgumentConsumer::multiple())
                                    .then(player_number_format_tree()),
                            ),
                        ),
                )
                .then(
                    literal("operation").then(
                        argument(ARG_TARGETS, ScoreHoldersArgumentConsumer::multiple()).then(
                            argument(
                                ARG_TARGET_OBJECTIVE,
                                scoreboard_argument(ScoreboardArgumentKind::Objective),
                            )
                            .then(
                                argument(
                                    ARG_OPERATION,
                                    scoreboard_argument(ScoreboardArgumentKind::Operation),
                                )
                                .then(
                                    argument(ARG_SOURCE, ScoreHoldersArgumentConsumer::multiple())
                                        .then(
                                            argument(
                                                ARG_SOURCE_OBJECTIVE,
                                                scoreboard_argument(
                                                    ScoreboardArgumentKind::Objective,
                                                ),
                                            )
                                            .execute(OperationExecutor),
                                        ),
                                ),
                            ),
                        ),
                    ),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::{apply_operation, init_command_tree, valid_criteria};
    use crate::command::tree::NodeType;

    fn command_paths() -> Vec<String> {
        let tree = init_command_tree();
        tree.iter_paths()
            .map(|path| {
                path.into_iter()
                    .filter_map(|index| match &tree.nodes[index].node_type {
                        NodeType::Literal { string } => Some(string.clone()),
                        NodeType::Argument { name, .. } => Some(format!("<{name}>")),
                        NodeType::ExecuteLeaf { .. } | NodeType::Require { .. } => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    }

    #[test]
    fn exposes_vanilla_objective_and_player_branches() {
        let paths = command_paths();
        for expected in [
            "objectives add <objective> <criteria>",
            "objectives setdisplay <slot> <objective>",
            "objectives modify <objective> rendertype hearts",
            "players set <targets> <objective> <score>",
            "players reset <targets> <objective>",
            "players operation <targets> <targetObjective> <operation> <source> <sourceObjective>",
            "players display name <targets> <objective> <name>",
        ] {
            assert!(
                paths.iter().any(|path| path == expected),
                "missing {expected}"
            );
        }
    }

    #[test]
    fn implements_vanilla_score_operations() {
        assert_eq!(apply_operation("=", 3, 7).unwrap(), (7, None));
        assert_eq!(
            apply_operation("+=", i32::MAX, 1).unwrap(),
            (i32::MIN, None)
        );
        assert_eq!(apply_operation("/=", -5, 2).unwrap(), (-3, None));
        assert_eq!(apply_operation("%=", -5, 2).unwrap(), (1, None));
        assert_eq!(apply_operation("><", 3, 7).unwrap(), (7, Some(3)));
        assert!(apply_operation("/=", 3, 0).is_err());
    }

    #[test]
    fn accepts_vanilla_base_and_stat_criteria() {
        assert!(valid_criteria("dummy"));
        assert!(valid_criteria("trigger"));
        assert!(valid_criteria("health"));
        assert!(valid_criteria("minecraft.mined:minecraft.stone"));
        assert!(!valid_criteria("not_a_criterion"));
    }
}
