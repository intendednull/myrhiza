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
    AuthorPubkey, BundleHash, Event, EventHash, GenesisV1, Topic, decode_canonical,
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

        // Step 3: genesis-specific validation.
        if event.seq == 1 {
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
        // Parents = deps ∪ {prev (if not ZERO)}; indegree records the
        // total parent count so the topo-sort planned for B-1 Task 13
        // can decrement as parents are resolved.
        let mut parents: BTreeSet<EventHash> = event.deps.clone();
        if event.prev != EventHash::ZERO {
            parents.insert(event.prev);
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
        self.by_hash.insert(wire_hash, event);

        Ok(Inserted::NewlyApplied {
            topo_index,
            hash: wire_hash,
        })
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
