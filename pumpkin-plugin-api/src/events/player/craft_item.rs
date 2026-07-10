use crate::wit::pumpkin::plugin::event::{CraftItemEventData, Event, EventType};

use super::super::FromIntoEvent;

/// An event that occurs when a player crafts an item.
pub struct CraftItemEvent;
impl FromIntoEvent for CraftItemEvent {
    const EVENT_TYPE: EventType = EventType::CraftItemEvent;
    type Data = CraftItemEventData;

    fn data_from_event(event: Event) -> Self::Data {
        match event {
            Event::CraftItemEvent(data) => data,
            _ => panic!("unexpected event"),
        }
    }

    fn data_into_event(data: Self::Data) -> Event {
        Event::CraftItemEvent(data)
    }
}
