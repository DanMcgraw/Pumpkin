use super::World;
use pumpkin_protocol::bedrock::client::{CLevelEvent, LevelEvent};
use pumpkin_protocol::java::client::play::{CGameEvent, GameEvent};
use rand::RngExt;

// Weather timing constants
const RAIN_DELAY_MIN: i32 = 12_000;
const RAIN_DELAY_MAX: i32 = 180_000;
const RAIN_DURATION_MIN: i32 = 12_000;
const RAIN_DURATION_MAX: i32 = 24_000;
const THUNDER_DELAY_MIN: i32 = 12_000;
const THUNDER_DELAY_MAX: i32 = 180_000;
const THUNDER_DURATION_MIN: i32 = 3_600;
const THUNDER_DURATION_MAX: i32 = 15_600;

const WEATHER_TRANSITION_SPEED: f32 = 0.01;

pub struct Weather {
    pub clear_weather_time: i32,
    pub raining: bool,
    pub rain_time: i32,
    pub thundering: bool,
    pub thunder_time: i32,

    pub rain_level: f32,
    pub old_rain_level: f32,
    pub thunder_level: f32,
    pub old_thunder_level: f32,

    pub weather_cycle_enabled: bool,
}

impl Default for Weather {
    fn default() -> Self {
        Self::new()
    }
}

impl Weather {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            clear_weather_time: 0,
            raining: false,
            rain_time: 0,
            thundering: false,
            thunder_time: 0,
            rain_level: 0.0,
            old_rain_level: 0.0,
            thunder_level: 0.0,
            old_thunder_level: 0.0,
            weather_cycle_enabled: true,
        }
    }

    pub fn set_weather_parameters(
        &mut self,
        world: &World,
        clear_time: i32,
        rain_time: i32,
        raining: bool,
        thundering: bool,
    ) {
        let was_raining = self.raining;

        self.clear_weather_time = clear_time;
        self.rain_time = rain_time;
        self.thunder_time = rain_time;
        self.raining = raining;
        self.thundering = thundering;

        if was_raining != raining {
            let java_packet = if was_raining {
                CGameEvent::new(GameEvent::EndRaining, 0.0)
            } else {
                CGameEvent::new(GameEvent::BeginRaining, 0.0)
            };
            world.broadcast_packet_except_editioned_sync(
                &[],
                &java_packet,
                &Self::bedrock_rain_packet(self.rain_level),
            );
        }
    }

    pub fn tick_weather(&mut self, world: &World) {
        self.advance_weather_cycle_if_enabled();

        // Update visual transitions
        self.old_rain_level = self.rain_level;
        self.old_thunder_level = self.thunder_level;

        if self.raining {
            self.rain_level = (self.rain_level + WEATHER_TRANSITION_SPEED).min(1.0);
        } else {
            self.rain_level = (self.rain_level - WEATHER_TRANSITION_SPEED).max(0.0);
        }

        if self.thundering {
            self.thunder_level = (self.thunder_level + WEATHER_TRANSITION_SPEED).min(1.0);
        } else {
            self.thunder_level = (self.thunder_level - WEATHER_TRANSITION_SPEED).max(0.0);
        }

        // Broadcast level changes if needed
        if (self.old_rain_level - self.rain_level).abs() > f32::EPSILON {
            world.broadcast_packet_except_editioned_sync(
                &[],
                &CGameEvent::new(GameEvent::RainLevelChange, self.rain_level),
                &Self::bedrock_rain_packet(self.rain_level),
            );

            let rain_started = self.old_rain_level <= 0.0 && self.rain_level > 0.0;
            let rain_stopped = self.old_rain_level > 0.0 && self.rain_level <= 0.0;
            let thunder_level_unchanged =
                (self.old_thunder_level - self.thunder_level).abs() <= f32::EPSILON;
            if thunder_level_unchanged
                && self.thunder_level > 0.0
                && (rain_started || rain_stopped)
            {
                world.broadcast_packet_bedrock_sync(&Self::bedrock_thunder_packet(
                    self.rain_level,
                    self.thunder_level,
                ));
            }
        }

        if (self.old_thunder_level - self.thunder_level).abs() > f32::EPSILON {
            world.broadcast_packet_except_editioned_sync(
                &[],
                &CGameEvent::new(GameEvent::ThunderLevelChange, self.thunder_level),
                &Self::bedrock_thunder_packet(self.rain_level, self.thunder_level),
            );
        }
    }

    fn advance_weather_cycle_if_enabled(&mut self) {
        if self.weather_cycle_enabled {
            self.advance_weather_cycle();
        }
    }

    #[must_use]
    pub fn bedrock_rain_packet(rain_level: f32) -> CLevelEvent {
        let event = if rain_level > 0.0 {
            LevelEvent::StartRaining
        } else {
            LevelEvent::StopRaining
        };
        CLevelEvent::weather(event, rain_level)
    }

    #[must_use]
    pub fn bedrock_thunder_packet(rain_level: f32, thunder_level: f32) -> CLevelEvent {
        if rain_level > 0.0 && thunder_level > 0.0 {
            CLevelEvent::weather(LevelEvent::StartThunderstorm, thunder_level)
        } else {
            CLevelEvent::weather(LevelEvent::StopThunderstorm, 0.0)
        }
    }

    fn advance_weather_cycle(&mut self) {
        // Removed async since there are no await calls
        if self.clear_weather_time > 0 {
            self.clear_weather_time -= 1;
            self.thunder_time = i32::from(!self.thundering);
            self.rain_time = i32::from(!self.raining);
            self.thundering = false;
            self.raining = false;
        } else {
            // Handle thunder timing
            if self.thunder_time > 0 {
                self.thunder_time -= 1;
                if self.thunder_time == 0 {
                    self.thundering = !self.thundering;
                }
            } else if self.thundering {
                self.thunder_time =
                    rand::rng().random_range(THUNDER_DURATION_MIN..=THUNDER_DURATION_MAX);
            } else {
                self.thunder_time = rand::rng().random_range(THUNDER_DELAY_MIN..=THUNDER_DELAY_MAX);
            }

            // Handle rain timing
            if self.rain_time > 0 {
                self.rain_time -= 1;
                if self.rain_time == 0 {
                    self.raining = !self.raining;
                }
            } else if self.raining {
                self.rain_time = rand::rng().random_range(RAIN_DURATION_MIN..=RAIN_DURATION_MAX);
            } else {
                self.rain_time = rand::rng().random_range(RAIN_DELAY_MIN..=RAIN_DELAY_MAX);
            }
        }
    }

    pub fn reset_weather_cycle(&mut self, world: &World) {
        self.set_weather_parameters(world, 0, 0, false, false);
    }
}

