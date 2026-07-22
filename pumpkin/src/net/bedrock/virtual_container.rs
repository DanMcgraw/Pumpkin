use pumpkin_data::{BlockStateId, screen::WindowType};
use pumpkin_util::math::position::BlockPos;

/// Bedrock only exposes three-row and six-row chest screens. A logical Java
/// chest is therefore presented inside one of those fixed physical capacities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualContainerLayout {
    logical_size: usize,
    physical_capacity: usize,
}

impl VirtualContainerLayout {
    pub const SINGLE_CHEST_CAPACITY: usize = 27;
    pub const DOUBLE_CHEST_CAPACITY: usize = 54;

    #[must_use]
    pub const fn new(logical_size: usize) -> Option<Self> {
        if logical_size == 0 || logical_size > Self::DOUBLE_CHEST_CAPACITY {
            return None;
        }
        if !logical_size.is_multiple_of(9) {
            return None;
        }

        let physical_capacity = if logical_size <= Self::SINGLE_CHEST_CAPACITY {
            Self::SINGLE_CHEST_CAPACITY
        } else {
            Self::DOUBLE_CHEST_CAPACITY
        };
        Some(Self {
            logical_size,
            physical_capacity,
        })
    }

    #[must_use]
    pub const fn from_window_type(window_type: WindowType) -> Option<Self> {
        let logical_size = match window_type {
            WindowType::Generic9x1 => 9,
            WindowType::Generic9x2 => 18,
            WindowType::Generic9x3 => 27,
            WindowType::Generic9x4 => 36,
            WindowType::Generic9x5 => 45,
            WindowType::Generic9x6 => 54,
            _ => return None,
        };
        Self::new(logical_size)
    }

    #[must_use]
    pub const fn logical_size(self) -> usize {
        self.logical_size
    }

    #[must_use]
    pub const fn physical_capacity(self) -> usize {
        self.physical_capacity
    }

    #[must_use]
    pub const fn is_double(self) -> bool {
        self.physical_capacity == Self::DOUBLE_CHEST_CAPACITY
    }

