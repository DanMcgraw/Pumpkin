use crate::wit::pumpkin::plugin::event::{BrewEventData, Event, EventType};

use super::super::FromIntoEvent;

/// An event that occurs when a brewing stand brews potions.
pub struct BrewEvent;
impl FromIntoEvent for BrewEvent {
    const EVENT_TYPE: EventType = EventType::BrewEvent;
    type Data = BrewEventData;

    fn data_from_event(event: Event) -> Self::Data {
        match event {
            Event::BrewEvent(data) => data,
            _ => panic!("unexpected event"),
        }
    }

    fn data_into_event(data: Self::Data) -> Event {
        Event::BrewEvent(data)
    }
}
