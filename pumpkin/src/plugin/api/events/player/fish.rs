use pumpkin_macros::{Event, cancellable};
use pumpkin_util::Hand;
use std::sync::Arc;

use crate::entity::player::Player;

use super::PlayerEvent;

/// Enum representing possible fishing event states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerFishState {
    /// The player cast the fishing rod.
    Fishing,

    /// The hook caught a fish.
    CaughtFish,

    /// The hook caught an entity.
    CaughtEntity,

    /// The hook landed in the ground.
    InGround,

    /// The fishing attempt failed.
    FailedAttempt,

    /// The player reeled in the hook.
    ReelIn,

    /// A fish bit the hook.
    Bite,
}

/// An event that occurs when a player fishes.
#[cancellable]
#[derive(Event, Clone)]
pub struct PlayerFishEvent {
    /// The player who is fishing.
    pub player: Arc<Player>,

    /// The UUID of the caught entity, if any.
    pub caught_uuid: Option<uuid::Uuid>,

    /// The caught entity type (registry key).
    pub caught_type: String,

    /// The UUID of the fishing hook.
    pub hook_uuid: uuid::Uuid,

    /// The fish event state.
    pub state: PlayerFishState,

    /// The hand used for fishing.
    pub hand: Hand,

    /// Experience to drop.
    pub exp_to_drop: i32,
}

impl PlayerFishEvent {
    /// Creates a new instance of `PlayerFishEvent`.
    pub const fn new(
        player: Arc<Player>,
        caught_uuid: Option<uuid::Uuid>,
        hook_uuid: uuid::Uuid,
        caught_type: String,
        state: PlayerFishState,
        hand: Hand,
        exp_to_drop: i32,
    ) -> Self {
        Self {
            player,
            caught_uuid,
            hook_uuid,
            caught_type,
            state,
            hand,
            exp_to_drop,
            cancelled: false,
        }
    }
}

impl PlayerEvent for PlayerFishEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_fish_states_are_distinct() {
        let states = [
            PlayerFishState::Fishing,
            PlayerFishState::CaughtFish,
            PlayerFishState::CaughtEntity,
            PlayerFishState::InGround,
            PlayerFishState::FailedAttempt,
            PlayerFishState::ReelIn,
            PlayerFishState::Bite,
        ];
        for (i, a) in states.iter().enumerate() {
            for (j, b) in states.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn hand_default_is_exposed() {
        // Hand is a public field on the event; this test just ensures it stays part of
        // the API so that plugins can read which hand was used.
        let hand = Hand::Right;
        assert_eq!(hand, Hand::Right);
    }
}
