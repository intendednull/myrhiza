//! Event DAG per convergence.md §4.
//!
//! Per-author chain integrity, cross-author deps, topo-sort. All
//! in-memory for B-1; persistence is B-7.
//!
//! Per plan-B-1 spec §4.2, [`EventDag::insert`] validates events in
//! strict order: signature -> duplicate -> genesis-specific checks
//! (when `seq == 1`) -> per-author chain integrity + equivocation ->
//! deps presence -> commit. Each rejection maps to a [`DagError`]
//! variant; missing deps return [`Inserted::Pending`] (NOT an error).

use std::collections::{BTreeMap, BTreeSet};

use myrhiza_types::{
    AuthorHead, AuthorPubkey, AuthorSeq, BundleHash, Event, EventHash, GenesisV1, Topic,
    decode_canonical,
};
use thiserror::Error;

/// Errors returned by [`EventDag::insert`].
///
/// Returning a `DagError` means the event was rejected and NOT
/// inserted. Missing-deps is NOT an error: see [`Inserted::Pending`].
#[derive(Debug, Error)]
pub enum DagError {
    /// Signature verification against `body_hash` failed.
    #[error("invalid Ed25519 signature")]
    InvalidSignature,
    /// Genesis-specific invariant violated: see message for which.
    #[error("invalid Genesis event: {0}")]
    InvalidGenesis(&'static str),
    /// Genesis payload decodes, but the derived topic does not match
    /// the DAG's bound topic.
    #[error("invalid topic: expected {expected:?}, derived {derived:?}")]
    InvalidTopic {
        /// Topic the DAG is bound to.
        expected: Topic,
        /// Topic derived from `(bundle_hash, genesis.seed, topic_name)`.
        derived: Topic,
    },
    /// Per-author chain integrity violated: `seq` or `prev` is off
    /// from the author's current head.
    #[error(
        "invalid chain for author {author:?}: expected seq {expected_seq}, got {got_seq}; expected prev {expected_prev:?}, got prev {got_prev:?}"
    )]
    InvalidChain {
        /// Author whose chain was violated.
        author: AuthorPubkey,
        /// Expected next seq for this author (`head_seq + 1`).
        expected_seq: u64,
        /// Seq the event claimed.
        got_seq: u64,
        /// Expected prev (current `head_hash` for the author).
        expected_prev: EventHash,
        /// Prev the event claimed.
        got_prev: EventHash,
    },
    /// Direct-receive equivocation: this author has already published
    /// a different event at the same `seq`.
    #[error(
        "equivocation by author {author:?} at seq {seq}: local hash {local_hash:?}, remote hash {remote_hash:?}"
    )]
    Equivocation {
        /// Author who equivocated.
        author: AuthorPubkey,
        /// Seq at which equivocation was observed.
        seq: u64,
        /// Wire-hash already on file at this `(author, seq)`.
        local_hash: EventHash,
        /// Wire-hash of the conflicting newcomer.
        remote_hash: EventHash,
    },
}

/// Per-author chain state.
///
/// Tracks the current head (seq + wire-hash) for one author plus the
/// full `seq -> wire_hash` history for direct-receive equivocation
/// detection. The empty state is signaled by `head_seq == 0` and
/// `head_hash == EventHash::ZERO` — see [`AuthorChain::empty`] /
/// the manual `Default` impl below.
#[derive(Clone)]
pub struct AuthorChain {
    /// Highest seq this author has published (0 == empty / unknown).
    pub head_seq: u64,
    /// Wire-hash at `head_seq`. [`EventHash::ZERO`] when `head_seq == 0`.
    pub head_hash: EventHash,
    /// Full history: `seq -> wire_hash`. Used for direct-receive
    /// equivocation detection.
    pub seq_to_hash: BTreeMap<u64, EventHash>,
}

impl AuthorChain {
    /// Empty chain (no events yet from this author). `EventHash` has
    /// no `Default`, so we hand-roll one here to keep the sentinel
    /// (`head_hash == EventHash::ZERO`) explicit.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            head_seq: 0,
            head_hash: EventHash::ZERO,
            seq_to_hash: BTreeMap::new(),
        }
    }
}

impl Default for AuthorChain {
    fn default() -> Self {
        Self::empty()
    }
}

/// Outcome of an [`EventDag::insert`] call (success path).
///
/// Failure path is [`DagError`]; missing-deps is NOT a failure — it
/// returns [`Inserted::Pending`].
#[derive(Debug, Clone)]
pub enum Inserted {
    /// Event was accepted and committed at `topo_index`.
    NewlyApplied {
        /// Monotonic insertion order within this DAG (0-based).
        topo_index: u64,
        /// Wire-hash of the inserted event.
        hash: EventHash,
    },
    /// Wire-hash was already in the DAG; no-op.
    AlreadyKnown,
    /// Event is well-formed but some declared deps are not yet known.
    /// Caller should buffer and retry when those deps land.
    Pending(BTreeSet<EventHash>),
}

