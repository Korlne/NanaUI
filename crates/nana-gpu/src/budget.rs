//! Bounds on how much submitted work one lane may have queued on the shared
//! device queue.
//!
//! The device has one queue, and the desktop compositor waits behind it: a
//! process that queues seconds of work starves every other window. Each
//! [`FrameLane`](crate::FrameLane) keeps its own count of submissions the GPU
//! has not finished, so producers are held back by their own backlog and
//! never by the host's window frames, and the host's frames are never held
//! back by a producer's count.
//!
//! The count is conservative: an entry leaves only once a completion callback
//! or a wait has proved it finished, so the real queue depth is never larger
//! than what is tracked here.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// Default number of unfinished producer submissions. Two keeps one in
/// execution and one ready behind it, enough to keep a producer busy without
/// letting it queue frames' worth of work ahead of the desktop compositor.
pub(crate) const DEFAULT_PRODUCER_SUBMISSIONS: usize = 2;

/// One lane's unfinished submissions. `I` is the queue's submission index
/// (generic so the bookkeeping is testable without a device).
pub(crate) struct SubmissionBudget<I> {
    state: Mutex<BudgetState<I>>,
}

struct BudgetState<I> {
    limit: usize,
    /// Submits that passed the budget and have not reached the queue yet.
    reserved: usize,
    /// In queue order, oldest first.
    in_flight: VecDeque<InFlight<I>>,
    peak: usize,
    waits: u64,
}

struct InFlight<I> {
    index: I,
    done: Arc<AtomicBool>,
}

/// What a submitter must do before it may queue more work.
pub(crate) enum Admission<I> {
    /// Room was reserved; the submit must [`SubmissionBudget::submitted`] or
    /// [`SubmissionBudget::cancel`] it.
    Reserved,
    /// Full: wait for this submission (oldest of the lane) to finish, then
    /// ask again. `done` marks it finished once the wait returns.
    Wait(I, Arc<AtomicBool>),
    /// Full, and the only room is held by submits about to reach the queue.
    Busy,
}

impl<I: Clone> SubmissionBudget<I> {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            state: Mutex::new(BudgetState {
                limit: limit.max(1),
                reserved: 0,
                in_flight: VecDeque::new(),
                peak: 0,
                waits: 0,
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, BudgetState<I>> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn set_limit(&self, limit: usize) {
        self.lock().limit = limit.max(1);
    }

    pub(crate) fn limit(&self) -> usize {
        self.lock().limit
    }

    /// Submissions tracked as unfinished, plus reserved ones.
    pub(crate) fn in_flight(&self) -> usize {
        let mut state = self.lock();
        state.prune();
        state.in_flight.len() + state.reserved
    }

    /// Highest [`Self::in_flight`] ever reached.
    pub(crate) fn peak(&self) -> usize {
        self.lock().peak
    }

    pub(crate) fn waits(&self) -> u64 {
        self.lock().waits
    }

    /// Whether there is room for one more submission, without reserving it.
    pub(crate) fn has_room(&self, limit: Option<usize>) -> Result<(), Admission<I>> {
        let mut state = self.lock();
        state.prune();
        state.check(limit)
    }

    /// Reserve room for one submission, or say what to wait for. `limit`
    /// overrides the configured one (the presentation lane follows the frame
    /// slot count).
    pub(crate) fn reserve(&self, limit: Option<usize>) -> Admission<I> {
        let mut state = self.lock();
        state.prune();
        match state.check(limit) {
            Ok(()) => {
                state.reserved += 1;
                let used = state.in_flight.len() + state.reserved;
                state.peak = state.peak.max(used);
                Admission::Reserved
            }
            Err(wait) => wait,
        }
    }

    /// Reserve room even when the lane is full: the device is lost (waiting
    /// cannot end) or a wait outlived its bound. Counted all the same.
    pub(crate) fn force_reserve(&self) {
        let mut state = self.lock();
        state.reserved += 1;
        let used = state.in_flight.len() + state.reserved;
        state.peak = state.peak.max(used);
    }

    pub(crate) fn record_wait(&self) {
        self.lock().waits += 1;
    }

    /// The reserved submit reached the queue as `index`. Call in queue order
    /// (under the submission-order lock); `done` is set when it finishes.
    pub(crate) fn submitted(&self, index: I, done: Arc<AtomicBool>) {
        let mut state = self.lock();
        state.reserved = state.reserved.saturating_sub(1);
        state.in_flight.push_back(InFlight { index, done });
    }

    /// The reserved submit never reached the queue.
    pub(crate) fn cancel(&self) {
        let mut state = self.lock();
        state.reserved = state.reserved.saturating_sub(1);
    }
}

impl<I: Clone> BudgetState<I> {
    /// Drop finished entries from the front. Completion is in queue order,
    /// so a finished entry behind an unfinished one waits its turn.
    fn prune(&mut self) {
        while self
            .in_flight
            .front()
            .is_some_and(|entry| entry.done.load(Ordering::Acquire))
        {
            self.in_flight.pop_front();
        }
    }

    fn check(&self, limit: Option<usize>) -> Result<(), Admission<I>> {
        let limit = limit.unwrap_or(self.limit).max(1);
        if self.in_flight.len() + self.reserved < limit {
            return Ok(());
        }
        Err(match self.in_flight.front() {
            Some(oldest) => Admission::Wait(oldest.index.clone(), Arc::clone(&oldest.done)),
            None => Admission::Busy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reserved<I>(admission: Admission<I>) -> bool {
        matches!(admission, Admission::Reserved)
    }

    #[test]
    fn a_full_lane_waits_for_its_oldest_submission() {
        let budget = SubmissionBudget::new(2);
        let first = Arc::new(AtomicBool::new(false));
        let second = Arc::new(AtomicBool::new(false));
        assert!(reserved(budget.reserve(None)));
        budget.submitted(1u32, Arc::clone(&first));
        assert!(reserved(budget.reserve(None)));
        budget.submitted(2, Arc::clone(&second));
        let Admission::Wait(oldest, done) = budget.reserve(None) else {
            panic!("a full lane must wait");
        };
        assert_eq!(oldest, 1);
        done.store(true, Ordering::Release);
        assert!(reserved(budget.reserve(None)));
        assert_eq!(budget.in_flight(), 2);
        assert_eq!(budget.peak(), 2);
    }

    #[test]
    fn completion_out_of_order_is_held_until_the_front_finishes() {
        let budget = SubmissionBudget::new(2);
        let first = Arc::new(AtomicBool::new(false));
        let second = Arc::new(AtomicBool::new(true));
        assert!(reserved(budget.reserve(None)));
        budget.submitted(1u32, Arc::clone(&first));
        assert!(reserved(budget.reserve(None)));
        budget.submitted(2, second);
        assert_eq!(budget.in_flight(), 2, "only the front may leave first");
        first.store(true, Ordering::Release);
        assert_eq!(budget.in_flight(), 0);
    }

    #[test]
    fn reservations_count_and_cancel_returns_them() {
        let budget = SubmissionBudget::<u32>::new(1);
        assert!(reserved(budget.reserve(None)));
        assert!(matches!(budget.reserve(None), Admission::Busy));
        budget.cancel();
        assert!(reserved(budget.reserve(None)));
        assert!(budget.has_room(Some(2)).is_ok());
        assert!(budget.has_room(None).is_err());
    }

    #[test]
    fn a_zero_limit_still_admits_one() {
        let budget = SubmissionBudget::<u32>::new(0);
        assert_eq!(budget.limit(), 1);
        assert!(reserved(budget.reserve(Some(0))));
    }
}