impl Clone for Weather {
    fn clone(&self) -> Self {
        Self {
            clear_weather_time: self.clear_weather_time,
            raining: self.raining,
            rain_time: self.rain_time,
            thundering: self.thundering,
            thunder_time: self.thunder_time,
            rain_level: self.rain_level,
            old_rain_level: self.old_rain_level,
            thunder_level: self.thunder_level,
            old_thunder_level: self.old_thunder_level,
            weather_cycle_enabled: self.weather_cycle_enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Weather;

    #[test]
    fn weather_cycle_only_advances_when_enabled() {
        let mut weather = Weather::new();
        weather.rain_time = 1;
        weather.thunder_time = 2;
        weather.weather_cycle_enabled = false;
        weather.advance_weather_cycle_if_enabled();
        assert_eq!(weather.rain_time, 1);
        assert!(!weather.raining);

        weather.weather_cycle_enabled = true;
        weather.advance_weather_cycle_if_enabled();
        assert_eq!(weather.rain_time, 0);
        assert!(weather.raining);
    }

    #[test]
    fn bedrock_thunder_requires_active_rain() {
        let stopped = Weather::bedrock_thunder_packet(0.0, 1.0);
        assert_eq!(stopped.event_id.0, 3004);
        assert_eq!(stopped.data.0, 0);

        let started = Weather::bedrock_thunder_packet(1.0, 0.5);
        assert_eq!(started.event_id.0, 3002);
        assert_eq!(started.data.0, 32_767);
    }
}
