use crate::wit::pumpkin::plugin::event::{EntitySpawnEventData, Event, EventType};

use super::super::FromIntoEvent;

/// An event that occurs when an entity is spawned into a world.
pub struct EntitySpawnEvent;
impl FromIntoEvent for EntitySpawnEvent {
    const EVENT_TYPE: EventType = EventType::EntitySpawnEvent;
    type Data = EntitySpawnEventData;

    fn data_from_event(event: Event) -> Self::Data {
        match event {
            Event::EntitySpawnEvent(data) => data,
            _ => panic!("unexpected event"),
        }
    }

    fn data_into_event(data: Self::Data) -> Event {
        Event::EntitySpawnEvent(data)
    }
}
