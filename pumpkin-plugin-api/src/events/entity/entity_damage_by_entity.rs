use crate::wit::pumpkin::plugin::event::{EntityDamageByEntityEventData, Event, EventType};

use super::super::FromIntoEvent;

/// An event that occurs when an entity is damaged by another entity.
pub struct EntityDamageByEntityEvent;
impl FromIntoEvent for EntityDamageByEntityEvent {
    const EVENT_TYPE: EventType = EventType::EntityDamageByEntityEvent;
    type Data = EntityDamageByEntityEventData;

    fn data_from_event(event: Event) -> Self::Data {
        match event {
            Event::EntityDamageByEntityEvent(data) => data,
            _ => panic!("unexpected event"),
        }
    }

    fn data_into_event(data: Self::Data) -> Event {
        Event::EntityDamageByEntityEvent(data)
    }
}
