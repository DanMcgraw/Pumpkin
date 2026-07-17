//! Shared identities and lifecycle terminology for plugin-visible transactions.
//!
//! Pumpkin allocates these identities. Plugins may compare and hash them while the
//! server is running, but must not persist them or use them to manufacture actions.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TRANSACTION_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_GUI_SESSION_ID: AtomicU64 = AtomicU64::new(1);

/// Opaque identity for one validated gameplay transaction or preview cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginTransactionId(u64);

impl PluginTransactionId {
    /// Allocates a process-unique transaction identity.
    #[must_use]
    pub(crate) fn allocate() -> Self {
        Self(allocate_id(&NEXT_TRANSACTION_ID, "plugin transaction"))
    }

    #[must_use]
    pub(crate) const fn into_internal(self) -> u64 {
        self.0
    }

    #[must_use]
    pub(crate) const fn from_internal(value: u64) -> Self {
        Self(value)
    }
}

/// Opaque identity for one open instance of a plugin-owned GUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginGuiSessionId(u64);

impl PluginGuiSessionId {
    /// Allocates a process-unique GUI session identity.
    #[must_use]
    pub(crate) fn allocate() -> Self {
        Self(allocate_id(&NEXT_GUI_SESSION_ID, "plugin GUI session"))
    }
}

fn allocate_id(counter: &AtomicU64, kind: &str) -> u64 {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .unwrap_or_else(|_| panic!("exhausted {kind} identities"))
}

/// Correlation data shared by every public stage of one transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionContext {
    /// Pumpkin-issued identity preserved from prepare through completion.
    pub id: PluginTransactionId,
    /// Server tick on which the validated action or preview cycle began.
    pub initiated_tick: i32,
}

impl TransactionContext {
    /// Starts a new plugin-visible transaction at `initiated_tick`.
    #[must_use]
    pub(crate) fn new(initiated_tick: i32) -> Self {
        Self {
            id: PluginTransactionId::allocate(),
            initiated_tick,
        }
    }

    #[must_use]
    pub(crate) const fn from_internal(id: u64, initiated_tick: i32) -> Option<Self> {
        if id == 0 {
            None
        } else {
            Some(Self {
                id: PluginTransactionId::from_internal(id),
                initiated_tick,
            })
        }
    }
}

/// Standard timing terminology for plugin-visible gameplay events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransactionPhase {
    /// Input was received, before vanilla validation completed.
    Attempt,
    /// Pumpkin computed a candidate result that blocking handlers may adjust.
    Prepare,
    /// Pumpkin revalidated and is about to apply the transaction atomically.
    Commit,
    /// All state changes succeeded; this phase is immutable and observational.
    Complete,
}

/// Internal guard used by multi-stage gameplay paths to enforce legal ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TransactionLifecycle {
    context: TransactionContext,
    state: LifecycleState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifecycleState {
    Started,
    Prepared,
    Committed,
    Cancelled,
    Completed,
}

impl TransactionLifecycle {
    #[must_use]
    pub(crate) fn new(initiated_tick: i32) -> Self {
        Self {
            context: TransactionContext::new(initiated_tick),
            state: LifecycleState::Started,
        }
    }

    #[must_use]
    pub(crate) const fn context(&self) -> TransactionContext {
        self.context
    }

    pub(crate) fn finish_prepare(&mut self, cancelled: bool) -> bool {
        if self.state != LifecycleState::Started {
            return false;
        }
        self.state = if cancelled {
            LifecycleState::Cancelled
        } else {
            LifecycleState::Prepared
        };
        !cancelled
    }

    pub(crate) fn finish_commit(&mut self, cancelled: bool) -> bool {
        if self.state != LifecycleState::Prepared {
            return false;
        }
        self.state = if cancelled {
            LifecycleState::Cancelled
        } else {
            LifecycleState::Committed
        };
        !cancelled
    }

    pub(crate) fn complete(&mut self) -> bool {
        if self.state != LifecycleState::Committed {
            return false;
        }
        self.state = LifecycleState::Completed;
        true
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{PluginGuiSessionId, TransactionLifecycle};

    #[test]
    fn allocated_identities_are_unique() {
        let transactions: HashSet<_> = (0..10_000)
            .map(|_| TransactionLifecycle::new(0).context().id)
            .collect();
        let sessions: HashSet<_> = (0..10_000)
            .map(|_| PluginGuiSessionId::allocate())
            .collect();

        assert_eq!(transactions.len(), 10_000);
        assert_eq!(sessions.len(), 10_000);
    }

    #[test]
    fn successful_sequence_preserves_identity_and_completes_once() {
        let mut lifecycle = TransactionLifecycle::new(42);
        let context = lifecycle.context();

        assert!(lifecycle.finish_prepare(false));
        assert_eq!(lifecycle.context(), context);
        assert!(lifecycle.finish_commit(false));
        assert_eq!(lifecycle.context(), context);
        assert!(lifecycle.complete());
        assert!(!lifecycle.complete());
    }

    #[test]
    fn cancellation_prevents_commit_and_completion() {
        let mut prepare_cancelled = TransactionLifecycle::new(0);
        assert!(!prepare_cancelled.finish_prepare(true));
        assert!(!prepare_cancelled.finish_commit(false));
        assert!(!prepare_cancelled.complete());

        let mut commit_cancelled = TransactionLifecycle::new(0);
        assert!(commit_cancelled.finish_prepare(false));
        assert!(!commit_cancelled.finish_commit(true));
        assert!(!commit_cancelled.complete());
    }

    #[test]
    fn internal_round_trip_preserves_context_and_rejects_sentinel() {
        let original = super::TransactionContext::new(91);
        let restored = super::TransactionContext::from_internal(
            original.id.into_internal(),
            original.initiated_tick,
        );

        assert_eq!(restored, Some(original));
        assert_eq!(super::TransactionContext::from_internal(0, 91), None);
    }
}
