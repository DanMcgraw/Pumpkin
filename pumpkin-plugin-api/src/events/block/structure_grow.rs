use crate::wit::pumpkin::plugin::event::{Event, EventType, StructureGrowEventData};

use super::super::FromIntoEvent;

/// An event that occurs before a structure grows from a sapling or mushroom.
pub struct StructureGrowEvent;
impl FromIntoEvent for StructureGrowEvent {
    const EVENT_TYPE: EventType = EventType::StructureGrowEvent;
    type Data = StructureGrowEventData;

    fn data_from_event(event: Event) -> Self::Data {
        match event {
            Event::StructureGrowEvent(data) => data,
            _ => panic!("unexpected event"),
        }
    }

    fn data_into_event(data: Self::Data) -> Event {
        Event::StructureGrowEvent(data)
    }
}
