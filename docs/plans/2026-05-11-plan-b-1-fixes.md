# Plan B-1 review-fixes implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve all findings from the two fresh reviewers (spec-compliance + code-quality) of branch `feat/plan-b-1-dag-memnet` so PR #2 is spec-compliant, has no silent-drop traps, and carries explicit TODOs for the two B-2 deferrals.

**Architecture:** Each task is scoped to a single concern + commit. TDD where the fix has an observable surface (new test or fixture); plain edit where the fix is style / TODO / comment cleanup. All fixes preserve the existing acceptance test set; no test is relaxed.

**Tech Stack:** Same as plan B-1 — Rust 2024, tokio (broadcast/sync), bincode 1.3.3 fixint big-endian, ed25519-dalek 2.1, async-trait. No new dependencies.

**Source review reports:** captured inline below per task (cross-reference).

---

## File Structure

Files modified by this plan, grouped by concern:

```
crates/kernel/src/
├── runtime.rs            — I-1 (HeadsRequest substitution), I-2 (DriftRateLimited variant shape), Q-2 (Pending re-arm), Q-3 escape hatch wiring, M-4 (dropped_at_apply map), N-1/N-12 (comment cleanup), deferred TODOs
├── dag.rs                — M-10 (cycle-detect assert), I-5 (shuffle property test), N-13 (build_genesis args)
├── drift.rs              — Q-5 (sliding-window fix), N-14 (clock injection)
└── identity.rs           — I-4 (PeerKeypair::generate)

crates/network/src/
└── memory.rs             — M-3 (MemBus::inject_lag)

crates/test-utils/src/
└── harness.rs            — Q-3 (await_digest pre-wait fix), N-3 ("deterministically" comment)

crates/types/tests/
└── wire_freeze.rs        — I-3 / M-11 (GossipMessage variant tag freeze)

crates/kernel/tests/
├── convergence.rs        — M-8 (Runtime-level equivocation test), M-9 (deterministic lag-recovery assertion)
└── helpers/mod.rs        — supporting test helpers if needed
```

Each task below cites the originating finding ID(s) so the diff trail back to the review is explicit.

---

## Task 1: GossipMessage wire-freeze snapshot

**Findings:** I-3 (spec §6.2) + M-11 (code-quality).

The spec says `GossipMessage` is "wire-frozen" with per-variant canonical encoding pinned. The existing `wire_freeze.rs` covers inner payloads but not the outer enum's variant tags. A variant reorder would silently break wire compat. Fix: add four `gossip_message_<variant>_wire_layout` tests that assert the exact bytes for each arm.

**Files:**
- Modify: `crates/types/tests/wire_freeze.rs`

- [ ] **Step 1: Write the failing tests**

Append to `crates/types/tests/wire_freeze.rs`:

```rust
#[test]
fn gossip_message_event_variant_tag_is_zero_u32_be() {
    use myrhiza_network::GossipMessage;
    use myrhiza_types::canonical_bincode;
    use bincode::Options;

    let env = sample_event_envelope();
    let msg = GossipMessage::Event(env);
    let bytes = canonical_bincode().serialize(&msg).expect("encode");
    assert_eq!(&bytes[..4], &[0x00, 0x00, 0x00, 0x00], "variant tag for Event");
}

#[test]
fn gossip_message_heads_summary_variant_tag_is_one_u32_be() {
    use myrhiza_network::GossipMessage;
    use myrhiza_types::{canonical_bincode, dag::{HeadsSummary, AuthorHead}};
    use bincode::Options;

    let hs = HeadsSummary { topic: sample_topic(), heads: vec![] };
    let msg = GossipMessage::HeadsSummary(hs);
    let bytes = canonical_bincode().serialize(&msg).expect("encode");
    assert_eq!(&bytes[..4], &[0x00, 0x00, 0x00, 0x01], "variant tag for HeadsSummary");
}

#[test]
fn gossip_message_heads_request_variant_tag_is_two_u32_be() {
    use myrhiza_network::GossipMessage;
    use myrhiza_types::{canonical_bincode, dag::HeadsRequest};
    use bincode::Options;

    let hr = HeadsRequest { topic: sample_topic(), author: sample_author_pubkey(), from_seq: 1, to_seq: 1 };
    let msg = GossipMessage::HeadsRequest(hr);
    let bytes = canonical_bincode().serialize(&msg).expect("encode");
    assert_eq!(&bytes[..4], &[0x00, 0x00, 0x00, 0x02], "variant tag for HeadsRequest");
}

#[test]
fn gossip_message_drift_variant_tag_is_three_u32_be() {
    use myrhiza_network::GossipMessage;
    use myrhiza_types::canonical_bincode;
    use bincode::Options;

    let msg = GossipMessage::Drift(sample_drift_message());
    let bytes = canonical_bincode().serialize(&msg).expect("encode");
    assert_eq!(&bytes[..4], &[0x00, 0x00, 0x00, 0x03], "variant tag for Drift");
}
```

