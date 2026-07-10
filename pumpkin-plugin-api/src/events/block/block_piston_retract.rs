use crate::wit::pumpkin::plugin::event::{BlockPistonRetractEventData, Event, EventType};

use super::super::FromIntoEvent;

/// An event that occurs when a piston is about to retract.
pub struct BlockPistonRetractEvent;
impl FromIntoEvent for BlockPistonRetractEvent {
    const EVENT_TYPE: EventType = EventType::BlockPistonRetractEvent;
    type Data = BlockPistonRetractEventData;

    fn data_from_event(event: Event) -> Self::Data {
        match event {
            Event::BlockPistonRetractEvent(data) => data,
            _ => panic!("unexpected event"),
        }
    }

    fn data_into_event(data: Self::Data) -> Event {
        Event::BlockPistonRetractEvent(data)
    }
}
