use std::{
    cmp::Ordering,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering as AtomicOrdering},
    },
};

use pumpkin_data::{BlockStateId, game_event::GameEvent};
use pumpkin_util::math::{position::BlockPos, vector3::Vector3};
use uuid::Uuid;

use crate::entity::EntityBase;

/// Context attached to a vanilla game event emission.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GameEventContext {
    pub source_entity: Option<Uuid>,
    pub affected_state: Option<BlockStateId>,
}

impl GameEventContext {
    #[must_use]
    pub const fn new(source_entity: Option<Uuid>, affected_state: Option<BlockStateId>) -> Self {
        Self {
            source_entity,
            affected_state,
        }
    }

    #[must_use]
    pub fn from_entity(entity: &dyn EntityBase) -> Self {
        Self {
            source_entity: Some(entity.get_entity().entity_uuid),
            affected_state: None,
        }
    }

    #[must_use]
    pub const fn with_affected_state(mut self, affected_state: BlockStateId) -> Self {
        self.affected_state = Some(affected_state);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GameEventListenerId(u64);

impl GameEventListenerId {
    #[cfg(test)]
    const fn raw(self) -> u64 {
        self.0
    }
}

/// Receives vanilla game event emissions from the world dispatcher.
pub trait GameEventListener: Send + Sync {
    fn position(&self) -> Vector3<f64>;

    fn listens_to(&self, _event: &GameEvent) -> bool {
        true
    }

    fn handle_game_event(
        &self,
        event: &GameEvent,
        position: Vector3<f64>,
        context: &GameEventContext,
    );
}

struct ListenerEntry {
    id: GameEventListenerId,
    listener: Arc<dyn GameEventListener>,
}

pub struct GameEventDispatcher {
    next_listener_id: AtomicU64,
    listeners: RwLock<Vec<ListenerEntry>>,
}

impl Default for GameEventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl GameEventDispatcher {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_listener_id: AtomicU64::new(1),
            listeners: RwLock::new(Vec::new()),
        }
    }

    pub fn register(&self, listener: Arc<dyn GameEventListener>) -> GameEventListenerId {
        let id = GameEventListenerId(self.next_listener_id.fetch_add(1, AtomicOrdering::Relaxed));
        self.listeners
            .write()
            .expect("game event listeners lock poisoned")
            .push(ListenerEntry { id, listener });
        id
    }

    pub fn unregister(&self, id: GameEventListenerId) -> bool {
        let mut listeners = self
            .listeners
            .write()
            .expect("game event listeners lock poisoned");
        let old_len = listeners.len();
        listeners.retain(|entry| entry.id != id);
        listeners.len() != old_len
    }

    pub fn emit(&self, event: GameEvent, position: Vector3<f64>, context: &GameEventContext) {
        let radius_squared = notification_radius_squared(&event);
        let mut listeners = self
            .listeners
            .read()
            .expect("game event listeners lock poisoned")
            .iter()
            .filter_map(|entry| {
                let listener_position = entry.listener.position();
                let distance_squared = squared_distance(position, listener_position);
                (distance_squared <= radius_squared && entry.listener.listens_to(&event))
                    .then(|| (distance_squared, entry.id, Arc::clone(&entry.listener)))
            })
            .collect::<Vec<_>>();

        listeners.sort_by(
            |(left_distance, left_id, _), (right_distance, right_id, _)| {
                left_distance
                    .partial_cmp(right_distance)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| left_id.0.cmp(&right_id.0))
            },
        );

        for (_, _, listener) in listeners {
            listener.handle_game_event(&event, position, context);
        }
    }
}

#[must_use]
pub fn block_position_center(position: BlockPos) -> Vector3<f64> {
    Vector3::new(
        f64::from(position.0.x) + 0.5,
        f64::from(position.0.y) + 0.5,
        f64::from(position.0.z) + 0.5,
    )
}

