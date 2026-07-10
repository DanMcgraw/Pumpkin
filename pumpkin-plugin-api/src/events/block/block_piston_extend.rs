use crate::wit::pumpkin::plugin::event::{BlockPistonExtendEventData, Event, EventType};

use super::super::FromIntoEvent;

/// An event that occurs when a piston is about to extend.
pub struct BlockPistonExtendEvent;
impl FromIntoEvent for BlockPistonExtendEvent {
    const EVENT_TYPE: EventType = EventType::BlockPistonExtendEvent;
    type Data = BlockPistonExtendEventData;

    fn data_from_event(event: Event) -> Self::Data {
        match event {
            Event::BlockPistonExtendEvent(data) => data,
            _ => panic!("unexpected event"),
        }
    }

    fn data_into_event(data: Self::Data) -> Event {
        Event::BlockPistonExtendEvent(data)
    }
}
