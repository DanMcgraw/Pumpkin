use crate::wit::pumpkin::plugin::event::{EntityExplodeEventData, Event, EventType};

use super::super::FromIntoEvent;

/// An event that occurs when an entity causes an explosion.
pub struct EntityExplodeEvent;
impl FromIntoEvent for EntityExplodeEvent {
    const EVENT_TYPE: EventType = EventType::EntityExplodeEvent;
    type Data = EntityExplodeEventData;

    fn data_from_event(event: Event) -> Self::Data {
        match event {
            Event::EntityExplodeEvent(data) => data,
            _ => panic!("unexpected event"),
        }
    }

    fn data_into_event(data: Self::Data) -> Event {
        Event::EntityExplodeEvent(data)
    }
}