Add helpers (`sample_event_envelope`, `sample_topic`, `sample_author_pubkey`, `sample_drift_message`) at module scope — read the existing file for canonical wire-fixture construction and reuse the same pattern. If `myrhiza-network` is not already a `[dev-dependencies]` entry of `myrhiza-types`, add it.

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p myrhiza-types --test wire_freeze
```

Expected: four new tests fail (likely "use of undeclared crate `myrhiza_network`" first; resolve by adding the dev-dep, then expect the byte asserts to pass on first try IF current variant order is `Event=0, HeadsSummary=1, HeadsRequest=2, Drift=3`).

- [ ] **Step 3: If the tests fail because GossipMessage variant order differs, decide before changing**

Read `crates/network/src/lib.rs` to confirm variant declaration order. The wire byte assertions must reflect what is currently on the wire — DO NOT change `GossipMessage` to make the tests pass. If the order in code differs from the asserts above, update the asserts (and document the actual order in the spec §6.2 if it does not already match).

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p myrhiza-types --test wire_freeze
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/types/tests/wire_freeze.rs crates/types/Cargo.toml
git commit -m "test(types): wire-freeze GossipMessage variant tags (review I-3/M-11)"
```

---

## Task 2: Topo-sort shuffle property test (N=100)

**Finding:** I-5 (spec §4.3).

Spec calls for a property test confirming randomly-shuffled insertion order produces identical topo-sort output across N=100 shuffles. Without it, the deterministic-topo-sort claim is asserted but not exercised under varying orderings.

**Files:**
- Modify: `crates/kernel/src/dag.rs`
- Modify: `crates/kernel/Cargo.toml` — add `rand = "0.8"` to `[dev-dependencies]` if not already present.

- [ ] **Step 1: Add rand dev-dep if missing**

Check `crates/kernel/Cargo.toml`. If `rand` is not in `[dev-dependencies]`, add:

```toml
[dev-dependencies]
rand = "0.8"
```

- [ ] **Step 2: Write the failing test**

In `crates/kernel/src/dag.rs`, inside `mod tests_topo`, add:

```rust
#[test]
fn topo_sort_is_invariant_under_insertion_order_shuffle() {
    use rand::seq::SliceRandom;
    use rand::SeedableRng;

    // Build a fixed DAG: a 5-author × 4-event chain (20 events) plus genesis.
    let bundle_hash = BundleHash::from_bytes([0xAA; 32]);
    let topic_name = "main";
    let seed = [0x11; 32];
    let topic = Topic::derive(&bundle_hash, &seed, topic_name);

    let founder = AuthorKeypair::deterministic([0xF0; 32]);
    let genesis = build_genesis(&founder, seed, bundle_hash, topic_name);
    let mut all_events: Vec<Event> = vec![genesis.clone()];

    let mut chain_heads: Vec<(AuthorKeypair, EventHash, u64)> = vec![
        (founder.clone(), event_hash(&genesis), 1),
    ];
    for seed_byte in [0x01u8, 0x02, 0x03, 0x04] {
        let a = AuthorKeypair::deterministic([seed_byte; 32]);
        // Each non-founder author posts seq=1 with genesis as its prior head.
        // (Plan-B genesis-applicability gate accepts non-founder seq=1.)
        let mut prev = event_hash(&genesis);
        let mut seq = 0u64;
        for _ in 0..4 {
            seq += 1;
            let e = build_chain_event(&a, &topic, seq, &[prev]);
            prev = event_hash(&e);
            all_events.push(e);
        }
        let _ = chain_heads;  // chain_heads not needed beyond the loop
    }

    // Insert in original order; capture reference topo-sort.
    let reference: Vec<EventHash> = {
        let mut dag = EventDag::new(topic, bundle_hash, topic_name.to_string());
        for e in &all_events {
            let _ = dag.insert(e.clone());
        }
        dag.topo_sort().iter().map(|e| event_hash(e)).collect()
    };

    // 100 random shuffles must produce the same topo-sort.
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xB1_F1_5HUFF1E);
    for trial in 0..100 {
        let mut shuffled = all_events.clone();
        // Genesis must remain at index 0 — it is a precondition of every other
        // event's chain validity. Shuffle the tail only.
        let (head, tail) = shuffled.split_first_mut().unwrap();
        let _ = head;
        let tail_slice: &mut [Event] = tail;
        tail_slice.shuffle(&mut rng);

        let mut dag = EventDag::new(topic, bundle_hash, topic_name.to_string());
        for e in &shuffled {
            let _ = dag.insert(e.clone());
        }
        let actual: Vec<EventHash> = dag.topo_sort().iter().map(|e| event_hash(e)).collect();
        assert_eq!(
            reference, actual,
            "trial {} produced divergent topo-sort", trial
        );
    }
}
```

Helper `build_chain_event` may need to be added (or use the existing `EventBuilder` from `myrhiza-test-utils` if the dev-dep cycle is acceptable). If not, write a small inline helper that mirrors `EventBuilder::next` semantics.

- [ ] **Step 3: Run test to verify it fails first time (if it does)**

```bash
cargo test -p myrhiza-kernel topo_sort_is_invariant_under_insertion_order_shuffle
```

Expected: PASS on first attempt because topo-sort already uses lex byte tie-break + implicit Genesis parent edge. **If it fails:** stop and investigate — that is the actual bug the property test exists to catch, and the spec's determinism claim is wrong. Do not adjust the test to make it pass.

- [ ] **Step 4: Commit**

```bash
git add crates/kernel/src/dag.rs crates/kernel/Cargo.toml
git commit -m "test(kernel): topo-sort invariant under 100-shuffle insertion order (review I-5)"
```

---

## Task 3: topo_sort_subset cycle-detection assert

**Finding:** M-10.

`topo_sort` panics on output-length mismatch (structural-invariant guard). `topo_sort_subset` does not — a future bug in indegree maintenance would silently return a partial sort, producing wrong replay state and silent divergence. Adding the assert costs one line.

**Files:**
- Modify: `crates/kernel/src/dag.rs`

- [ ] **Step 1: Locate the function**