/// In-memory event DAG bound to one topic.
///
/// Per plan-B-1 spec §4: every [`Event`] carries an `(author, seq,
/// prev, deps)` quadruple. The DAG enforces per-author chain
/// integrity via [`AuthorChain`] and cross-author causality via
/// `deps`. The topic binding allows genesis-event verification:
/// `Topic::derive(bundle_hash, genesis.seed, topic_name)` MUST
/// match the bound topic.
pub struct EventDag {
    by_hash: BTreeMap<EventHash, Event>,
    by_author: BTreeMap<AuthorPubkey, AuthorChain>,
    parents_to_children: BTreeMap<EventHash, BTreeSet<EventHash>>,
    indegree: BTreeMap<EventHash, usize>,
    topic: Topic,
    app_bundle_hash: BundleHash,
    /// Pre-NFC-normalized topic name; used for Genesis topic
    /// verification. Callers MUST normalize before constructing.
    topic_name: String,
    next_topo_index: u64,
    /// Set on the first successful Genesis insert (per spec §4.2 step 3
    /// applicability rule). Used to distinguish "topic Genesis"
    /// (founder's seq=1) from "author-chain start" (non-founder's
    /// seq=1) — the latter skips Genesis validation and falls through
    /// to step 4 (chain integrity). `None` means no Genesis observed yet.
    genesis_author: Option<AuthorPubkey>,
}

impl EventDag {
    /// Construct an empty DAG for the given topic.
    ///
    /// `topic_name` MUST already be NFC-normalized (use
    /// `myrhiza_manifest::derive_topic_normalized` upstream when the
    /// name comes from untrusted input).
    #[must_use]
    pub fn new(topic: Topic, app_bundle_hash: BundleHash, topic_name: String) -> Self {
        Self {
            by_hash: BTreeMap::new(),
            by_author: BTreeMap::new(),
            parents_to_children: BTreeMap::new(),
            indegree: BTreeMap::new(),
            topic,
            app_bundle_hash,
            topic_name,
            next_topo_index: 0,
            genesis_author: None,
        }
    }

    /// Look up an event by its wire-hash.
    #[must_use]
    pub fn get(&self, hash: &EventHash) -> Option<&Event> {
        self.by_hash.get(hash)
    }

    /// Snapshot of all wire-hashes currently in the DAG.
    #[must_use]
    pub fn known_hashes(&self) -> BTreeSet<EventHash> {
        self.by_hash.keys().copied().collect()
    }

    /// Borrow the chain state for an author, if known.
    #[must_use]
    pub fn author_chain(&self, author: &AuthorPubkey) -> Option<&AuthorChain> {
        self.by_author.get(author)
    }