fn notification_radius_squared(event: &GameEvent) -> f64 {
    let radius = if matches!(event, &GameEvent::JukeboxPlay | &GameEvent::JukeboxStopPlay) {
        10.0
    } else if matches!(event, &GameEvent::Shriek) {
        32.0
    } else {
        16.0
    };
    radius * radius
}

fn squared_distance(left: Vector3<f64>, right: Vector3<f64>) -> f64 {
    let x = left.x - right.x;
    let y = left.y - right.y;
    let z = left.z - right.z;
    x * x + y * y + z * z
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct RecordingListener {
        name: &'static str,
        position: Vector3<f64>,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    impl RecordingListener {
        fn new(
            name: &'static str,
            position: Vector3<f64>,
            calls: Arc<Mutex<Vec<&'static str>>>,
        ) -> Self {
            Self {
                name,
                position,
                calls,
            }
        }
    }

    impl GameEventListener for RecordingListener {
        fn position(&self) -> Vector3<f64> {
            self.position
        }

        fn handle_game_event(
            &self,
            _event: &GameEvent,
            _position: Vector3<f64>,
            _context: &GameEventContext,
        ) {
            self.calls.lock().unwrap().push(self.name);
        }
    }

    #[test]
    fn dispatcher_notifies_nearest_listeners_first_inside_radius() {
        let dispatcher = GameEventDispatcher::default();
        let calls = Arc::new(Mutex::new(Vec::new()));

        dispatcher.register(Arc::new(RecordingListener::new(
            "far",
            Vector3::new(8.0, 0.0, 0.0),
            Arc::clone(&calls),
        )));
        dispatcher.register(Arc::new(RecordingListener::new(
            "near",
            Vector3::new(2.0, 0.0, 0.0),
            Arc::clone(&calls),
        )));
        dispatcher.register(Arc::new(RecordingListener::new(
            "outside",
            Vector3::new(16.1, 0.0, 0.0),
            Arc::clone(&calls),
        )));

        dispatcher.emit(
            GameEvent::Step,
            Vector3::new(0.0, 0.0, 0.0),
            &GameEventContext::default(),
        );

        assert_eq!(*calls.lock().unwrap(), ["near", "far"]);
    }

    #[test]
    fn dispatcher_uses_vanilla_special_radii() {
        let dispatcher = GameEventDispatcher::default();
        let calls = Arc::new(Mutex::new(Vec::new()));

        dispatcher.register(Arc::new(RecordingListener::new(
            "jukebox",
            Vector3::new(10.1, 0.0, 0.0),
            Arc::clone(&calls),
        )));
        dispatcher.register(Arc::new(RecordingListener::new(
            "shriek",
            Vector3::new(31.9, 0.0, 0.0),
            Arc::clone(&calls),
        )));

        dispatcher.emit(
            GameEvent::JukeboxPlay,
            Vector3::new(0.0, 0.0, 0.0),
            &GameEventContext::default(),
        );
        assert!(calls.lock().unwrap().is_empty());

        dispatcher.emit(
            GameEvent::Shriek,
            Vector3::new(0.0, 0.0, 0.0),
            &GameEventContext::default(),
        );
        assert_eq!(*calls.lock().unwrap(), ["jukebox", "shriek"]);
    }

    #[test]
    fn dispatcher_unregisters_listeners() {
        let dispatcher = GameEventDispatcher::default();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let id = dispatcher.register(Arc::new(RecordingListener::new(
            "removed",
            Vector3::new(0.0, 0.0, 0.0),
            Arc::clone(&calls),
        )));

        assert_eq!(id.raw(), 1);
        assert!(dispatcher.unregister(id));
        assert!(!dispatcher.unregister(id));

        dispatcher.emit(
            GameEvent::Step,
            Vector3::new(0.0, 0.0, 0.0),
            &GameEventContext::default(),
        );
        assert!(calls.lock().unwrap().is_empty());
    }
}