Find `topo_sort_subset` in `crates/kernel/src/dag.rs` (around line 499 per review).

- [ ] **Step 2: Add the assert at the end**

After the sort loop's final write to `out`, before returning, insert:

```rust
assert_eq!(
    out.len(),
    subset.len(),
    "topo_sort_subset structural invariant: subset has a cycle or indegree drift"
);
```

(Use the actual local name for the subset cardinality — check the surrounding code; if `subset` is a `&BTreeSet<EventHash>` the count is `subset.len()`. If indegree was built only over subset members and may include implicit-genesis parents not in the subset, count what was actually queued for processing, not the input subset.)

- [ ] **Step 3: Run tests to confirm nothing breaks**

```bash
cargo test -p myrhiza-kernel
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kernel/src/dag.rs
git commit -m "fix(kernel): assert topo_sort_subset output length (review M-10)"
```

---

## Task 4: Replace HeadsSummary substitution with HeadsRequest

**Findings:** I-1 (spec §7.2) + N-2 (`let _ = missing` discard).

When `Inserted::Pending(missing)` is returned, the spec says publish a targeted `HeadsRequest` for the authors who own the missing events. The current code ignores `missing` and publishes a `HeadsSummary` instead, adding an unnecessary RTT and discarding the typed information the DAG already produced.

