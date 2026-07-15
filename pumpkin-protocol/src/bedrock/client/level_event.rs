use pumpkin_macros::packet;
use pumpkin_util::math::vector3::Vector3;

use crate::{codec::var_int::VarInt, serial::PacketWrite};

#[derive(PacketWrite)]
#[packet(25)]
pub struct CLevelEvent {
    pub event_id: VarInt,
    pub position: Vector3<f32>,
    pub data: VarInt,
}

impl CLevelEvent {
    #[must_use]
    pub fn weather(event_id: LevelEvent, strength: f32) -> Self {
        Self {
            event_id: VarInt(event_id as i32),
            position: Vector3::default(),
            data: VarInt((strength.clamp(0.0, 1.0) * 65_535.0) as i32),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum LevelEvent {
    // There are hundreds of these, adding only what we need for now
    StartRaining = 3001,
    StartThunderstorm = 3002,
    StopRaining = 3003,
    StopThunderstorm = 3004,
    BlockStartBreak = 3600,
    BlockStopBreak = 3601,
    BlockUpdateBreak = 3602,
}

#[cfg(test)]
mod tests {
    use super::{CLevelEvent, LevelEvent};

    #[test]
    fn weather_events_use_bedrock_ids_and_scaled_strength() {
        let rain = CLevelEvent::weather(LevelEvent::StartRaining, 0.5);
        assert_eq!(rain.event_id.0, 3001);
        assert_eq!(rain.data.0, 32_767);
        assert_eq!(rain.position.x, 0.0);
        assert_eq!(rain.position.y, 0.0);
        assert_eq!(rain.position.z, 0.0);

        let stopped = CLevelEvent::weather(LevelEvent::StopThunderstorm, 2.0);
        assert_eq!(stopped.event_id.0, 3004);
        assert_eq!(stopped.data.0, 65_535);
    }
}
