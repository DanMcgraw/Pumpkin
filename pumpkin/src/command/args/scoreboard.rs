use pumpkin_nbt::tag::NbtTag;
use pumpkin_protocol::java::client::play::{ArgumentType, CommandSuggestion, SuggestionProviders};
use pumpkin_util::text::style::Style;

use crate::{
    command::{
        CommandSender,
        args::{Arg, ArgumentConsumer, ConsumeResult, GetClientSideArgParser, SuggestResult},
        dispatcher::CommandError,
        snbt::SnbtParser,
        string_reader::StringReader,
        tree::RawArgs,
    },
    server::Server,
};

#[derive(Clone, Copy)]
pub enum ScoreboardArgumentKind {
    Objective,
    Criteria,
    Operation,
    Slot,
    Style,
}

pub struct ScoreboardArgumentConsumer(pub ScoreboardArgumentKind);

impl GetClientSideArgParser for ScoreboardArgumentConsumer {
    fn get_client_side_parser(&self) -> ArgumentType {
        match self.0 {
            ScoreboardArgumentKind::Objective => ArgumentType::Objective,
            ScoreboardArgumentKind::Criteria => ArgumentType::ObjectiveCriteria,
            ScoreboardArgumentKind::Operation => ArgumentType::Operation,
            ScoreboardArgumentKind::Slot => ArgumentType::ScoreboardSlot,
            ScoreboardArgumentKind::Style => ArgumentType::Style,
        }
    }

    fn get_client_side_suggestion_type_override(&self) -> Option<SuggestionProviders> {
        (!matches!(self.0, ScoreboardArgumentKind::Style)).then_some(SuggestionProviders::AskServer)
    }
}

impl ArgumentConsumer for ScoreboardArgumentConsumer {
    fn consume<'a>(
        &'a self,
        _sender: &'a CommandSender,
        _server: &'a Server,
        args: &mut RawArgs<'a>,
    ) -> ConsumeResult<'a> {
        let Some(value) = args.pop().map(|arg| arg.value) else {
            return Box::pin(async { None });
        };
        let arg = match self.0 {
            ScoreboardArgumentKind::Operation
                if !matches!(
                    value,
                    "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "<" | ">" | "><"
                ) =>
            {
                None
            }
            ScoreboardArgumentKind::Slot if parse_display_slot(value).is_none() => None,
            ScoreboardArgumentKind::Style => parse_style(value).map(Arg::Style),
            _ => Some(Arg::Simple(value)),
        };
        Box::pin(async move { arg })
    }

    fn suggest<'a>(
        &'a self,
        sender: &CommandSender,
        server: &'a Server,
        _input: &'a str,
    ) -> SuggestResult<'a> {
        let world = sender
            .world()
            .or_else(|| server.worlds.load().first().cloned());
        Box::pin(async move {
            let values = match self.0 {
                ScoreboardArgumentKind::Objective => {
                    if let Some(world) = world {
                        world.scoreboard.lock().await.objective_names()
                    } else {
                        Vec::new()
                    }
                }
                ScoreboardArgumentKind::Criteria => vec![
                    "dummy".to_string(),
                    "trigger".to_string(),
                    "deathCount".to_string(),
                    "playerKillCount".to_string(),
                    "totalKillCount".to_string(),
                    "health".to_string(),
                    "food".to_string(),
                    "air".to_string(),
                    "armor".to_string(),
                    "xp".to_string(),
                    "level".to_string(),
                ],
                ScoreboardArgumentKind::Operation => {
                    ["=", "+=", "-=", "*=", "/=", "%=", "<", ">", "><"]
                        .into_iter()
                        .map(str::to_string)
                        .collect()
                }
                ScoreboardArgumentKind::Slot => {
                    DISPLAY_SLOT_NAMES.into_iter().map(str::to_string).collect()
                }
                ScoreboardArgumentKind::Style => Vec::new(),
            };
            Ok(Some(
                values
                    .into_iter()
                    .map(|value| CommandSuggestion::new(value, None))
                    .collect(),
            ))
        })
    }
}

