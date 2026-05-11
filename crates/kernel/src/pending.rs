//! Out-of-order event buffer per convergence.md §4.2 + §4.8.
//!
//! Two independent eviction policies: age-based TTL + capacity-based
//! (with per-author sub-cap to thwart Sybil flooding). Eviction is
//! convergence-preserving (§4.8): peers re-request via `HeadsSummary`.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use myrhiza_types::{AuthorPubkey, Event, EventHash};

/// Configuration for [`PendingBuffer`] capacity + TTL bounds.
#[derive(Clone)]
pub struct PendingCfg {
    /// Hard cap on total entries; oldest evicted on overflow.
    pub max_total: usize,
    /// Per-author sub-cap; thwarts a single Sybil flooding the buffer.
    pub max_per_author: usize,
    /// Maximum time an entry may remain before lazy TTL eviction.
    pub ttl: Duration,
}

impl Default for PendingCfg {
    fn default() -> Self {
        Self {
            max_total: 10_000,
            max_per_author: 10_000 / 50,
            ttl: Duration::from_hours(1),
        }
    }
}

struct PendingEntry {
    event: Event,
    missing_deps: BTreeSet<EventHash>,
    inserted_at: Instant,
}

/// Buffer of events waiting for their cross-author deps to arrive.
///
/// Indexed three ways: by wire hash (primary), by author (for the
/// per-author cap), and by `(inserted_at, hash)` (for oldest-first
/// eviction). The compound time-index key lets two entries inserted
/// in the same `Instant` tick coexist without overwriting each other.
pub struct PendingBuffer {
    by_hash: BTreeMap<EventHash, PendingEntry>,
    by_author_count: BTreeMap<AuthorPubkey, usize>,
    by_insert_time: BTreeSet<(Instant, EventHash)>,
    cfg: PendingCfg,
}

impl PendingBuffer {
    /// Construct an empty buffer with the given config.
    #[must_use]
    pub fn new(cfg: PendingCfg) -> Self {
        Self {
            by_hash: BTreeMap::new(),
            by_author_count: BTreeMap::new(),
            by_insert_time: BTreeSet::new(),
            cfg,
        }
    }

    /// Number of entries currently buffered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_hash.len()
    }

    /// `true` if no entries are buffered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_hash.is_empty()
    }

    /// Insert an event with its missing deps. Evicts oldest entries
    /// if at-capacity. Lazy-evicts TTL-expired entries.
    pub fn insert(&mut self, event: Event, missing_deps: BTreeSet<EventHash>) {
        let now = Instant::now();
        self.evict_expired(now);
        // Per-author cap.
        let author_count = self
            .by_author_count
            .get(&event.author)
            .copied()
            .unwrap_or(0);
        if author_count >= self.cfg.max_per_author {
            self.evict_oldest_for_author(event.author);
        }
        // Total cap.
        if self.by_hash.len() >= self.cfg.max_total {
            self.evict_oldest();
        }

        let hash = event.wire_hash();
        let author = event.author;
        let entry = PendingEntry {
            event,
            missing_deps,
            inserted_at: now,
        };
        if self.by_hash.insert(hash, entry).is_none() {
            *self.by_author_count.entry(author).or_insert(0) += 1;
            self.by_insert_time.insert((now, hash));
        }
    }

    /// Drain entries whose deps are all now in `known`. Single pass;
    /// caller may loop if newly-satisfied entries themselves unblock
    /// further entries (the caller adds the just-applied hashes to
    /// `known` and calls again).
    pub fn newly_satisfied(&mut self, known: &BTreeSet<EventHash>) -> Vec<Event> {
        let ready: Vec<EventHash> = self
            .by_hash
            .iter()
            .filter(|(_, e)| e.missing_deps.is_subset(known))
            .map(|(h, _)| *h)
            .collect();
        let mut out = Vec::with_capacity(ready.len());
        for h in ready {
            if let Some(entry) = self.by_hash.remove(&h) {
                self.by_insert_time.remove(&(entry.inserted_at, h));
                if let Some(c) = self.by_author_count.get_mut(&entry.event.author) {
                    *c = c.saturating_sub(1);
                    if *c == 0 {
                        self.by_author_count.remove(&entry.event.author);
                    }
                }
                out.push(entry.event);
            }
        }
        out
    }

    fn evict_expired(&mut self, now: Instant) {
        // `checked_sub` keeps us safe in tests where `now` may be
        // earlier than `cfg.ttl`-from-epoch on cold-start. If the TTL
        // window extends before the monotonic clock's origin, nothing
        // is yet expired and we early-return.
        let Some(cutoff) = now.checked_sub(self.cfg.ttl) else {
            return;
        };
        let expired: Vec<(Instant, EventHash)> = self
            .by_insert_time
            .iter()
            .take_while(|(t, _)| *t < cutoff)
            .copied()
            .collect();
        for key in expired {
            self.remove_by_key(key);
        }
    }

    fn evict_oldest(&mut self) {
        if let Some(&key) = self.by_insert_time.iter().next() {
            self.remove_by_key(key);
        }
    }

    fn evict_oldest_for_author(&mut self, author: AuthorPubkey) {
        let target = self
            .by_insert_time
            .iter()
            .find(|(_, h)| {
                self.by_hash
                    .get(h)
                    .is_some_and(|e| e.event.author == author)
            })
            .copied();
        if let Some(key) = target {
            self.remove_by_key(key);
        }
    }

    fn remove_by_key(&mut self, key: (Instant, EventHash)) {
        let hash = key.1;
        if let Some(entry) = self.by_hash.remove(&hash)
            && let Some(c) = self.by_author_count.get_mut(&entry.event.author)
        {
            *c = c.saturating_sub(1);
            if *c == 0 {
                self.by_author_count.remove(&entry.event.author);
            }
        }
        self.by_insert_time.remove(&key);
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::cast_possible_truncation
)]
mod tests {
    use super::*;
    use myrhiza_types::Hlc;

    fn evt(author: u8, seq: u64) -> Event {
        Event {
            author: AuthorPubkey::from_bytes([author; 32]),
            seq,
            prev: EventHash::ZERO,
            deps: BTreeSet::new(),
            hlc: Hlc {
                wall_ms: 0,
                logical: 0,
            },
            payload: vec![seq as u8],
            signature: [0; 64],
        }
    }

    #[test]
    fn newly_satisfied_returns_events_whose_deps_become_known() {
        let mut pb = PendingBuffer::new(PendingCfg::default());
        let parent_hash = EventHash::blake3(b"parent");
        let mut deps = BTreeSet::new();
        deps.insert(parent_hash);
        pb.insert(evt(1, 1), deps);
        assert_eq!(pb.len(), 1);

        let known = BTreeSet::new();
        assert!(pb.newly_satisfied(&known).is_empty());

        let mut known = BTreeSet::new();
        known.insert(parent_hash);
        let ready = pb.newly_satisfied(&known);
        assert_eq!(ready.len(), 1);
        assert_eq!(pb.len(), 0);
    }

    #[test]
    fn capacity_eviction_drops_oldest() {
        let cfg = PendingCfg {
            max_total: 2,
            max_per_author: 10,
            ttl: Duration::from_hours(1),
        };
        let mut pb = PendingBuffer::new(cfg);
        pb.insert(evt(1, 1), BTreeSet::new());
        std::thread::sleep(Duration::from_millis(1));
        pb.insert(evt(2, 1), BTreeSet::new());
        std::thread::sleep(Duration::from_millis(1));
        pb.insert(evt(3, 1), BTreeSet::new());
        assert_eq!(pb.len(), 2);
    }
}
