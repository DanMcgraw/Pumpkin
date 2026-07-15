use std::collections::HashSet;

use pumpkin_protocol::java::client::play::{ArgumentType, CommandSuggestion, SuggestionProviders};

use crate::{
    command::{
        CommandSender,
        args::{
            Arg, ArgumentConsumer, ConsumeResult, ConsumeResultWithSyntax, FindArg,
            GetClientSideArgParser, SuggestResult,
        },
        dispatcher::CommandError,
        errors::command_syntax_error::CommandSyntaxError,
        tree::{RawArg, RawArgs},
    },
    server::Server,
};

use super::entities::parse_target_selector_with_context;

/// Parses vanilla scoreboard holders: arbitrary names, `*`, or entity selectors.
pub struct ScoreHoldersArgumentConsumer {
    multiple: bool,
}

impl ScoreHoldersArgumentConsumer {
    #[must_use]
    pub const fn single() -> Self {
        Self { multiple: false }
    }

    #[must_use]
    pub const fn multiple() -> Self {
        Self { multiple: true }
    }

    fn resolve(
        &self,
        sender: &CommandSender,
        server: &Server,
        raw_arg: RawArg<'_>,
    ) -> Result<Vec<String>, CommandSyntaxError> {
        if !raw_arg.value.starts_with('@') {
            return Ok(vec![raw_arg.value.to_string()]);
        }

        let selector = parse_target_selector_with_context(raw_arg)?;
        let mut holders = Vec::new();
        for entity in server.select_entities(&selector, Some(sender)) {
            let uuid = entity.get_entity().entity_uuid;
            let name = server.get_player_by_uuid(uuid).map_or_else(
                || uuid.to_string(),
                |player| player.gameprofile.name.clone(),
            );
            if !holders.contains(&name) {
                holders.push(name);
            }
        }
        Ok(holders)
    }
}

impl GetClientSideArgParser for ScoreHoldersArgumentConsumer {
    fn get_client_side_parser(&self) -> ArgumentType {
        ArgumentType::ScoreHolder {
            flags: u8::from(self.multiple),
        }
    }

    fn get_client_side_suggestion_type_override(&self) -> Option<SuggestionProviders> {
        Some(SuggestionProviders::AskServer)
    }
}

impl ArgumentConsumer for ScoreHoldersArgumentConsumer {
    fn consume<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &mut RawArgs<'a>,
    ) -> ConsumeResult<'a> {
        let Some(raw_arg) = args.pop() else {
            return Box::pin(async { None });
        };
        let resolved = self.resolve(sender, server, raw_arg).ok();
        Box::pin(async move { resolved.map(Arg::ScoreHolders) })
    }

    fn consume_with_syntax<'a>(
        &'a self,
        sender: &'a CommandSender,
        server: &'a Server,
        args: &mut RawArgs<'a>,
    ) -> ConsumeResultWithSyntax<'a> {
        let Some(raw_arg) = args.pop() else {
            return Box::pin(async { Ok(None) });
        };
        let resolved = self.resolve(sender, server, raw_arg);
        Box::pin(async move { resolved.map(|holders| Some(Arg::ScoreHolders(holders))) })
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
            let mut names = server
                .get_all_players()
                .into_iter()
                .map(|player| player.gameprofile.name.clone())
                .collect::<HashSet<_>>();
            if let Some(world) = world {
                names.extend(world.scoreboard.lock().await.tracked_entities());
            }
            let mut names = names.into_iter().collect::<Vec<_>>();
            names.sort_unstable();
            let mut suggestions = vec![CommandSuggestion::new("*".to_string(), None)];
            suggestions.extend(
                names
                    .into_iter()
                    .map(|name| CommandSuggestion::new(name, None)),
            );
            Ok(Some(suggestions))
        })
    }
}

impl<'a> FindArg<'a> for ScoreHoldersArgumentConsumer {
    type Data = &'a [String];

    fn find_arg(args: &'a super::ConsumedArgs, name: &str) -> Result<Self::Data, CommandError> {
        match args.get(name) {
            Some(Arg::ScoreHolders(holders)) => Ok(holders),
            _ => Err(CommandError::InvalidConsumption(Some(name.to_string()))),
        }
    }
}