fn nbt_to_json(tag: NbtTag) -> Option<serde_json::Value> {
    use serde_json::{Map, Number, Value};

    Some(match tag {
        NbtTag::End => Value::Null,
        NbtTag::Byte(value) if matches!(value, 0 | 1) => Value::Bool(value != 0),
        NbtTag::Byte(value) => Value::Number(Number::from(value)),
        NbtTag::Short(value) => Value::Number(Number::from(value)),
        NbtTag::Int(value) => Value::Number(Number::from(value)),
        NbtTag::Long(value) => Value::Number(Number::from(value)),
        NbtTag::Float(value) => Value::Number(Number::from_f64(f64::from(value))?),
        NbtTag::Double(value) => Value::Number(Number::from_f64(value)?),
        NbtTag::String(value) => Value::String(value.into()),
        NbtTag::List(values) => Value::Array(
            values
                .into_iter()
                .map(nbt_to_json)
                .collect::<Option<Vec<_>>>()?,
        ),
        NbtTag::Compound(compound) => Value::Object(
            compound
                .child_tags
                .into_iter()
                .map(|(key, value)| Some((key.into(), nbt_to_json(value)?)))
                .collect::<Option<Map<_, _>>>()?,
        ),
        NbtTag::ByteArray(values) => Value::Array(
            values
                .into_vec()
                .into_iter()
                .map(|value| Value::Number(Number::from(value)))
                .collect(),
        ),
        NbtTag::IntArray(values) => Value::Array(
            values
                .into_iter()
                .map(|value| Value::Number(Number::from(value)))
                .collect(),
        ),
        NbtTag::LongArray(values) => Value::Array(
            values
                .into_iter()
                .map(|value| Value::Number(Number::from(value)))
                .collect(),
        ),
    })
}

fn parse_style(value: &str) -> Option<Style> {
    if let Ok(style) = serde_json::from_str(value) {
        return Some(style);
    }
    let tag = SnbtParser::parse_for_commands(&mut StringReader::new(value)).ok()?;
    serde_json::from_value(nbt_to_json(tag)?).ok()
}

pub fn find_style(args: &super::ConsumedArgs, name: &str) -> Result<Style, CommandError> {
    match args.get(name) {
        Some(Arg::Style(style)) => Ok(style.clone()),
        _ => Err(CommandError::InvalidConsumption(Some(name.to_string()))),
    }
}

pub const DISPLAY_SLOT_NAMES: [&str; 19] = [
    "list",
    "sidebar",
    "below_name",
    "sidebar.team.black",
    "sidebar.team.dark_blue",
    "sidebar.team.dark_green",
    "sidebar.team.dark_aqua",
    "sidebar.team.dark_red",
    "sidebar.team.dark_purple",
    "sidebar.team.gold",
    "sidebar.team.gray",
    "sidebar.team.dark_gray",
    "sidebar.team.blue",
    "sidebar.team.green",
    "sidebar.team.aqua",
    "sidebar.team.red",
    "sidebar.team.light_purple",
    "sidebar.team.yellow",
    "sidebar.team.white",
];

#[must_use]
pub fn parse_display_slot(value: &str) -> Option<pumpkin_data::scoreboard::ScoreboardDisplaySlot> {
    use pumpkin_data::scoreboard::ScoreboardDisplaySlot;
    Some(match value {
        "list" => ScoreboardDisplaySlot::List,
        "sidebar" => ScoreboardDisplaySlot::Sidebar,
        "below_name" => ScoreboardDisplaySlot::BelowName,
        "sidebar.team.black" => ScoreboardDisplaySlot::TeamBlack,
        "sidebar.team.dark_blue" => ScoreboardDisplaySlot::TeamDarkBlue,
        "sidebar.team.dark_green" => ScoreboardDisplaySlot::TeamDarkGreen,
        "sidebar.team.dark_aqua" => ScoreboardDisplaySlot::TeamDarkAqua,
        "sidebar.team.dark_red" => ScoreboardDisplaySlot::TeamDarkRed,
        "sidebar.team.dark_purple" => ScoreboardDisplaySlot::TeamDarkPurple,
        "sidebar.team.gold" => ScoreboardDisplaySlot::TeamGold,
        "sidebar.team.gray" => ScoreboardDisplaySlot::TeamGray,
        "sidebar.team.dark_gray" => ScoreboardDisplaySlot::TeamDarkGray,
        "sidebar.team.blue" => ScoreboardDisplaySlot::TeamBlue,
        "sidebar.team.green" => ScoreboardDisplaySlot::TeamGreen,
        "sidebar.team.aqua" => ScoreboardDisplaySlot::TeamAqua,
        "sidebar.team.red" => ScoreboardDisplaySlot::TeamRed,
        "sidebar.team.light_purple" => ScoreboardDisplaySlot::TeamLightPurple,
        "sidebar.team.yellow" => ScoreboardDisplaySlot::TeamYellow,
        "sidebar.team.white" => ScoreboardDisplaySlot::TeamWhite,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use pumpkin_util::text::color::{Color, NamedColor};

    use super::parse_style;

    #[test]
    fn parses_vanilla_snbt_style_syntax() {
        let style = parse_style("{bold: true, color: 'red'}").expect("valid vanilla style");
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.color, Some(Color::Named(NamedColor::Red)));
    }
}
