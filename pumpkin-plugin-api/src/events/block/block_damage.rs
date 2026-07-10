use crate::wit::pumpkin::plugin::event::{BlockDamageEventData, Event, EventType};

use super::super::FromIntoEvent;

/// An event that occurs when a player starts damaging a block.
pub struct BlockDamageEvent;
impl FromIntoEvent for BlockDamageEvent {
    const EVENT_TYPE: EventType = EventType::BlockDamageEvent;
    type Data = BlockDamageEventData;

    fn data_from_event(event: Event) -> Self::Data {
        match event {
            Event::BlockDamageEvent(data) => data,
            _ => panic!("unexpected event"),
        }
    }

    fn data_into_event(data: Self::Data) -> Event {
        Event::BlockDamageEvent(data)
    }
}
