//! One sorting routine for the framework's call sites.
//!
//! `slice::sort_by` and `sort_by_key` are generic over the element type *and*
//! the closure, so every call site compiles its own copy of the standard
//! library's sort: about 6 KiB each, and the framework has more than a
//! hundred. That was 0.7 MiB of a minimal window for sorts that are almost
//! all over a handful of elements.
//!
//! [`CompactSort`] sorts through one shared copy instead. A slice of up to
//! [`INSERTION_MAX`] elements is insertion sorted in place. A longer one has
//! its *indices* sorted by the standard stable sort, compiled once for
//! `usize` behind a `dyn` comparator, and is then rearranged to that order.
//! What is left per element type is a short loop; per call site, the closure.
//!
//! Both paths are stable, so the methods also replace `sort_unstable_by` and
//! `sort_unstable_by_key`. A long slice costs an index allocation and an
//! indirect call per comparison, so a sort that is hot *and* long keeps the
//! standard method. Two places do: the layout engine
//! (`nana-ui-runtime/src/layout_engine`) and the hit-test index
//! (`world/hit_test/index.rs`) sort a container's whole child list on the
//! frame path, and routing them here cost 0.7% more instructions on the
//! dirty-frame benchmark (2000 rows). A slice of plain
//! integers sorted without a closure (`sort_unstable()`) already shares its
//! copy per element type and has nothing to gain here.

use std::cmp::Ordering;

/// The longest slice sorted by insertion. The standard sort switches to its
/// small-slice path at the same length.
pub const INSERTION_MAX: usize = 20;

/// Stable sorts that share one compiled routine. See the [module](self).
pub trait CompactSort<T> {
    /// What `sort_by` does, through the shared routine.
    fn compact_sort_by(&mut self, compare: impl FnMut(&T, &T) -> Ordering);

    /// What `sort_by_key` does, through the shared routine. `key` is called
    /// for both sides of every comparison, as the standard method does.
    fn compact_sort_by_key<K: Ord>(&mut self, key: impl FnMut(&T) -> K);
}

impl<T> CompactSort<T> for [T] {
    // Most slices sorted in a frame are empty or hold one element (a node's
    // children, a line's spans): those return here, without the call.
    #[inline]
    fn compact_sort_by(&mut self, mut compare: impl FnMut(&T, &T) -> Ordering) {
        if self.len() > 1 {
            sort_slice(self, &mut compare);
        }
    }

    #[inline]
    fn compact_sort_by_key<K: Ord>(&mut self, mut key: impl FnMut(&T) -> K) {
        if self.len() > 1 {
            sort_slice(self, &mut |left, right| key(left).cmp(&key(right)));
        }
    }
}

/// Per element type, not per call site: the comparator is `dyn`.
#[inline(never)]
fn sort_slice<T>(items: &mut [T], compare: &mut dyn FnMut(&T, &T) -> Ordering) {
    let len = items.len();
    if len <= INSERTION_MAX {
        for sorted in 1..len {
            let mut at = sorted;
            while at > 0 && compare(&items[at - 1], &items[at]) == Ordering::Greater {
                items.swap(at - 1, at);
                at -= 1;
            }
        }
        return;
    }
    let mut order: Vec<usize> = (0..len).collect();
    sort_order(&mut order, &mut |left, right| {
        compare(&items[left], &items[right])
    });
    // `order[place]` is the element that belongs at `place`. Walk each cycle
    // of that permutation, swapping its elements home; a visited place is
    // marked by pointing at itself.
    for start in 0..len {
        let mut place = start;
        while order[place] != start {
            let next = order[place];
            items.swap(place, next);
            order[place] = place;
            place = next;
        }
        order[place] = place;
    }
}

/// The one copy of the standard stable sort the framework's call sites share.
#[inline(never)]
fn sort_order(order: &mut [usize], compare: &mut dyn FnMut(usize, usize) -> Ordering) {
    order.sort_by(|left, right| compare(*left, *right));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small deterministic generator: the cases must not change between runs.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self, below: u64) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) % below
        }
    }

    /// Keys repeat (`below` is small against the length), and each element
    /// carries where it started, so an unstable result is a different vector.
    fn cases() -> Vec<Vec<(u64, usize)>> {
        let mut random = Lcg(0x5eed);
        let mut cases = Vec::new();
        for len in (0..=64).chain([100, 255, 256, 257, 1000, 4097]) {
            for below in [1, 2, 7, 1 << 40] {
                cases.push((0..len).map(|index| (random.next(below), index)).collect());
            }
        }
        // Already sorted, reversed, and one element out of place.
        for len in [INSERTION_MAX, INSERTION_MAX + 1, 300] {
            cases.push((0..len).map(|index| (index as u64, index)).collect());
            cases.push(
                (0..len)
                    .map(|index| ((len - index) as u64, index))
                    .collect(),
            );
            let mut nearly: Vec<(u64, usize)> =
                (0..len).map(|index| (index as u64, index)).collect();
            nearly[len / 2].0 = 0;
            cases.push(nearly);
        }
        cases
    }

    #[test]
    fn it_orders_as_the_standard_stable_sort_does_at_every_length() {
        for case in cases() {
            let mut expected = case.clone();
            expected.sort_by_key(|item| item.0);
            let mut by = case.clone();
            by.compact_sort_by(|left, right| left.0.cmp(&right.0));
            assert_eq!(by, expected, "compact_sort_by, {} elements", case.len());
            let mut by_key = case.clone();
            by_key.compact_sort_by_key(|item| item.0);
            assert_eq!(
                by_key,
                expected,
                "compact_sort_by_key, {} elements",
                case.len()
            );
        }
    }

    #[test]
    fn it_orders_descending_and_keeps_equal_elements_in_their_order() {
        for case in cases() {
            let mut expected = case.clone();
            expected.sort_by_key(|item| std::cmp::Reverse(item.0));
            let mut sorted = case.clone();
            sorted.compact_sort_by(|left, right| right.0.cmp(&left.0));
            assert_eq!(sorted, expected, "{} elements", case.len());
        }
    }

    /// Elements that are not `Copy` are moved, never duplicated or dropped.
    #[test]
    fn it_moves_owned_elements() {
        let mut random = Lcg(7);
        let mut words: Vec<String> = (0..500).map(|_| random.next(60).to_string()).collect();
        let mut expected = words.clone();
        expected.sort_by(|left, right| left.len().cmp(&right.len()).then(left.cmp(right)));
        words.compact_sort_by(|left, right| left.len().cmp(&right.len()).then(left.cmp(right)));
        assert_eq!(words, expected);
    }

    /// The point of the module: one comparator type reaches the standard
    /// sort, whatever is being sorted.
    #[test]
    fn a_long_slice_compares_through_its_indices() {
        let mut calls = 0usize;
        let mut items: Vec<u32> = (0..200).rev().collect();
        items.compact_sort_by(|left, right| {
            calls += 1;
            left.cmp(right)
        });
        assert!(items.is_sorted());
        // A reversed run is one pass for the standard sort: far below n log n.
        assert!(calls < 200 * 8, "{calls} comparisons");
    }
}