    /// Insert an event into the DAG.
    ///
    /// See module docs for validation order. The function is
    /// transactional in the sense that on any `DagError` return, no
    /// internal state is mutated; on `Inserted::Pending` return, no
    /// internal state is mutated; only on `Inserted::NewlyApplied`
    /// does the DAG actually grow.
    ///
    /// # Errors
    /// Returns [`DagError`] variants per plan-B-1 spec §4.2.
    pub fn insert(&mut self, event: Event) -> Result<Inserted, DagError> {
        let wire_hash = event.wire_hash();

        // Step 1: verify signature against body_hash.
        let body_hash = event.hash_signed_body();
        myrhiza_manifest::verify_signature(
            event.author.as_bytes(),
            body_hash.as_bytes(),
            &event.signature,
        )
        .map_err(|_| DagError::InvalidSignature)?;

        // Step 2: duplicate check.
        if self.by_hash.contains_key(&wire_hash) {
            return Ok(Inserted::AlreadyKnown);
        }

        // Step 3: Genesis-specific validation, gated by the
        // applicability rule (spec §4.2 step 3): only run when the
        // event could plausibly BE the topic Genesis — i.e., we have
        // not yet recorded a Genesis (`genesis_author.is_none()`), or
        // this event's author already IS the recorded Genesis author
        // (re-presented Genesis: step 2 catches the duplicate, step 4
        // catches equivocation, but the validation itself still has to
        // run for completeness). Non-founder seq=1 events fall through
        // to step 4 — their per-author chain head is validated as a
        // chain head, not as the topic Genesis.
        let runs_genesis_validation =
            event.seq == 1 && self.genesis_author.is_none_or(|a| a == event.author);
        if runs_genesis_validation {
            if event.prev != EventHash::ZERO {
                return Err(DagError::InvalidGenesis("prev != ZERO"));
            }
            if !event.deps.is_empty() {
                return Err(DagError::InvalidGenesis("deps != empty"));
            }
            let genesis: GenesisV1 = decode_canonical(&event.payload)
                .map_err(|_| DagError::InvalidGenesis("payload not a canonical GenesisV1"))?;
            let derived = Topic::derive(&self.app_bundle_hash, &genesis.seed, &self.topic_name);
            if derived != self.topic {
                return Err(DagError::InvalidTopic {
                    expected: self.topic,
                    derived,
                });
            }
            if event.author != genesis.founder_pubkey {
                return Err(DagError::InvalidGenesis("author != founder_pubkey"));
            }
        }

        // Look up the Genesis event-hash (if any) BEFORE taking the
        // per-author chain mutable borrow below — we need it in step 6
        // to add the implicit Genesis dependency for non-founder chain
        // heads, and the borrow checker can't see that `genesis_author`
        // is distinct from `event.author` in the non-founder case.
        let genesis_hash: Option<EventHash> = self
            .genesis_author
            .filter(|a| *a != event.author)
            .and_then(|a| self.by_author.get(&a))
            .and_then(|c| c.seq_to_hash.get(&1).copied());

        // Step 4: chain integrity + equivocation check (genesis + non-genesis both).
        let chain = self.by_author.entry(event.author).or_default();

        // Direct-receive equivocation check.
        if let Some(&existing_hash) = chain.seq_to_hash.get(&event.seq)
            && existing_hash != wire_hash
        {
            return Err(DagError::Equivocation {
                author: event.author,
                seq: event.seq,
                local_hash: existing_hash,
                remote_hash: wire_hash,
            });
        }

        let expected_seq = chain.head_seq + 1;
        let expected_prev = chain.head_hash;
        if event.seq != expected_seq || event.prev != expected_prev {
            return Err(DagError::InvalidChain {
                author: event.author,
                expected_seq,
                got_seq: event.seq,
                expected_prev,
                got_prev: event.prev,
            });
        }

        // Step 5: deps presence.
        let mut missing = BTreeSet::new();
        for d in &event.deps {
            if !self.by_hash.contains_key(d) {
                missing.insert(*d);
            }
        }
        if !missing.is_empty() {
            return Ok(Inserted::Pending(missing));
        }

        // Step 6: commit.
        // Parents = deps ∪ {prev (if not ZERO)} ∪ {Genesis (if non-founder chain head)};
        // indegree records the total parent count so the topo-sort
        // decrements as parents are resolved.
        //
        // Non-founder chain heads (seq=1 where genesis_author is Some
        // and != event.author) carry an implicit causal dependency on
        // the topic Genesis: their state-apply requires Genesis to have
        // run first so prior_state is populated. Without this edge,
        // topo-sort (BTreeSet lex tie-break on EventHash) can place a
        // non-founder seq=1 before Genesis whenever the non-founder's
        // hash sorts before Genesis's hash — causing the state-apply
        // discriminator (`seq == 1 && prior_state.is_empty()`) to mis-
        // identify the non-founder event as Genesis and reject. Making
        // the dependency explicit in the DAG turns a hash-ordering
        // accident into a structural guarantee: Genesis is the unique
        // event with indegree 0 once it has been inserted, so every
        // peer's replay applies it first regardless of insertion order.
        // See plan-B-1 §4.3 + master convergence.md §"Genesis event
        // semantics" ("the first event in any topic MUST be a Genesis
        // event").
        let mut parents: BTreeSet<EventHash> = event.deps.clone();
        if event.prev != EventHash::ZERO {
            parents.insert(event.prev);
        }
        if event.seq == 1
            && !runs_genesis_validation
            && let Some(g_hash) = genesis_hash
        {
            parents.insert(g_hash);
        }
        self.indegree.insert(wire_hash, parents.len());
        for parent in &parents {
            self.parents_to_children
                .entry(*parent)
                .or_default()
                .insert(wire_hash);
        }
        chain.head_seq = event.seq;
        chain.head_hash = wire_hash;
        chain.seq_to_hash.insert(event.seq, wire_hash);
        let topo_index = self.next_topo_index;
        self.next_topo_index += 1;
        let event_author = event.author;
        self.by_hash.insert(wire_hash, event);

        // Per §4.2 step 3 applicability rule: a successful insert of an
        // event that ran (and passed) Genesis validation IS the topic
        // Genesis. Record the author so subsequent non-founder seq=1
        // events skip step 3. Write-once: a second founder seq=1 fails
        // step 4's equivocation check before reaching here.
        if runs_genesis_validation && self.genesis_author.is_none() {
            self.genesis_author = Some(event_author);
        }

        Ok(Inserted::NewlyApplied {
            topo_index,
            hash: wire_hash,
        })
    }

