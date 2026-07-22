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
    pub phase: VirtualContainerPhase,
}

impl VirtualContainerSession {
    #[must_use]
    pub fn matches_acknowledgement(&self, timestamp: i64) -> bool {
        self.phase == VirtualContainerPhase::AwaitingAcknowledgement
            && self.acknowledgement_timestamp == timestamp
    }
}

#[cfg(test)]
mod tests {
    use pumpkin_data::screen::WindowType;

    use super::VirtualContainerLayout;

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
}