**Files:**
- Modify: `crates/kernel/src/runtime.rs`
- Modify: `crates/kernel/src/dag.rs` — `Inserted::Pending` may need to carry the author + last-known-seq pair instead of bare `BTreeSet<EventHash>`, so the runtime can build a `HeadsRequest` per author. Decide between two designs:

  **Design A** (preferred — minimal): the runtime walks each missing dep hash, looks it up in `by_hash` (it won't be there — that's why it's missing), but the *event whose deps were missing* (the inbound event itself) names its parents in `event.parents`, and each parent in `by_hash` would carry author info. For deps that are truly absent, the runtime can fall back to publishing a `HeadsSummary` only when no author is derivable. Simpler approach: pending events themselves carry `event.author`, so the runtime can issue a `HeadsRequest` for *that* author asking for everything from `event.author`'s last-known seq through `event.seq - 1`.

  **Design B** (richer): change `Inserted::Pending` to `Inserted::Pending { missing: BTreeSet<EventHash>, author_hints: BTreeMap<AuthorPubkey, AuthorSeq> }` so the runtime emits one `HeadsRequest` per author.

Use **Design A** — it is purely additive on the runtime side and does not change the DAG's return shape. (Design B is a B-2 ergonomic improvement.)

- [ ] **Step 1: Write a failing test in convergence.rs**

Add a test that proves the protocol shape:

```rust
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn pending_event_triggers_heads_request_not_heads_summary() {
    // Two peers A and B. A authors events 1..=3. Deliver events out of order
    // to B: event 3 first (parents missing). Capture all gossip messages B
    // publishes between receiving event 3 and converging. Assert at least one
    // is GossipMessage::HeadsRequest { author: A, from_seq: 2, to_seq: 2 }.
    // No HeadsSummary should be required for this recovery path under
    // current spec.

    // Test body uses InProcessHarness with a MemBus tap to capture B's outbound
    // gossip stream. See `crates/test-utils/src/harness.rs` for the publish-tap
    // pattern (add one if absent — this is a legitimate test affordance).
    todo!("write test per Design A — author derived from event.author");
}
```

The `todo!()` is acceptable here only because the next step writes the real body. **DO NOT commit this step with the todo.**

- [ ] **Step 2: Implement the test body**

Replace the `todo!()` with a real test. Use the existing convergence-test patterns from `crates/kernel/tests/convergence.rs`. Capture publishes via `MemBus::subscribe(topic)` from a third tap-subscriber that records what B emits. Assert the first non-Event message B publishes after receiving the out-of-order event 3 is a `HeadsRequest { author: A_pubkey, from_seq: 2, to_seq: 2 }` (or a range that covers it).

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test -p myrhiza-kernel --test convergence pending_event_triggers_heads_request_not_heads_summary
```

Expected: FAIL — current implementation publishes `HeadsSummary` instead.

- [ ] **Step 4: Fix the runtime**

In `crates/kernel/src/runtime.rs`, find `request_missing_authors` (around line 614). Replace its body. Sketch:

```rust
fn request_missing_authors(&mut self, event: &Event, _missing: &BTreeSet<EventHash>) {
    let chain = self.dag.author_chain(&event.author);
    let known_head_seq = chain.map(|c| c.head_seq).unwrap_or(0);
    if event.seq <= known_head_seq + 1 {
        // No gap — pending must be on a *different* author's chain. Fall back to
        // HeadsSummary nudge for the cross-author case.
        let summary = self.build_heads_summary();
        let _ = self.network.publish(self.topic, GossipMessage::HeadsSummary(summary));
        return;
    }
    let req = HeadsRequest {
        topic: self.topic,
        author: event.author,
        from_seq: known_head_seq + 1,
        to_seq: event.seq.saturating_sub(1),
    };
    let _ = self.network.publish(self.topic, GossipMessage::HeadsRequest(req));
}
```

(Adapt names — `self.network` may be `self.network` via `NetworkErased`; `self.topic` from the runtime struct; `self.dag.author_chain` may need to be added as an accessor returning `Option<&AuthorChain>`.)

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo test -p myrhiza-kernel --test convergence pending_event_triggers_heads_request_not_heads_summary
```

Expected: PASS. Also re-run the full convergence suite to confirm no regressions:

```bash
cargo test -p myrhiza-kernel --test convergence
```

- [ ] **Step 6: Commit**

```bash
git add crates/kernel/src/runtime.rs crates/kernel/tests/convergence.rs
git commit -m "fix(kernel): publish HeadsRequest for known-author pending events (review I-1)

Replace HeadsSummary substitution in request_missing_authors with a
targeted HeadsRequest derived from event.author and the DAG's known
head_seq for that author. Cross-author pending falls back to
HeadsSummary nudge as before.

Closes review-finding I-1 (spec §7.2)."
```

---

## Task 5: DriftRateLimited struct-variant shape

**Finding:** I-2 (spec §13).

Spec declares `DriftRateLimited { kind: RateLimitKind }`. Code has `DriftRateLimited(RateLimitKind)` tuple variant. Mechanical fix.

**Files:**
- Modify: `crates/kernel/src/runtime.rs`

- [ ] **Step 1: Change the variant declaration**

At `crates/kernel/src/runtime.rs:170`:

```rust
DriftRateLimited { kind: RateLimitKind },
```

- [ ] **Step 2: Update the construction site**

At `crates/kernel/src/runtime.rs:962`:

```rust
.push(PeerWarning::DriftRateLimited { kind });
```

- [ ] **Step 3: Update any pattern-match sites**

```bash
grep -rn "DriftRateLimited" crates/ tests/
```

Update each `DriftRateLimited(k)` to `DriftRateLimited { kind: k }`. There may be no test pattern-matches; that is OK.

- [ ] **Step 4: Run tests**

```bash
cargo test -p myrhiza-kernel
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/kernel/src/runtime.rs
git commit -m "fix(kernel): DriftRateLimited becomes struct variant per spec §13 (review I-2)"
```

---

## Task 6: DriftRateLimit sliding-window correction + clock injection

**Findings:** Q-5 (window flaw) + N-14 (clock injection).

Two bugs in one struct:
1. `today_started` resets to `now` on rollover and `today_count` resets to 0, allowing `2 * daily_cap` bursts across a 24h boundary (resetting window, not sliding).
2. `DriftRateLimit::new` calls `Instant::now()` at construction, making the struct non-deterministic in tests.

**Files:**
- Modify: `crates/kernel/src/drift.rs`
- Modify: `crates/kernel/src/runtime.rs` — pass `now` into `DriftRateLimit::new` from the Runtime's start time.

- [ ] **Step 1: Write the failing sliding-window test**

In `crates/kernel/src/drift.rs` tests module, add:

```rust
#[test]
fn daily_cap_does_not_admit_double_burst_across_rollover() {
    let mut rl = DriftRateLimit::new(
        Instant::now(),
        Duration::from_secs(60),  // min_interval
        4,                         // daily_cap
    );
    let t0 = Instant::now();

    // Burn the daily cap right before rollover.
    let near_rollover = t0 + Duration::from_secs(24 * 60 * 60 - 1);
    let mut t = near_rollover;
    for _ in 0..4 {
        rl.try_emit(t).expect("within daily cap");
        t += Duration::from_secs(60);
    }
    // Immediately after rollover, true sliding window should still reject —
    // because the prior 4 emits all sit inside the trailing 24h window.
    let after_rollover = t0 + Duration::from_secs(24 * 60 * 60 + 1);
    let r = rl.try_emit(after_rollover);
    assert!(
        matches!(r, Err(RateLimitKind::DailyCap)),
        "sliding window must still reject across rollover, got {:?}",
        r
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p myrhiza-kernel --lib drift::tests::daily_cap_does_not_admit_double_burst_across_rollover
```

Expected: FAIL — current code resets count to 0 on rollover and admits the post-rollover emit.

- [ ] **Step 3: Replace counter with timestamp ring**

Replace the `today_started` + `today_count` pair with a `VecDeque<Instant>` ring of recent emit times. Logic:

```rust
pub struct DriftRateLimit {
    min_interval: Duration,
    daily_cap: usize,
    last_emit: Option<Instant>,
    recent_emits: std::collections::VecDeque<Instant>,
}

impl DriftRateLimit {
    pub fn new(now: Instant, min_interval: Duration, daily_cap: usize) -> Self {
        let _ = now;  // accepted for symmetry; only used if we want to seed an empty window
        Self {
            min_interval,
            daily_cap,
            last_emit: None,
            recent_emits: std::collections::VecDeque::with_capacity(daily_cap + 1),
        }
    }

    pub fn try_emit(&mut self, now: Instant) -> Result<(), RateLimitKind> {
        let day = Duration::from_secs(24 * 60 * 60);
        while let Some(&front) = self.recent_emits.front() {
            if now.duration_since(front) >= day {
                self.recent_emits.pop_front();
            } else {
                break;
            }
        }
        if self.recent_emits.len() >= self.daily_cap {
            return Err(RateLimitKind::DailyCap);
        }
        if let Some(prev) = self.last_emit {
            if now.duration_since(prev) < self.min_interval {
                return Err(RateLimitKind::MinInterval);
            }
        }
        self.recent_emits.push_back(now);
        self.last_emit = Some(now);
        Ok(())
    }
}
```

This is a true trailing-24h sliding window.

- [ ] **Step 4: Update all call sites**

```bash
grep -rn "DriftRateLimit::new" crates/
```

Update each site to pass `now: Instant`. The Runtime call site is in `Runtime::start` — use `Instant::now()` at start (or wire from `RuntimeCfg::start_instant` if such a field is added; not required for this fix).

- [ ] **Step 5: Run tests**

```bash
cargo test -p myrhiza-kernel
```

Expected: PASS, including both the existing rate-limit tests AND the new sliding-window test.

- [ ] **Step 6: Commit**

```bash
git add crates/kernel/src/drift.rs crates/kernel/src/runtime.rs
git commit -m "fix(kernel): DriftRateLimit uses true trailing-24h sliding window (review Q-5/N-14)

Replace the resetting today_count/today_started pair with a VecDeque<Instant>
of recent emit timestamps. A pre-rollover burst of daily_cap emits now correctly
blocks subsequent emits until the trailing 24h window evicts the oldest.

Construction also accepts an explicit now: Instant for deterministic tests."
```

---

## Task 7: PeerKeypair::generate

**Finding:** I-4 (spec §10).

Spec lists `PeerKeypair::generate<R: CryptoRng + RngCore>(rng: &mut R) -> Self`. Code only has `from_secret_bytes` + `deterministic`.

**Files:**
- Modify: `crates/kernel/src/identity.rs`

- [ ] **Step 1: Write the failing test**

In `crates/kernel/src/identity.rs` tests module:

```rust
#[test]
fn peer_keypair_generate_with_csprng_produces_distinct_keys() {
    use rand::rngs::OsRng;
    let mut rng = OsRng;
    let a = PeerKeypair::generate(&mut rng);
    let b = PeerKeypair::generate(&mut rng);
    assert_ne!(a.public(), b.public(), "two CSPRNG draws must differ");
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p myrhiza-kernel --lib identity::tests::peer_keypair_generate_with_csprng_produces_distinct_keys
```

Expected: FAIL — `PeerKeypair::generate` does not exist.

- [ ] **Step 3: Add the method**

In the `PeerKeypair` impl block:

```rust
pub fn generate<R: rand_core::CryptoRng + rand_core::RngCore>(rng: &mut R) -> Self {
    let signing = ed25519_dalek::SigningKey::generate(rng);
    Self::from_signing_key(signing)
}
```

(Adapt to whichever private constructor pattern the file uses — `from_secret_bytes` likely takes `[u8; 32]`; if so, draw 32 bytes via `rng.fill_bytes(&mut buf)` and call that.)

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p myrhiza-kernel --lib identity::tests::peer_keypair_generate_with_csprng_produces_distinct_keys
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/kernel/src/identity.rs
git commit -m "feat(kernel): PeerKeypair::generate<R: CryptoRng + RngCore> (review I-4)"
```

---

## Task 8: MemBus::inject_lag test helper

**Finding:** M-3 (spec §6.3).

Spec names `inject_lag(topic)` as a `cfg(test)` affordance for deterministic lag-recovery testing. Without it, the existing `lagged_broadcast_recovers_via_heads_summary` test relies on natural capacity overflow, which is non-deterministic.

**Files:**
- Modify: `crates/network/src/memory.rs`

- [ ] **Step 1: Write the failing test**

In `crates/network/tests/memory_basic.rs`:

```rust
#[tokio::test]
async fn inject_lag_forces_next_recv_to_return_lagged() {
    use myrhiza_network::{MemBus, MemNetwork, Network, SubError, GossipMessage};
    let bus = MemBus::new_with_capacity(8);
    let net = MemNetwork::new(bus.clone());
    let topic = sample_topic();
    let mut sub = net.subscribe(topic).await.expect("subscribe");
    bus.inject_lag(topic);
    let r = sub.recv().await;
    assert!(matches!(r, Err(SubError::Lagged(_))), "expected Lagged, got {:?}", r);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p myrhiza-network inject_lag_forces_next_recv_to_return_lagged
```

Expected: FAIL — method does not exist.

- [ ] **Step 3: Implement inject_lag**

In `crates/network/src/memory.rs`, on the `MemBus` impl, add:

```rust
#[cfg(any(test, feature = "test-helpers"))]
pub fn inject_lag(&self, topic: crate::Topic) {
    // Insert a sentinel value into the broadcast channel that causes the
    // subscriber to observe a Lagged error on next recv. Simplest: drain the
    // channel of capacity-1 messages so the next send overflows; but the
    // cleanest is to publish (capacity + 1) tombstone messages without any
    // active receiver having read them.
    // ...implementation detail; the spec only requires the method exists and
    // its effect is deterministic Lagged on next recv.
}
```

Pick the simplest correct implementation. One option: keep an `Arc<Mutex<HashSet<Topic>>>` of "lagged" topics; in `subscribe`, the returned `Subscription::recv` checks the flag first and returns `Lagged(1)` once, clearing the flag. Choose the design that fits the existing `MemBus` internals best — read the file first.

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p myrhiza-network inject_lag_forces_next_recv_to_return_lagged
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/network/src/memory.rs crates/network/tests/memory_basic.rs
git commit -m "feat(network): MemBus::inject_lag for deterministic lag-recovery tests (review M-3)"
```

---

## Task 9: Tighten lag-recovery test to use inject_lag

**Finding:** M-9 / Q-9.

Existing `lagged_broadcast_recovers_via_heads_summary` accepts `converged || lagged_seen`. With `inject_lag` available, both can be required deterministically.

**Files:**
- Modify: `crates/kernel/tests/convergence.rs`

- [ ] **Step 1: Update the test**

Locate `lagged_broadcast_recovers_via_heads_summary` (around line 475 per review). Replace the natural-overflow setup with `bus.inject_lag(topic)` invoked after subscribing, before delivery. Assert both:

```rust
assert!(lagged_seen, "BroadcastLagged warning must be recorded");
assert!(converged, "B must converge after lag recovery");
```

(Not `lagged_seen || converged`.)

- [ ] **Step 2: Run test to verify it still passes**

```bash
cargo test -p myrhiza-kernel --test convergence lagged_broadcast_recovers_via_heads_summary
```

Expected: PASS. **If it fails:** the lag-recovery path has a real defect; do not relax the assertion.

- [ ] **Step 3: Commit**

```bash
git add crates/kernel/tests/convergence.rs
git commit -m "test(kernel): require both lag warning AND convergence in lag-recovery test (review M-9)"
```

---

## Task 10: Runtime-level equivocation acceptance test

**Finding:** M-8.

Existing `equivocation_author_chain_first_seen_wins` exercises `EventDag::insert` directly. A cross-peer Runtime-level test would catch a future bug where the Runtime swallows `DagError::Equivocation` or fails to push it to `peer_warnings`.

**Files:**
- Modify: `crates/kernel/tests/convergence.rs`

- [ ] **Step 1: Write the test**

```rust
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn equivocation_via_membus_surfaces_in_peer_warnings() {
    // Setup: two peers A and B, shared MemBus, same topic, same bundle.
    // A authors event-1 normally. Construct a *second*, conflicting event-1
    // signed by A's authoring key (use AuthorKeypair::deterministic with a
    // different "claimed seq"... no — must reuse seq=1 with different body).
    // Publish both to the topic.
    // B observes both, accepts the first, flags the second.
    // Assert: B.equivocation_log() contains exactly one entry naming author=A.

    // Use the existing EventBuilder to construct two divergent seq=1 events
    // with the same author key but different app payloads.
    todo!()
}
```

- [ ] **Step 2: Implement and run**

Implement the test body. Use existing harness helpers. Verify `RuntimeHandle::equivocation_log()` (or whatever accessor exists; add one if missing — read `runtime.rs` first) returns one entry.

```bash
cargo test -p myrhiza-kernel --test convergence equivocation_via_membus_surfaces_in_peer_warnings
```

Expected: PASS.

- [ ] **Step 3: Keep the existing DAG-level test**

Do NOT delete `equivocation_author_chain_first_seen_wins` — it is a useful unit test for the DAG itself. The new test is additive, not a replacement.

- [ ] **Step 4: Commit**

```bash
git add crates/kernel/tests/convergence.rs
git commit -m "test(kernel): equivocation surfaces via cross-peer MemBus path (review M-8)"
```

---

## Task 11: Fix await_digest pre-wait race

**Finding:** Q-3.

`PeerHandle::await_digest` calls `mark_unchanged()` then checks `*digest_watch.borrow() == expected` before awaiting `changed()`. If the digest already equals `expected` at call time (test-state leakage across reuse, or a prior synchronous emit), the function returns `true` immediately without confirming any new delivery happened — the test passes vacuously.

**Files:**
- Modify: `crates/test-utils/src/harness.rs`

- [ ] **Step 1: Write a failing regression test**

In `crates/test-utils/src/harness.rs` tests module (add one if absent):

```rust
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn await_digest_does_not_return_on_stale_already_equal_state() {
    // Construct a PeerHandle whose digest_watch already holds `target_digest`
    // BEFORE await_digest is called. await_digest should NOT return true; it
    // should wait for a fresh `changed()` signal.
    //
    // (If the test setup forces the watch to hold target_digest without ever
    // signaling change after mark_unchanged, the function should block — assert
    // via tokio::time::timeout that it returns Err(Elapsed) within 100ms.)
    todo!("write test")
}
```

- [ ] **Step 2: Run test to verify it fails**

Expected: with current code, function returns `Ok(true)` immediately — test fails because elapsed timeout was expected.

- [ ] **Step 3: Fix the function**

Remove the pre-wait equality check. The semantic should be: `await_digest` always waits for at least one fresh `changed()` notification, then checks equality. Replace:

```rust
self.digest_watch.mark_unchanged();
if *self.digest_watch.borrow() == expected { return Ok(true); }
// then loop on changed()
```

With:

```rust
self.digest_watch.mark_unchanged();
loop {
    // Wait for a fresh signal first; only then check equality.
    self.digest_watch.changed().await.map_err(|_| AwaitErr::Closed)?;
    if *self.digest_watch.borrow() == expected { return Ok(true); }
    // continue loop, with a deadline check that returns Ok(false) on timeout
}
```

Adapt to the existing function's deadline/error shape — read it first.

- [ ] **Step 4: Run all convergence tests**

```bash
cargo test -p myrhiza-kernel --test convergence
```

Expected: all 8 still pass. **If any test now fails**, that test was previously vacuously passing — the fix has surfaced a real bug or test misdesign. Investigate per CLAUDE.md "a failing test is a question." Do not revert the harness fix.

- [ ] **Step 5: Commit**

```bash
git add crates/test-utils/src/harness.rs
git commit -m "fix(test-utils): await_digest waits for fresh signal before equality check (review Q-3)

Removes the pre-wait equality check that allowed vacuous passes on
stale state. The function now always awaits at least one changed()
notification before returning Ok(true).

Closes review-finding Q-3."
```

---

## Task 12: Eliminate silent Pending re-arm in drain loop

**Finding:** Q-2.

Pending-drain loop has a catch-all arm `AlreadyKnown | Pending(_) | Err(_) => {}` that silently drops a `Pending` outcome from the inner `dag.insert` call. Currently unreachable for honest input (since `newly_satisfied` filters on `is_subset(known)`), but this is a structural trap for any future maintainer who refactors `newly_satisfied`.

**Files:**
- Modify: `crates/kernel/src/runtime.rs`

- [ ] **Step 1: Locate the drain loop**

In `crates/kernel/src/runtime.rs` around lines 537–569.

- [ ] **Step 2: Split the catch-all**

Replace:

```rust
Inserted::AlreadyKnown | Inserted::Pending(_) | Err(_) => {}
```

With:

```rust
Inserted::AlreadyKnown => {
    // Event was re-promoted by a parallel path. No-op.
}
Inserted::Pending(still_missing) => {
    // newly_satisfied filtered on is_subset(known); reaching Pending here
    // implies a DAG/pending invariant drift. Re-buffer the event with the
    // current missing-set so it can be retried on the next round.
    self.pending.insert(event.clone(), still_missing, now);
    log_invariant_warning(format!(
        "pending-drain produced Pending(); re-buffered author={:?} seq={}",
        event.author, event.seq
    ));
}
Err(e) => {
    log_invariant_warning(format!("pending-drain insert error: {:?}", e));
}
```

(Adapt to the actual `Inserted` enum + error variants. The point is: no silent drop. `log_invariant_warning` may be a `tracing::warn!` or push to `peer_warnings` — pick what's consistent with the file.)

- [ ] **Step 3: Run all tests**

```bash
cargo test -p myrhiza-kernel
```

Expected: PASS — the path is currently unreachable, so changing the catch-all does not regress any test.

- [ ] **Step 4: Commit**

```bash
git add crates/kernel/src/runtime.rs
git commit -m "fix(kernel): pending-drain Pending/Err arms no longer silently drop (review Q-2)

Splits the catch-all in the drain loop so a Pending outcome re-buffers
rather than discards, and an Err logs an invariant-warning. Currently
unreachable for honest input but a structural trap closed."
```

---

## Task 13: dropped_at_apply map

**Finding:** M-4 (spec §4.4 + §14 edge-case 8).

Spec calls for a `dropped_at_apply` map so that events rejected at apply-time are tracked (for potential re-acceptance under a future state ordering, and for diagnostics). Currently `replay_full` silently drops rejected events.

**Files:**
- Modify: `crates/kernel/src/runtime.rs`

- [ ] **Step 1: Add the field**

In the `Runtime` struct definition, add:

```rust
dropped_at_apply: std::collections::HashMap<EventHash, String>,
```

Initialize to `HashMap::new()` in the constructor.

- [ ] **Step 2: Populate in replay_full**

In the `replay_full` loop, where the current code does `if let ApplyOutcome::Accepted = r.outcome { state = r.new_state; }`, add an `else` branch:

```rust
ApplyOutcome::Rejected { reason } => {
    self.dropped_at_apply.insert(event_hash(&e), reason.clone());
}
```

(Adapt to the actual `ApplyOutcome` variant name. If `reason` is a structured enum, format it with `{:?}` or pick a stable string repr.)

- [ ] **Step 3: Add a getter on RuntimeHandle**

```rust
impl RuntimeHandle {
    pub fn dropped_at_apply(&self) -> Vec<(EventHash, String)> {
        // Read the runtime's snapshot; the field lives in the run loop task,
        // so this needs an oneshot channel or a watch. Use the existing
        // pattern for peer_warnings/equivocation_log accessors.
        ...
    }
}
```

If no existing accessor pattern works (e.g. the runtime exposes state via `tokio::sync::watch` for digest but not for the warnings log), wire `dropped_at_apply` the same way you wired `equivocation_log` — pick the simplest consistent path. If neither exists, add a `command::Snapshot` AuthorCommand variant returning a `RuntimeSnapshot`. (Smaller scope: just expose this in the existing `peer_warnings` or similar list; the field's diagnostic value matters more than the access shape.)

- [ ] **Step 4: Add a test**

```rust
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn dropped_at_apply_records_rejected_events() {
    // Use the pre-check-rejector fixture (from plan A) so state-apply
    // returns Reject for every event. Author one event on peer A.
    // Assert that after publish, A.dropped_at_apply() contains one entry
    // for that event hash.
    ...
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p myrhiza-kernel
```

Expected: PASS including new test.

- [ ] **Step 6: Commit**

```bash
git add crates/kernel/src/runtime.rs crates/kernel/tests/convergence.rs
git commit -m "feat(kernel): track dropped_at_apply for events rejected by state-apply (review M-4)

Closes review-finding M-4. Implements spec §4.4 / §14 edge-case 8."
```

---

## Task 14: Explicit B-2 TODOs for replay_full + compute_anchor_digest

**Findings:** Q-1 + Q-7.

Both are documented in the PR description as B-2 deferrals, but the source code lacks an inline TODO anchoring the deferral to a specific carry-over plan. Without that, a future maintainer reading the code cold sees what looks like a perf bug, not a known deferred concern.

**Files:**
- Modify: `crates/kernel/src/runtime.rs`

- [ ] **Step 1: Add TODO above replay_full**

```rust
// B-2 carry-over: replay_full is O(N) per call and is called once per
// drain-round in handle_event. For DAGs > ~10k events the resulting
// O(N) work per insert dominates the select loop. Plan B-2 owns
// replacing this with an incremental apply driven by topo_index
// deltas; see docs/specs/2026-05-10-plan-b-1-dag-memnet-design.md
// "Documented follow-ups" §18.
fn replay_full(...) { ... }
```

- [ ] **Step 2: Add TODO above compute_anchor_digest**

```rust
// B-2 carry-over: compute_anchor_digest runs synchronously inside the
// biased select loop. For a large anchor subset it can starve incoming
// gossip. Plan B-2 owns moving this to a tokio::task::spawn_blocking or
// caching by anchor identity. See spec §18.
fn compute_anchor_digest(...) { ... }
```

- [ ] **Step 3: Commit**

```bash
git add crates/kernel/src/runtime.rs
git commit -m "docs(kernel): anchor B-2 TODOs for replay_full + compute_anchor_digest (review Q-1/Q-7)"
```

---

## Task 15: Pending-drain equivocation peer-identity carry-over note

**Finding:** Q-4.

`handle_event` drains pending into the DAG and on `DagError::Equivocation` logs with `peer: None`. Cannot be fixed in B-1 because the pending buffer does not carry the originating peer. This is a B-2 concern (peer identity wiring).

**Files:**
- Modify: `crates/kernel/src/runtime.rs`

- [ ] **Step 1: Add a code comment at the call site**

At `crates/kernel/src/runtime.rs:561` (or wherever the `peer: None` is currently set inside the drain loop), add:

```rust
// B-2 carry-over: pending events do not carry their originating peer
// identity, so equivocations surfaced during drain log peer=None. Plan
// B-2 extends PendingBuffer entries to record the source peer so this
// log is fully attributable. Closes review-finding Q-4.
```

- [ ] **Step 2: Commit**

```bash
git add crates/kernel/src/runtime.rs
git commit -m "docs(kernel): note B-2 carry-over for pending-drain equivocation peer-id (review Q-4)"
```

---

## Task 16: Style + comment cleanup

**Findings:** N-1, N-3, N-12, N-13.

Mechanical:
- N-1 (`runtime.rs` header comment): "Task 16 / Tasks 17-19" reference to plan tasks — delete or rewrite to describe the module's actual purpose.
- N-3 (`convergence.rs` "deterministically" comment): replace "Wait deterministically" with "Wait up to 5s" or similar honest framing of the timeout-based wait.
- N-12 (`handle_heads_summary` `#[allow(clippy::too_many_lines)]`): either refactor into named sub-functions (preferred) OR keep the allow + add a `// TODO(B-2): split into behind/equal/ahead/local sub-fns` note.
- N-13 (`build_genesis` test helper accepts but ignores `bundle_hash`/`topic_name`): either remove the unused parameters from the signature OR actually use them (e.g. `assert_eq!` on derivation match).

**Files:**
- Modify: `crates/kernel/src/runtime.rs`
- Modify: `crates/kernel/tests/convergence.rs`
- Modify: `crates/kernel/src/dag.rs`

- [ ] **Step 1: Apply each cleanup**

Edit each location. Smallest viable change per finding.

- [ ] **Step 2: Run lints + tests**

```bash
just ci
```

Expected: PASS, zero warnings.

- [ ] **Step 3: Commit**

```bash
git add crates/kernel/src/runtime.rs crates/kernel/tests/convergence.rs crates/kernel/src/dag.rs
git commit -m "chore(kernel): cleanup stale comments + unused test-helper args (review N-1/N-3/N-12/N-13)"
```

---

## Final gate

After Task 16:

- [ ] **Run full CI gate**

```bash
just ci
```

Expected: PASS, zero warnings, all 162+ tests green. New tests added by this plan (~5-7 depending on scope) should also pass.

- [ ] **Push to remote**

```bash
git push origin feat/plan-b-1-dag-memnet
```

- [ ] **Comment on PR #2**

Brief summary of the fixes; cite this plan path. Do not open a new PR — these fixes belong on the same B-1 branch.

---

## Self-review

**Spec coverage:** All 8 IMPORTANT findings (I-1, I-2, I-3, I-4, I-5, M-3, M-4 implicitly via M-4 + Q-3, M-8/M-9) have at least one dedicated task. All 5 quality IMPORTANT findings (Q-1, Q-2, Q-3, Q-4, Q-5, Q-7) covered. 3 MINOR (M-10, M-11 dedup with I-3, M-9 dedup with Task 9). 4 NIT covered by Task 16.

**Placeholder scan:** Two tasks (Task 4 step 1, Task 10 step 1, Task 11 step 1) intentionally start with `todo!()` as a TDD pin and require the *same task's* later step to replace the body. This is acceptable — the tasks explicitly say "do not commit with the todo." No other placeholders.

**Type consistency:** `PeerWarning::DriftRateLimited { kind }` (Task 5) is referenced in Task 14 / Task 16 contexts — naming consistent. `dropped_at_apply` (Task 13) is a `HashMap<EventHash, String>` — referenced once, consistent.

**Risk:** Tasks 4 and 11 are the highest-blast-radius — they change runtime protocol shape and harness semantics respectively. Both have TDD tests that would catch regression in the existing 8 acceptance tests.

**Sequence:** Tasks ordered so cheaper isolated fixes (1, 2, 3) land first, then protocol + semantics changes (4, 11, 12), then mechanical struct-shape fixes (5, 7), then sliding-window correctness (6), then new affordances (8, 9, 10, 13), then deferred-tracking docs (14, 15), then nit sweep (16). No task depends on a later task.

---

## Execution Handoff

Plan complete and saved to `docs/plans/2026-05-11-plan-b-1-fixes.md`. Two execution options:

1. **Subagent-Driven (recommended)** — fresh subagent per task, two-stage review between tasks, fast iteration on a clean branch state.
2. **Inline Execution** — execute tasks in this session using executing-plans, batch with checkpoints.

Which approach?