    #[must_use]
    pub const fn contains_logical_slot(self, slot: usize) -> bool {
        slot < self.logical_size
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualContainerPhase {
    AwaitingAcknowledgement,
    RetryDelay,
    Open,
}

/// Per-connection state for a client-only chest holder.
#[derive(Clone, Debug)]
pub struct VirtualContainerSession {
    pub sync_id: u8,
    pub layout: VirtualContainerLayout,
    pub holder_positions: Vec<BlockPos>,
    pub original_states: Vec<BlockStateId>,
    pub acknowledgement_timestamp: i64,
    pub prepared_tick: i32,
    pub open_attempts: u8,
    pub phase: VirtualContainerPhase,
}

impl VirtualContainerSession {
    pub const FALLBACK_OPEN_DELAY_TICKS: i32 = 4;
    pub const RETRY_DELAY_TICKS: i32 = 3;
    pub const MAX_OPEN_ATTEMPTS: u8 = 7;

    #[must_use]
    pub fn matches_acknowledgement(&self, timestamp: i64) -> bool {
        self.phase == VirtualContainerPhase::AwaitingAcknowledgement
            && self.acknowledgement_timestamp == timestamp
    }

    #[must_use]
    pub fn should_fallback_open(&self, current_tick: i32) -> bool {
        self.phase == VirtualContainerPhase::AwaitingAcknowledgement
            && current_tick.wrapping_sub(self.prepared_tick) >= Self::FALLBACK_OPEN_DELAY_TICKS
    }

    /// Bedrock reports a rejected container open with container ID `-1`.
    /// Keep the client-side holder alive and retry after the same 150 ms delay
    /// used by Geyser instead of treating the rejection as a normal close.
    pub fn defer_rejected_open(&mut self, current_tick: i32) -> bool {
        if self.phase != VirtualContainerPhase::Open
            || self.open_attempts >= Self::MAX_OPEN_ATTEMPTS
        {
            return false;
        }

        self.phase = VirtualContainerPhase::RetryDelay;
        self.prepared_tick = current_tick;
        self.open_attempts += 1;
        true
    }

    #[must_use]
    pub fn should_begin_retry(&self, current_tick: i32) -> bool {
        self.phase == VirtualContainerPhase::RetryDelay
            && current_tick.wrapping_sub(self.prepared_tick) >= Self::RETRY_DELAY_TICKS
    }

    pub fn begin_retry(&mut self, timestamp: i64, current_tick: i32) {
        self.acknowledgement_timestamp = timestamp;
        self.prepared_tick = current_tick;
        self.phase = VirtualContainerPhase::AwaitingAcknowledgement;
    }
}

#[cfg(test)]
mod tests {
    use pumpkin_data::screen::WindowType;

    use super::{VirtualContainerLayout, VirtualContainerPhase, VirtualContainerSession};

    #[test]
    fn chest_rows_select_the_smallest_bedrock_capacity() {
        for size in [9, 18, 27] {
            let layout = VirtualContainerLayout::new(size).unwrap();
            assert_eq!(layout.physical_capacity(), 27);
            assert!(!layout.is_double());
        }
        for size in [36, 45, 54] {
            let layout = VirtualContainerLayout::new(size).unwrap();
            assert_eq!(layout.physical_capacity(), 54);
            assert!(layout.is_double());
        }
    }

    #[test]
    fn invalid_chest_shapes_are_rejected() {
        for size in [0, 1, 8, 10, 53, 55] {
            assert!(VirtualContainerLayout::new(size).is_none(), "size={size}");
        }
    }

    #[test]
    fn window_types_preserve_their_logical_rows() {
        let cases = [
            (WindowType::Generic9x1, 9),
            (WindowType::Generic9x2, 18),
            (WindowType::Generic9x3, 27),
            (WindowType::Generic9x4, 36),
            (WindowType::Generic9x5, 45),
            (WindowType::Generic9x6, 54),
        ];
        for (window_type, expected) in cases {
            assert_eq!(
                VirtualContainerLayout::from_window_type(window_type)
                    .unwrap()
                    .logical_size(),
                expected
            );
        }
        assert!(VirtualContainerLayout::from_window_type(WindowType::Hopper).is_none());
    }

    #[test]
    fn padding_is_never_a_logical_slot() {
        let layout = VirtualContainerLayout::new(18).unwrap();
        assert!(layout.contains_logical_slot(17));
        assert!(!layout.contains_logical_slot(18));
        assert!(!layout.contains_logical_slot(26));
    }

    #[test]
    fn pending_container_falls_back_after_four_ticks() {
        let session = VirtualContainerSession {
            sync_id: 1,
            layout: VirtualContainerLayout::new(27).unwrap(),
            holder_positions: Vec::new(),
            original_states: Vec::new(),
            acknowledgement_timestamp: -1,
            prepared_tick: 10,
            open_attempts: 1,
            phase: VirtualContainerPhase::AwaitingAcknowledgement,
        };

        assert!(!session.should_fallback_open(13));
        assert!(session.should_fallback_open(14));
    }

    #[test]
    fn rejected_container_retries_after_three_ticks() {
        let mut session = VirtualContainerSession {
            sync_id: 1,
            layout: VirtualContainerLayout::new(27).unwrap(),
            holder_positions: Vec::new(),
            original_states: Vec::new(),
            acknowledgement_timestamp: -1,
            prepared_tick: 10,
            open_attempts: 1,
            phase: VirtualContainerPhase::Open,
        };

        assert!(session.defer_rejected_open(20));
        assert_eq!(session.open_attempts, 2);
        assert!(!session.should_begin_retry(22));
        assert!(session.should_begin_retry(23));

        session.begin_retry(-2, 23);
        assert_eq!(session.acknowledgement_timestamp, -2);
        assert_eq!(
            session.phase,
            VirtualContainerPhase::AwaitingAcknowledgement
        );
    }

    #[test]
    fn rejected_container_stops_after_seven_attempts() {
        let mut session = VirtualContainerSession {
            sync_id: 1,
            layout: VirtualContainerLayout::new(27).unwrap(),
            holder_positions: Vec::new(),
            original_states: Vec::new(),
            acknowledgement_timestamp: -1,
            prepared_tick: 10,
            open_attempts: VirtualContainerSession::MAX_OPEN_ATTEMPTS,
            phase: VirtualContainerPhase::Open,
        };

        assert!(!session.defer_rejected_open(20));
        assert_eq!(session.phase, VirtualContainerPhase::Open);
    }
}
