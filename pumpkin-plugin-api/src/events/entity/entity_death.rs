use crate::wit::pumpkin::plugin::event::{EntityDeathEventData, Event, EventType};

use super::super::FromIntoEvent;

/// An event that occurs when an entity dies.
pub struct EntityDeathEvent;
impl FromIntoEvent for EntityDeathEvent {
    const EVENT_TYPE: EventType = EventType::EntityDeathEvent;
    type Data = EntityDeathEventData;

    fn data_from_event(event: Event) -> Self::Data {
        match event {
            Event::EntityDeathEvent(data) => data,
            _ => panic!("unexpected event"),
        }
    }

    fn data_into_event(data: Self::Data) -> Event {
        Event::EntityDeathEvent(data)
    }
}
