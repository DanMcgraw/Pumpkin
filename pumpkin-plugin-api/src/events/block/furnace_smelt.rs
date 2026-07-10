use crate::wit::pumpkin::plugin::event::{Event, EventType, FurnaceSmeltEventData};

use super::super::FromIntoEvent;

/// An event that occurs when a furnace smelts an item.
pub struct FurnaceSmeltEvent;
impl FromIntoEvent for FurnaceSmeltEvent {
    const EVENT_TYPE: EventType = EventType::FurnaceSmeltEvent;
    type Data = FurnaceSmeltEventData;

    fn data_from_event(event: Event) -> Self::Data {
        match event {
            Event::FurnaceSmeltEvent(data) => data,
            _ => panic!("unexpected event"),
        }
    }

    fn data_into_event(data: Self::Data) -> Event {
        Event::FurnaceSmeltEvent(data)
    }
}