    /// Topo-sort the full DAG.
    ///
    /// Kahn's algorithm with a [`BTreeSet`] ready-set: ties between
    /// events whose dependencies have all resolved are broken by
    /// `EventHash` lex byte-order (per plan-B-1 spec §4.1, §4.3).
    /// The result is canonical across peers given the same DAG
    /// contents, which is what makes cross-peer convergence checkable.
    ///
    /// # Panics
    ///
    /// Panics if the DAG is corrupted (cycle / orphan in indegree map).
    /// The structural invariant — per-author `prev` strictly points
    /// earlier in seq, cross-author `deps` are content-hashes which by
    /// collision resistance cannot back-reference — guarantees the
    /// DAG is acyclic by construction. A `len` mismatch here therefore
    /// signals memory corruption, not a recoverable input error.
    /// Aborting via panic is the correct response: returning a partial
    /// sort would silently violate convergence.
    #[must_use]
    pub fn topo_sort(&self) -> Vec<EventHash> {
        let mut indegree = self.indegree.clone();
        let mut ready: BTreeSet<EventHash> = indegree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(h, _)| *h)
            .collect();
        let mut out = Vec::with_capacity(self.by_hash.len());
        while let Some(next) = ready.pop_first() {
            out.push(next);
            if let Some(children) = self.parents_to_children.get(&next) {
                for child in children {
                    if let Some(deg) = indegree.get_mut(child) {
                        *deg -= 1;
                        if *deg == 0 {
                            ready.insert(*child);
                        }
                    }
                }
            }
        }
        // CLAUDE.md "no panics in non-test code" — but a corrupted DAG
        // is unrecoverable; halting via panic is the correct response.
        // `manual_assert` is allowed because the explicit if/panic form
        // is what the spec body prescribes and reads more clearly than
        // `assert!(a == b, msg, ...)` for the structural-invariant case.
        #[allow(clippy::panic, clippy::manual_assert)]
        if out.len() != self.by_hash.len() {
            panic!(
                "DAG topo-sort produced {} events from {} total — DAG corrupted (cycle?)",
                out.len(),
                self.by_hash.len()
            );
        }
        out
    }

    /// Topo-sort a SUBSET of events selected by `filter`.
    ///
    /// Used by anchor-bounded replay (plan-B-1 spec §8.4 step 3):
    /// callers pick a slice of the DAG (e.g. events at or below a
    /// `DriftAnchor`) and ask for that slice in canonical order.
    ///
    /// Implementation: a local `sub_indegree` map is built counting
    /// only those parents that are themselves in the subset. The
    /// children-decrement step uses `get_mut` (not `expect`) so that
    /// children outside the subset are silently skipped — they are
    /// not errors, they are just not part of this sort.
    ///
    /// # Panics
    ///
    /// Panics if the produced sort is shorter than the input subset.
    /// By DAG construction the subset is acyclic, so a mismatch
    /// indicates internal indegree-bookkeeping drift — an unrecoverable
    /// state. Mirrors [`Self::topo_sort`]'s structural-invariant guard.
    pub fn topo_sort_subset<F: Fn(&Event) -> bool>(&self, filter: F) -> Vec<EventHash> {
        // Build local sub_indegree from in-subset parents only.
        let in_subset: BTreeSet<EventHash> = self
            .by_hash
            .iter()
            .filter(|(_, e)| filter(e))
            .map(|(h, _)| *h)
            .collect();

        // Implicit Genesis dependency: non-founder chain heads (seq=1
        // from author != genesis_author) carry an implicit edge from
        // the topic Genesis (see `insert` step 6 for rationale). Mirror
        // that edge here so sub_indegree matches `parents_to_children`.
        let genesis_in_subset: Option<EventHash> = self
            .genesis_author
            .and_then(|a| self.by_author.get(&a))
            .and_then(|c| c.seq_to_hash.get(&1).copied())
            .filter(|h| in_subset.contains(h));

        let mut sub_indegree: BTreeMap<EventHash, usize> = BTreeMap::new();
        for hash in &in_subset {
            let event = &self.by_hash[hash];
            let mut count = 0usize;
            if event.prev != EventHash::ZERO && in_subset.contains(&event.prev) {
                count += 1;
            }
            for d in &event.deps {
                if in_subset.contains(d) {
                    count += 1;
                }
            }
            if event.seq == 1
                && let Some(g_author) = self.genesis_author
                && event.author != g_author
                && genesis_in_subset.is_some()
            {
                count += 1;
            }
            sub_indegree.insert(*hash, count);
        }

        let mut ready: BTreeSet<EventHash> = sub_indegree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(h, _)| *h)
            .collect();
        let mut out = Vec::with_capacity(in_subset.len());
        while let Some(next) = ready.pop_first() {
            out.push(next);
            if let Some(children) = self.parents_to_children.get(&next) {
                for child in children {
                    // get_mut, NOT expect: children outside the subset
                    // are not in sub_indegree and must be skipped, not
                    // panicked on.
                    if let Some(deg) = sub_indegree.get_mut(child) {
                        *deg -= 1;
                        if *deg == 0 {
                            ready.insert(*child);
                        }
                    }
                }
            }
        }
        // Structural-invariant guard (review M-10): mirror `topo_sort`'s
        // cardinality check. `out.len() < in_subset.len()` means some
        // subset member never reached indegree 0 — either the subset
        // induces a cycle (impossible by DAG construction) or indegree
        // bookkeeping has drifted from `parents_to_children`. Either way,
        // returning a truncated sort would silently break the caller's
        // anchor-bounded replay invariants (spec §8.4 step 3). Panic
        // here so the bug surfaces at the boundary instead of becoming
        // a downstream convergence drift.
        #[allow(clippy::panic, clippy::manual_assert)]
        if out.len() != in_subset.len() {
            panic!(
                "topo_sort_subset produced {} events from a subset of {} — \
                 subset has a cycle or indegree bookkeeping has drifted",
                out.len(),
                in_subset.len(),
            );
        }
        out
    }

    /// Build the canonical `Vec<AuthorSeq>` used in [`DriftAnchor`].
    ///
    /// Authors with no events yet (`head_seq == 0`) are filtered out:
    /// a `DriftAnchor` only meaningfully constrains authors that have
    /// actually published. The result is ordered by author pubkey
    /// (lex byte-order, inherited from the `BTreeMap` backing
    /// `by_author`).
    ///
    /// [`DriftAnchor`]: myrhiza_types::DriftAnchor
    #[must_use]
    pub fn author_seq_vec(&self) -> Vec<AuthorSeq> {
        self.by_author
            .iter()
            .filter(|(_, c)| c.head_seq > 0)
            .map(|(author, c)| AuthorSeq {
                author: *author,
                max_seq: c.head_seq,
            })
            .collect()
    }

    /// Build the `Vec<AuthorHead>` used in [`HeadsSummary`].
    ///
    /// Like [`Self::author_seq_vec`] but carries the head wire-hash as
    /// well — used by gossip peers to detect head-divergence and
    /// trigger event requests. Authors with no events are filtered
    /// out.
    ///
    /// [`HeadsSummary`]: myrhiza_types::HeadsSummary
    #[must_use]
    pub fn author_heads(&self) -> Vec<AuthorHead> {
        self.by_author
            .iter()
            .filter(|(_, c)| c.head_seq > 0)
            .map(|(author, c)| AuthorHead {
                author: *author,
                seq: c.head_seq,
                hash: c.head_hash,
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests_genesis {
    use super::*;
    use crate::identity::AuthorKeypair;
    use bincode::Options;
    use myrhiza_types::{Hlc, canonical_bincode};
    use std::collections::BTreeSet;

    pub(super) fn build_genesis(
        kp: &AuthorKeypair,
        bundle_hash: BundleHash,
        seed: [u8; 32],
        topic_name: &str,
        app_payload: Vec<u8>,
    ) -> Event {
        let payload = GenesisV1 {
            seed,
            founder_pubkey: kp.author,
            app_payload,
        };
        let payload_bytes = canonical_bincode().serialize(&payload).expect("encode");
        let body = Event {
            author: kp.author,
            seq: 1,
            prev: EventHash::ZERO,
            deps: BTreeSet::new(),
            hlc: Hlc {
                wall_ms: 0,
                logical: 0,
            },
            payload: payload_bytes,
            signature: [0; 64],
        };
        let body_hash = body.hash_signed_body();
        let sig = kp.sign_body_hash(body_hash);
        let _ = (bundle_hash, topic_name);
        Event {
            signature: sig,
            ..body
        }
    }

    #[test]
    fn genesis_inserts_into_empty_dag() {
        let bundle_hash = BundleHash::from_bytes([0xAA; 32]);
        let seed = [0x11; 32];
        let topic = Topic::derive(&bundle_hash, &seed, "main");
        let mut dag = EventDag::new(topic, bundle_hash, "main".into());
        let kp = AuthorKeypair::deterministic(1);
        let ev = build_genesis(&kp, bundle_hash, seed, "main", vec![0x00; 8]);

        let r = dag.insert(ev).expect("insert");
        assert!(matches!(r, Inserted::NewlyApplied { topo_index: 0, .. }));
    }

    #[test]
    fn genesis_rejected_on_topic_mismatch() {
        let bundle_hash = BundleHash::from_bytes([0xAA; 32]);
        let real_seed = [0x11; 32];
        let other_seed = [0x22; 32];
        let topic = Topic::derive(&bundle_hash, &real_seed, "main");
        let mut dag = EventDag::new(topic, bundle_hash, "main".into());
        let kp = AuthorKeypair::deterministic(1);
        // Build genesis with wrong seed (so derived topic differs).
        let ev = build_genesis(&kp, bundle_hash, other_seed, "main", vec![]);
        let r = dag.insert(ev).expect_err("must reject");
        assert!(matches!(r, DagError::InvalidTopic { .. }));
    }

    #[test]
    fn genesis_rejected_on_bad_signature() {
        let bundle_hash = BundleHash::from_bytes([0xAA; 32]);
        let seed = [0x11; 32];
        let topic = Topic::derive(&bundle_hash, &seed, "main");
        let mut dag = EventDag::new(topic, bundle_hash, "main".into());
        let kp = AuthorKeypair::deterministic(1);
        let mut ev = build_genesis(&kp, bundle_hash, seed, "main", vec![]);
        ev.signature[0] ^= 0xFF; // tamper
        let r = dag.insert(ev).expect_err("must reject");
        assert!(matches!(r, DagError::InvalidSignature));
    }

    #[test]
    fn duplicate_genesis_returns_already_known() {
        let bundle_hash = BundleHash::from_bytes([0xAA; 32]);
        let seed = [0x11; 32];
        let topic = Topic::derive(&bundle_hash, &seed, "main");
        let mut dag = EventDag::new(topic, bundle_hash, "main".into());
        let kp = AuthorKeypair::deterministic(1);
        let ev = build_genesis(&kp, bundle_hash, seed, "main", vec![]);
        dag.insert(ev.clone()).expect("first insert");
        let r = dag.insert(ev).expect("second insert");
        assert!(matches!(r, Inserted::AlreadyKnown));
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests_chain {
    use super::tests_genesis::build_genesis;
    use super::*;
    use crate::identity::AuthorKeypair;
    use myrhiza_types::{Event, Hlc};
    use std::collections::BTreeSet;

    pub(super) fn build_next(kp: &AuthorKeypair, prev: &Event, payload: Vec<u8>) -> Event {
        let body = Event {
            author: kp.author,
            seq: prev.seq + 1,
            prev: prev.wire_hash(),
            deps: BTreeSet::new(),
            hlc: Hlc {
                wall_ms: 0,
                logical: 0,
            },
            payload,
            signature: [0; 64],
        };
        let body_hash = body.hash_signed_body();
        let sig = kp.sign_body_hash(body_hash);
        Event {
            signature: sig,
            ..body
        }
    }

    pub(super) fn fresh_dag_with_genesis() -> (EventDag, AuthorKeypair, Event) {
        let bundle_hash = BundleHash::from_bytes([0xAA; 32]);
        let seed = [0x11; 32];
        let topic = Topic::derive(&bundle_hash, &seed, "main");
        let mut dag = EventDag::new(topic, bundle_hash, "main".into());
        let kp = AuthorKeypair::deterministic(1);
        let g = build_genesis(&kp, bundle_hash, seed, "main", vec![]);
        dag.insert(g.clone()).expect("genesis");
        (dag, kp, g)
    }

    #[test]
    fn next_event_inserts_after_genesis() {
        let (mut dag, kp, g) = fresh_dag_with_genesis();
        let e2 = build_next(&kp, &g, vec![0xCA]);
        let r = dag.insert(e2).expect("insert");
        assert!(matches!(r, Inserted::NewlyApplied { topo_index: 1, .. }));
    }

    #[test]
    fn out_of_order_seq_rejected_as_invalid_chain() {
        let (mut dag, kp, g) = fresh_dag_with_genesis();
        let e2 = build_next(&kp, &g, vec![]);
        // skip ahead — build e3 against e2 BUT don't insert e2.
        let e3 = build_next(&kp, &e2, vec![]);
        let r = dag.insert(e3).expect_err("must reject");
        assert!(matches!(
            r,
            DagError::InvalidChain {
                expected_seq: 2,
                got_seq: 3,
                ..
            }
        ));
    }

    #[test]
    fn equivocation_at_genesis_seq_one_returns_equivocation_error() {
        let bundle_hash = BundleHash::from_bytes([0xAA; 32]);
        let seed = [0x11; 32];
        let topic = Topic::derive(&bundle_hash, &seed, "main");
        let mut dag = EventDag::new(topic, bundle_hash, "main".into());
        let kp = AuthorKeypair::deterministic(1);

        let g1 = build_genesis(&kp, bundle_hash, seed, "main", vec![0xAA]);
        let g2 = build_genesis(&kp, bundle_hash, seed, "main", vec![0xBB]); // different payload
        assert_ne!(g1.wire_hash(), g2.wire_hash());

        dag.insert(g1.clone()).expect("first genesis");
        let r = dag.insert(g2.clone()).expect_err("must reject");
        match r {
            DagError::Equivocation {
                author,
                seq,
                local_hash,
                remote_hash,
            } => {
                assert_eq!(author, kp.author);
                assert_eq!(seq, 1);
                assert_eq!(local_hash, g1.wire_hash());
                assert_eq!(remote_hash, g2.wire_hash());
            }
            other => panic!("expected Equivocation, got {other:?}"),
        }
    }

    #[test]
    fn equivocation_at_non_genesis_seq_returns_equivocation_error() {
        let (mut dag, kp, g) = fresh_dag_with_genesis();
        let e2a = build_next(&kp, &g, vec![0xCA]);
        let e2b = build_next(&kp, &g, vec![0xFE]);
        assert_ne!(e2a.wire_hash(), e2b.wire_hash());
        dag.insert(e2a.clone()).expect("first");
        let r = dag.insert(e2b.clone()).expect_err("equivocation");
        assert!(matches!(r, DagError::Equivocation { seq: 2, .. }));
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests_topo {
    use super::*;
    use crate::identity::AuthorKeypair;

    fn build_dag_chain(n: usize) -> (EventDag, Vec<EventHash>) {
        let bundle_hash = BundleHash::from_bytes([0xAA; 32]);
        let seed = [0x11; 32];
        let topic = Topic::derive(&bundle_hash, &seed, "main");
        let mut dag = EventDag::new(topic, bundle_hash, "main".into());
        let kp = AuthorKeypair::deterministic(1);

        let g = super::tests_genesis::build_genesis(&kp, bundle_hash, seed, "main", vec![]);
        dag.insert(g.clone()).expect("genesis");
        let mut hashes = vec![g.wire_hash()];

        let mut prev = g;
        for _ in 1..n {
            let e = super::tests_chain::build_next(&kp, &prev, vec![]);
            hashes.push(e.wire_hash());
            dag.insert(e.clone()).expect("insert");
            prev = e;
        }
        (dag, hashes)
    }

    #[test]
    fn topo_sort_linear_chain_is_in_insertion_order() {
        let (dag, hashes) = build_dag_chain(5);
        let sorted = dag.topo_sort();
        assert_eq!(sorted, hashes, "linear chain topo-sort == insertion order");
    }

    #[test]
    fn topo_sort_subset_excludes_filtered_events() {
        let (dag, hashes) = build_dag_chain(5);
        // Subset: first 3 events only.
        let allowed: BTreeSet<_> = hashes.iter().take(3).copied().collect();
        let sorted = dag.topo_sort_subset(|e| allowed.contains(&e.wire_hash()));
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted, hashes[..3]);
    }

    #[test]
    fn topo_sort_subset_empty_filter_returns_empty() {
        let (dag, _) = build_dag_chain(3);
        let sorted = dag.topo_sort_subset(|_| false);
        assert!(sorted.is_empty());
    }

    #[test]
    fn author_seq_vec_matches_chain_heads() {
        let (dag, _) = build_dag_chain(4);
        let asv = dag.author_seq_vec();
        assert_eq!(asv.len(), 1);
        assert_eq!(asv[0].max_seq, 4);
    }

    #[test]
    fn author_heads_matches_chain_heads_with_hash() {
        let (dag, hashes) = build_dag_chain(4);
        let heads = dag.author_heads();
        assert_eq!(heads.len(), 1);
        assert_eq!(heads[0].seq, 4);
        assert_eq!(heads[0].hash, *hashes.last().expect("non-empty"));
    }

    /// Covers: convergence.md §4.3 (deterministic topo-sort under arbitrary
    /// insertion order). Review I-5.
    ///
    /// Build a fixed multi-author DAG (founder + 4 non-founders, each a
    /// 4-event chain = 21 events incl. genesis). Insert in a reference
    /// order, capture the topo-sort. Then perform N=100 random shuffles
    /// of the *tail* of the insertion order (genesis stays at index 0 —
    /// no other event is insertable before it because every other event
    /// transitively requires it). For each shuffle, insert into a fresh
    /// DAG and assert byte-identical topo-sort output vs. the reference.
    ///
    /// A failure here means the per-event tie-break (lex byte-order on
    /// `EventHash` via `BTreeSet::pop_first`) is no longer the sole
    /// determinant of ordering — i.e., insertion order is leaking into
    /// the output, which would break cross-peer convergence.
    #[test]
    fn topo_sort_is_invariant_under_insertion_order_shuffle() {
        use super::tests_chain::build_next;
        use super::tests_genesis::build_genesis;
        use rand::SeedableRng;
        use rand::seq::SliceRandom;

        let bundle_hash = BundleHash::from_bytes([0xAA; 32]);
        let seed = [0x11; 32];
        let topic_name = "main";
        let topic = Topic::derive(&bundle_hash, &seed, topic_name);

        // Founder authors the genesis + 3 follow-on events (4 total).
        let founder = AuthorKeypair::deterministic(0xF0);
        let genesis = build_genesis(&founder, bundle_hash, seed, topic_name, vec![]);

        // Build the full event list deterministically.
        //   - founder chain: genesis (seq=1) + seq=2..=4 = 4 events
        //   - 4 non-founder chains: seq=1..=4 each = 16 events
        //   - extra founder event seq=5 (so founder also has 5 events)
        //   total: 21
        // Layout chosen to match plan-B-1 spec §4.3 ("5-author × 4-event
        // chain + genesis = 21 events"): 5 authors total, founder's
        // chain has 5 events including genesis, the other 4 have 4.
        let mut all_events: Vec<Event> = vec![genesis.clone()];
        let mut founder_prev = genesis.clone();
        for _ in 0..4 {
            let e = build_next(&founder, &founder_prev, vec![]);
            all_events.push(e.clone());
            founder_prev = e;
        }
        for seed_byte in [1u8, 2, 3, 4] {
            let a = AuthorKeypair::deterministic(u64::from(seed_byte));
            // Non-founder seq=1: prev = ZERO, deps = empty.
            // DAG adds the implicit genesis edge at insert time.
            let seq1 = build_non_founder_seq1(&a);
            all_events.push(seq1.clone());
            let mut prev = seq1;
            for _ in 0..3 {
                let e = build_next(&a, &prev, vec![]);
                all_events.push(e.clone());
                prev = e;
            }
        }
        assert_eq!(
            all_events.len(),
            21,
            "1 founder chain (5) + 4 non-founder chains (4 each) = 21 events"
        );

        // Reference: insert in declared order, record topo-sort.
        let reference: Vec<EventHash> = {
            let mut dag = EventDag::new(topic, bundle_hash, topic_name.into());
            for e in &all_events {
                let _ = dag.insert(e.clone()).expect("reference insert");
            }
            dag.topo_sort()
        };
        assert_eq!(
            reference.len(),
            all_events.len(),
            "reference topo-sort must cover every inserted event"
        );

        // 100 shuffles of the tail (genesis stays at index 0 — without
        // it, downstream events fail chain validation and never apply).
        //
        // For each shuffle, we mirror the runtime's pending-buffer
        // discipline: events whose chain predecessor is not yet present
        // are deferred to a re-try list. Pass over events, insert any
        // that succeed (or are AlreadyKnown), defer the rest. Repeat
        // until a pass makes no progress. Convergence is finite because
        // each successful insert strictly reduces the deferred set.
        let mut rng = rand::rngs::StdRng::seed_from_u64(0xB1_5F_FE);
        for trial in 0..100 {
            let mut shuffled = all_events.clone();
            shuffled[1..].shuffle(&mut rng);

            let mut dag = EventDag::new(topic, bundle_hash, topic_name.into());
            let mut deferred: Vec<Event> = shuffled;
            loop {
                let mut next_deferred = Vec::with_capacity(deferred.len());
                let mut progressed = false;
                for e in deferred.drain(..) {
                    match dag.insert(e.clone()) {
                        Ok(Inserted::NewlyApplied { .. }) => progressed = true,
                        Ok(Inserted::AlreadyKnown) => {}
                        // InvalidChain / Pending: retry next pass.
                        _ => next_deferred.push(e),
                    }
                }
                deferred = next_deferred;
                if !progressed {
                    break;
                }
            }
            assert!(
                deferred.is_empty(),
                "trial {trial}: {} events failed to insert after convergence",
                deferred.len(),
            );

            let actual = dag.topo_sort();
            assert_eq!(
                actual.len(),
                reference.len(),
                "trial {trial}: topo-sort length mismatch ({} != {})",
                actual.len(),
                reference.len(),
            );
            assert_eq!(
                actual, reference,
                "trial {trial}: topo-sort diverged from reference under shuffled insertion order"
            );
        }
    }

    /// Build a non-founder seq=1 event (prev=ZERO, deps=empty). The DAG
    /// adds the implicit Genesis parent edge at insert time per
    /// `EventDag::insert` step 6 — the event itself does not name it.
    fn build_non_founder_seq1(kp: &AuthorKeypair) -> Event {
        use myrhiza_types::Hlc;
        let body = Event {
            author: kp.author,
            seq: 1,
            prev: EventHash::ZERO,
            deps: BTreeSet::new(),
            hlc: Hlc {
                wall_ms: 0,
                logical: 0,
            },
            payload: vec![],
            signature: [0; 64],
        };
        let body_hash = body.hash_signed_body();
        let sig = kp.sign_body_hash(body_hash);
        Event {
            signature: sig,
            ..body
        }
    }
}
