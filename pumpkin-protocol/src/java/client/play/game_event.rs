use pumpkin_data::packet::clientbound::PLAY_GAME_EVENT;
use pumpkin_macros::java_packet;
use serde::{Deserialize, Serialize};

/// Updates the game state or triggers specific environmental changes.
///
/// This packet is the primary way the server communicates global or
/// context-specific transitions, such as changing the weather,
/// altering the player's gamemode, or displaying the credits.
#[derive(Serialize, Deserialize)]
#[java_packet(PLAY_GAME_EVENT)]
pub struct CGameEvent {
    /// The ID of the event type.
    pub event: u8,
    /// A value associated with the event (usage depends on the event ID).
    pub value: f32,
}

/// You need to implement all the random stuff somewhere, right?
impl CGameEvent {
    #[must_use]
    pub const fn new(event: GameEvent, value: f32) -> Self {
        Self {
            event: event as u8,
            value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum GameEvent {
    /// Vanilla `NO_RESPAWN_BLOCK_AVAILABLE`.
    NoRespawnBlockAvailable = 0,
    /// Vanilla `START_RAINING`.
    BeginRaining = 1,
    /// Vanilla `STOP_RAINING`.
    EndRaining = 2,
    /// Vanilla `CHANGE_GAME_MODE`.
    ChangeGameMode = 3,
    /// Vanilla `WIN_GAME`.
    WinGame = 4,
    /// Vanilla `DEMO_EVENT`.
    DemoEvent = 5,
    /// Vanilla `PLAY_ARROW_HIT_SOUND`.
    ArrowHitPlayer = 6,
    /// Vanilla `RAIN_LEVEL_CHANGE`.
    RainLevelChange = 7,
    /// Vanilla `THUNDER_LEVEL_CHANGE`.
    ThunderLevelChange = 8,
    /// Vanilla `PUFFER_FISH_STING`.
    PlayPufferfishStringSound = 9,
    /// Vanilla `GUARDIAN_ELDER_EFFECT`.
    PlayElderGuardianMobAppearance = 10,
    /// Vanilla `IMMEDIATE_RESPAWN`.
    EnabledRespawnScreen = 11,
    /// Vanilla `LIMITED_CRAFTING`.
    LimitedCrafting = 12,
    /// Vanilla `LEVEL_CHUNKS_LOAD_START`.
    StartWaitingChunks = 13,
}

#[cfg(test)]
mod tests {
    use super::GameEvent;

    #[test]
    fn game_event_ids_match_vanilla() {
        let ids = [
            GameEvent::NoRespawnBlockAvailable as u8,
            GameEvent::BeginRaining as u8,
            GameEvent::EndRaining as u8,
            GameEvent::ChangeGameMode as u8,
            GameEvent::WinGame as u8,
            GameEvent::DemoEvent as u8,
            GameEvent::ArrowHitPlayer as u8,
            GameEvent::RainLevelChange as u8,
            GameEvent::ThunderLevelChange as u8,
            GameEvent::PlayPufferfishStringSound as u8,
            GameEvent::PlayElderGuardianMobAppearance as u8,
            GameEvent::EnabledRespawnScreen as u8,
            GameEvent::LimitedCrafting as u8,
            GameEvent::StartWaitingChunks as u8,
        ];

        assert_eq!(ids, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]);
    }
}
