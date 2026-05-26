**Date:** 2026-05-26
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/mvp.md §15.2](2026-05-09-myrhiza-master-design/mvp.md)
**Subject:** Plan B-6 — Poll app v1 (second MVP demo app)

# Plan B-6 — Poll app v1

## 1. Goal

B-6 ships the second MVP demo app called for in [mvp.md §15.2](2026-05-09-myrhiza-master-design/mvp.md) — a minimal poll application that exercises multi-author voting, a permission-gated event (`EndPoll`, creator-only), and the full four-component profile shape (state-apply + state-propose + interaction + manifest). It closes [implementation.md §20 item 16](2026-05-09-myrhiza-master-design/implementation.md) per the [post-B-4 gap analysis](../reports/2026-05-21-mvp-gap-analysis.md). Criterion 4 ("two apps coexist on one peer") is already met by B-5's counter + echo coexistence test, so B-6 is **not v1-blocking** — it is the second non-trivial demo for the v1 release showcase and exercises ABI surfaces that counter alone does not (permissioned events, structured per-option tallies, and the first fixture that tolerates non-empty `deps`). Per the gap analysis, B-6 is estimated at 2–3 days of focused work.

## 2. Scope

### 2.1 In v1 (this slice)

- Four-component poll bundle: `state-apply`, `state-propose`, `interaction`, `manifest`.
- State model + event vocabulary + permission gate for `EndPoll`.
- State-tier unit tests for the six canonical state-apply cases (§7.1).
- Kernel-tier in-process MemNetwork test exercising the full propose → pre-check → apply → re-project loop, plus a multi-author voting scenario.
- Coexistence extension: the existing B-5 `coexistence.rs::two_apps_coexist_no_event_crossing` test gains a poll-vs-counter variant (one peer running both bundles on different topics; no event crossing) per critical-ambiguity #6.
- `myrhiza-cli --bundle <poll-bundle>` works against the B-7 harness with one small harness contract addition: the harness populates `peer_state` with the local `AuthorPubkey` bytes per §4.1.4. No WIT changes, no new host imports.

### 2.2 Explicitly out of v1 (deferred)

- **`behavior` component** for "close poll automatically after a wall-clock timeout" — this is the v1.1 stretch precedent that mirrors counter's `auto-reset-at-midnight` (mvp.md §15.2). Deferred to v1.1 alongside acceptance criterion #6 per [mvp.md §15.1](2026-05-09-myrhiza-master-design/mvp.md).
- **Cross-process E2E test for the poll bundle.** Counter-cli's E2E test (B-7) already exercises the bundle-loader; poll uses the same harness with a different `--bundle`. A poll-specific cross-process test belongs in B-10 (real iroh distribution) alongside any other multi-bundle E2E work.
- **Multi-creator polls / nested polls.** A topic hosts exactly one poll (§4.1 critical-ambiguity #1); multi-poll-per-topic deferred indefinitely as not-currently-useful.
- **`AddOption` / `RemoveOption`.** Options are frozen at `CreatePoll` time. v1 vocabulary is exactly the three events mvp.md §15.2 lists.
- **Vote retraction (`Unvote`).** Re-voting (last-vote-wins) covers the common case; explicit unvote not needed for v1 (see critical-ambiguity #2).
- **Anonymous/sealed-sender voting.** Voter identity is the event's signing author — visible to anyone on the topic per Myrhiza's normal event-log semantics. Sealed-sender voting is a future module (per `prior-art/signal/` sealed-sender) and not in scope.
- **`examples/poll/` workspace member layout.** B-8 will move both apps into `examples/`. B-6 lands as `tests/fixtures/poll-*/` until B-8 ships (see §3.1 contingency).

## 3. Cross-slice coordination

B-8 (`examples/` + `crates/sdk/` workspace scaffolding) is happening in parallel and will land **before** B-6 at merge time. Two contingencies, picked at impl-start:

### 3.1 Path-A — B-8 has shipped (preferred)

The poll app lives at `examples/poll/` with the canonical mvp.md §15.4 layout:

```
examples/poll/
├── Cargo.toml
├── manifest.toml             # canonical TOML; signed-bincode lives under target/
└── src/
    ├── state.rs              # state-apply
    ├── propose.rs            # state-propose
    └── interaction.rs        # interaction
    (no behavior.rs — v1.1)
```

Source layout mirrors counter's once B-8 lands counter at `examples/counter/`. Dependency-direction CI (B-8) verifies `examples/poll` → `crates/sdk`, not the other way around.

### 3.2 Path-B — B-8 has not shipped (fallback)

The poll components live under `tests/fixtures/`, mirroring counter's pre-B-8 layout exactly:

```
tests/fixtures/
├── poll-state-apply/         # Cargo.toml + wit/ + src/lib.rs + .gitignore
├── poll-state-propose/
└── poll-interaction/
```

Each fixture is a `[workspace.exclude]`-listed sibling Cargo project (the established pattern from `counter-*-fixture` and `echo-state-apply-fixture`). `just build-fixtures` gains three new recipes (one per component). Test helpers land under `crates/test-utils/src/bundle.rs::build_signed_poll_bundle()` and `crates/kernel/tests/helpers/mod.rs::poll_handle()` mirroring `build_signed_counter_bundle` / `counter_handle`.

**Path-A is preferred**; Path-B is the no-blocking-on-B-8 fallback. The plan-writer phase resolves which path to take by checking whether B-8 has landed at impl-start. The technical content of the spec (§4 onward) is identical under both paths; only file layout differs.

## 4. Design

### 4.1 Critical-ambiguity resolutions

Six decisions called out by the task brief. Each lists the chosen answer + the rejected runner-up + why.

#### 4.1.1 One poll per topic (locked)

**Decision**: a topic instance hosts **exactly one poll**. The genesis event embeds the poll's options and creator; subsequent events (`Vote`, `EndPoll`) operate against that single poll.

**Runner-up**: multi-poll-per-topic, with `CreatePoll(poll_id, options)` events introducing new polls into a shared state space.

**Why**: Myrhiza topics are per-app-instance event streams per [convergence.md §4.6](2026-05-09-myrhiza-master-design/convergence.md). A topic's lifetime is the poll's lifetime; opening a new poll = opening a new topic instance (new `app_instance_seed`, new `topic_id`). Multi-poll-per-topic would force the state-apply to carry an unbounded `HashMap<poll_id, PollState>` whose pruning policy is its own design problem (when can old polls be GC'd? what happens if a Vote arrives for a poll the apply-side has pruned?). Topic churn is the simpler, deterministic answer and matches the per-app-instance shape every other use case in the spec assumes (per `prior-art/willow/state-machine.md` "Genesis event defines server-id" — same shape: genesis defines the topic; the topic is the poll). Multi-poll within a single topic is a future "poll-board" app, not v1's "minimal poll demo."

#### 4.1.2 Vote-replay semantics: last-vote-wins (locked)

**Decision**: a peer may re-vote any number of times before the poll ends. The latest `Vote` event (in topo-sort order) for that peer's `author_pubkey` is the canonical vote. Earlier votes from the same author are **overwritten in the materialized tally**, not stored as a history.

**Runner-up rejected**: (a) **reject-duplicate** ("first vote sticks; subsequent votes are `Verdict::Reject(\"already voted\")`"), (b) **idempotent / no-op on re-vote**, (c) **append-only "ballot history" preserving each cast vote**.

**Why**:

- **(a) reject-duplicate** runs into the topo-sort + concurrent-author-views problem: if two peers each vote, then peer A re-votes after seeing only its own first vote (its locally-known head), every concurrent re-vote becomes Reject for peers who later integrate a different topo order. Two peers that have seen the same set of events would still converge (state-apply is pure), but UX is "your vote may flip from Accept to Reject depending on whose view you replay" — confusing and arbitrary.
- **(b) idempotent / no-op** silently discards the user's intent to change vote, which is the legitimate use case ("I picked B but meant A").
- **(c) ballot history** is reasonable but balloons state-size, requires a separate "current effective vote" projection, and adds nothing the simple overwrite doesn't.

**Last-vote-wins is deterministic**: state-apply is `(prior, event) -> (verdict, new_state)`; the natural implementation is `votes.insert(author_pubkey, option_index)`. Determinism is preserved because topo-sort order is canonical — per [`prior-art/willow/state-machine.md`](../prior-art/willow/state-machine.md) §"Convergence property" leg 2: "Topo-sort is deterministic (Kahn's algorithm + `BTreeSet` lex order)." Two peers that have seen the same event set apply them in the same canonical order and arrive at the same `votes` map. The "vote" the author intended is the last `Vote` event from that author in canonical order — well-defined and inspectable.

The state model uses `BTreeMap<AuthorPubkey, OptionIndex>` (BTreeMap, not HashMap) to make the on-wire state-digest stable across runs without sorting separately (per [determinism.md §5.1](2026-05-09-myrhiza-master-design/determinism.md) deterministic helper set discipline). `HashMap` iteration order is allocator-dependent and would invalidate `state-digest` cross-peer; BTreeMap iteration is sorted-by-key-bytes by construction. The "HashMap is not deterministic" claim is normative in [convergence.md §4.3](2026-05-09-myrhiza-master-design/convergence.md) (line 100, "Why not hash WASM linear memory" sub-bullet: "allocator behavior, struct field padding, `HashMap` iteration order would diverge trivially across peers").

#### 4.1.3 EndPoll permission gate — creator embedded in CreatePoll payload (locked)

**Decision**: at genesis, `state-apply` reads the **`founder_pubkey` field of the `GenesisV1` envelope** and stores it as `state.creator`. On every subsequent `EndPoll` event, state-apply reads `event.author` from the canonical envelope and rejects if `event.author != state.creator`. The kernel enforces `event.author == GenesisV1.founder_pubkey` for the genesis event itself (per [convergence.md §4.6](2026-05-09-myrhiza-master-design/convergence.md)), so reading `founder_pubkey` and `event.author` from the envelope yields the same byte-string at genesis time — picking `founder_pubkey` matches the counter fixture's pattern (`tests/fixtures/counter-state-apply/src/lib.rs:200-211`) and avoids re-parsing the outer envelope's author field on genesis.

**Runner-up rejected**: **(implicit) creator = author of the first event in topo-sort.**

**Why**:

The runner-up looks attractive because the genesis author is structurally the topic creator (per [convergence.md §4.6](2026-05-09-myrhiza-master-design/convergence.md)). But:

1. **What "state-apply sees" is materialized state, not the topo-sort history.** The kernel hands `state-apply` `(prior_state, event_envelope)`. `state-apply` does not see "the list of all prior events" — it sees the snapshot. To find the creator, state-apply must have already stored that information in `prior_state`. Embedding it explicitly at genesis is the only path that keeps state-apply pure (no host calls into the event log).
2. **The genesis envelope's `founder_pubkey` is already canonical.** State-apply already decodes the `GenesisV1` envelope at genesis to extract `app_payload`; reading `founder_pubkey` from the same envelope adds no new decode work. No new host import required.
3. **Trusting unsigned data is the trapdoor to avoid.** The kernel verifies the genesis envelope (B-1 chain integrity + Ed25519 signature; equality `event.author == GenesisV1.founder_pubkey` enforced at validation time) *before* state-apply runs. State-apply trusts the kernel's verification — it does not re-verify signatures (per [determinism.md §5.1](2026-05-09-myrhiza-master-design/determinism.md) — crypto primitives are kernel-side host imports, not part of state-apply's hot path). The kernel signs the envelope-author binding; state-apply reads `founder_pubkey` at genesis and `event.author` on subsequent events; comparison is trustworthy.

**Trace through (state-apply call)**:

1. Kernel receives `EndPoll` event from gossip / direct stream.
2. Kernel verifies Ed25519 signature over canonical body (matches `event.author`). If invalid → `PeerWarning::SignatureInvalid` per B-4.8; state-apply is never called.
3. Kernel inserts into per-author DAG (chain-integrity check); topo-sort places it.
4. Kernel calls `state-apply.apply(prior_state, event_bytes)`:
   - state-apply decodes `event.author` from canonical envelope at offset 8 (per the offset constants in `tests/fixtures/counter-state-apply/src/lib.rs:137-147` — `author` is the first field, length-prefixed 32-byte pubkey).
   - state-apply decodes `event.payload` and finds it's an `EndPoll` (single-byte discriminator + no further data).
   - state-apply reads `prior_state.creator: [u8; 32]` (set from `founder_pubkey` at genesis-apply).
   - If `event.author != prior_state.creator` → `Verdict::Reject("EndPoll: not poll creator")`.
   - Else → `Verdict::Accept`; sets `new_state.ended = true`.

The creator pubkey lives in materialized state as a 32-byte field, included in every `state-digest()` output. Cross-peer convergence relies on this — peers that disagree on who created the poll cannot converge.

#### 4.1.4 Interaction view shape — all four projections (locked)

**Decision**: `view(state, peer_state) -> bytes` projects a UTF-8 text block containing:

```
poll: in-progress
options:
  [0] Yes              (3 votes)
  [1] No               (1 vote)
  [2] Abstain          (0 votes)
your vote: 1 (No)      # absent if you have not voted; "<not voted>" otherwise
```

When the poll has ended, the first line reads `poll: ended` instead. The `poll: <in-progress | ended>` line is a literal status indicator — it is **not** a placeholder for runtime data. `PollState` carries no prompt field (per mvp.md §15.2 master design); poll context (the question being asked) is surfaced out-of-band by the originating user (URL, chat, etc.). See §4.2 for the data-model rationale and v1.1 candidate flag.

All four projections in one view: (a) live counts per option, (b) ended/in-progress flag (rendered as the literal status line), (c) per-peer "your vote" display, (d) plus the option labels themselves (which were not in the brief's enumeration but are necessary to make the view legible).

`peer_state` is a 32-byte slice containing the local `AuthorPubkey` of the interacting user (whose vote is "yours"). This is the first non-empty use of `peer_state` in Myrhiza — counter ignored it (per B-7 Choice D, which defined `peer_state` as opaque app-defined bytes, always empty for counter v1). It is **read-only** in v1 (mirrors B-7 §6 resolved decision: harness owns peer_state mutation). Pluralize the vote-count noun via a simple `if n == 1 { "vote" } else { "votes" }` — no i18n machinery in v1.

**Harness contract addition** (normative — owned by this spec):

B-6 introduces a harness ABI contract addition that B-7 Choice D left as a placeholder. The B-7 native interaction harness MUST populate `peer_state` with the local `AuthorPubkey` bytes (32 bytes, the raw pubkey of the keypair the harness uses to sign events) for every `view` call when running poll bundles — or, more precisely, for every bundle whose `peer_state` shape is non-empty. The harness already holds the local keypair (for signing events), so passing the pubkey bytes through into `view` is a small, deterministic-helper-set-compatible addition (no new host imports, no new WIT changes). This is a real contract change; the plan-writer phase implements it, but the contract itself is owned here.

**Runner-up rejected**: subset views (counts only; counts + ended flag without "your vote") — every projection adds discrete testable behavior; cutting any of them reduces the demo's pedagogical value. The interaction component is non-deterministic-OK per [architecture.md §3.1](2026-05-09-myrhiza-master-design/architecture.md), so per-peer divergence in the rendered text is fine.

**Why testable with the B-7 harness**: the harness drives `view → stdin → dispatch → propose → pre-check → apply` (per [B-7 design §3.6](2026-05-21-plan-b-7-interaction-harness-design.md)). The test driver writes scripted stdin (e.g., `"vote 1\nquit\n"`) and asserts the **final view bytes contain expected substrings** (`"option 1.*1 vote"`, `"your vote: 1"`, `"status: in-progress"`). The harness's existing stdout-capture flow is exactly the surface needed; only the `peer_state` plumbing above is new.

#### 4.1.5 State-tier coverage — six canonical cases (locked)

State-tier unit tests are pure invocations of `state-apply.apply(prior, event)` with crafted envelopes — no kernel, no DAG. The six required cases per the task brief, each as its own `#[test]` fn:

| # | Test name | Setup | Event | Expected verdict | Expected new state |
|---|---|---|---|---|---|
| 1 | `genesis_create_poll_accepts` | empty `prior_state` | `CreatePoll{options=["Yes","No"], creator=alice_pk}` (in genesis envelope, founder=alice, seq=1) | `Accept` | `{creator=alice_pk, options=["Yes","No"], votes={}, ended=false}` |
| 2 | `vote_records_choice` | state with `creator=alice, options=["Yes","No"]`, votes empty | `Vote{option_index=0}` signed by bob | `Accept` | votes = `{bob → 0}`, ended=false |
| 3 | `vote_replay_overwrites_prior_choice` | state with `votes={bob → 0}` | `Vote{option_index=1}` signed by bob | `Accept` | votes = `{bob → 1}` (overwrite — per §4.1.2) |
| 4 | `end_poll_by_creator_accepts` | state with `creator=alice` | `EndPoll` signed by alice | `Accept` | `ended=true` |
| 5 | `end_poll_by_non_creator_rejects` | state with `creator=alice` | `EndPoll` signed by bob | `Reject("EndPoll: not poll creator")` | unchanged |
| 6 | `vote_after_end_poll_rejects` | state with `ended=true` | `Vote{option_index=0}` signed by carol | `Reject("Vote: poll has ended")` | unchanged |
| 6b | `vote_replay_out_of_order_converges_to_lex_last` | two `Vote` events from the same author applied in either order against the same prior state | both orderings `Accept`; final `votes[author]` is the lex-last `(event-hash, option_index)` pair under topo-sort tie-break (per §4.1.2) | converges to canonical order regardless of arrival sequence |

Plus four structural-validity cases not in the brief but required for completeness (state-apply is the only check on these — the propose path can be bypassed by a hand-crafted signed event):

| # | Test name | Event | Expected verdict |
|---|---|---|---|
| 7 | `vote_out_of_range_option_rejects` | `Vote{option_index=5}` against 2-option poll | `Reject("Vote: option_index out of range")` |
| 8 | `non_genesis_create_poll_rejects` | `CreatePoll` event with seq=2 against initialized state | `Reject("CreatePoll: only valid as genesis")` |
| 9 | `genesis_zero_options_rejects` | `CreatePoll{options=[]}` in genesis envelope | `Reject("CreatePoll: must declare ≥1 option")` |
| 10 | `genesis_too_many_options_rejects` | `CreatePoll{options=[...]}` with `options.len() > MAX_OPTIONS` in genesis envelope | `Reject("CreatePoll: must declare 1..=MAX_OPTIONS")` |

All state-tier tests (cases 1–6, 6b, and 7–10) run in `crates/kernel/tests/poll_state_apply.rs` (new file) by instantiating the poll state-apply WASM fixture via the existing `WasmtimeBackend::instantiate_state_apply` + `StateApplyHandle` pattern (mirrors counter's pattern from `crates/kernel/tests/acceptance.rs::kernel_instantiates_and_applies_increment`). State-tier tests do NOT instantiate a `Runtime` — they call `apply()` directly with hand-built envelopes constructed via `myrhiza_kernel::event_builder::EventBuilder` (re-homed in B-7.0).

**Kernel-tier coverage** (in-process MemNetwork, single peer, no real iroh):

| # | Test name | What it verifies |
|---|---|---|
| K1 | `poll_e2e_single_peer_full_lifecycle` | end-to-end propose → pre-check → apply for: CreatePoll genesis → Vote → ReVote → EndPoll. Asserts final state via `state-digest()` matches the expected canonical bytes. |
| K2 | `poll_multi_author_voting` | three authors (alice creator, bob + carol voters) on the **same in-process bus**; all three see the same final tally; `state-digest` agrees. Voters MUST declare `deps = {creator's genesis event hash}` (not empty) so the non-empty-deps code path in `state-apply` is genuinely exercised — the first fixture to do so. This exercises Myrhiza's multi-author DAG (per `prior-art/willow/state-machine.md` "per-author Merkle DAG") in a way counter (which has been multi-author tested, but only with empty deps) does not. |
| K3 | `poll_unauthorized_end_poll_rejected_by_authority` | bob attempts EndPoll on alice's poll; kernel calls apply; verdict is Reject; bob's event is dropped at apply (per `RuntimeHandle::dropped_at_apply` already exposed). |
| K4 | `poll_and_counter_coexist_no_event_crossing` | extension of B-5's `two_apps_coexist_no_event_crossing`. One peer, two `Runtime`s on two topics — counter on one, poll on the other. Author increment on counter; author vote on poll; assert each digest changes independently and the other runtime's digest does not. Closes critical-ambiguity #6. |

K1–K3 live in `crates/kernel/tests/poll.rs` (new file); K4 lives as a new function in the existing `crates/kernel/tests/coexistence.rs`.

#### 4.1.6 Coexistence with counter (locked)

**Decision**: yes — coexistence is demonstrated by adding test K4 above to the existing `coexistence.rs` test file, alongside `two_apps_coexist_no_event_crossing`. The new test is structurally identical to the B-5 test but substitutes the echo bundle for poll; counter stays as the other app. Same peer, different topics, different bundles — assertions on independent digest progression + no `dropped_at_apply` cross-pollution + no `peer_warnings::SignatureInvalid` from one runtime's events arriving on the other's bus subscription.

**Why**: B-5's coexistence test already proved counter + echo coexist. Adding poll + counter is a single ~50-line test that closes the question concretely. The marginal cost is low and the test gives B-6 a concrete coexistence proof-point without requiring a new mechanism. The echo-vs-poll case stays implicit (transitively: both apps coexist with counter ⇒ both apps share the same isolation guarantees; no kernel changes between the two cases).

### 4.2 Data model

Poll state, stored as the materialized state bytes returned by `state-apply.apply` and consumed by `state-digest`:

```rust
// Conceptual shape; on-wire is canonical bincode of this exact field
// order. Compatible with state-apply's WIT contract: state is
// `list<u8>` (opaque to kernel; canonical bytes the kernel passes
// verbatim back into apply on the next event).
struct PollState {
    // The author who signed the CreatePoll genesis event. Read on
    // every EndPoll to gate the permission check (§4.1.3).
    creator: [u8; 32],

    // Option labels, declared at CreatePoll, frozen for the lifetime
    // of the poll. Bounded to MAX_OPTIONS = 16 at genesis-validation
    // time to keep state-size deterministic.
    options: Vec<String>,

    // Per-author current vote. BTreeMap, not HashMap — iteration
    // order is sorted-by-key-bytes, which makes the canonical-bincode
    // encoding stable across peers (determinism critical, §4.1.2).
    votes: BTreeMap<[u8; 32], u32>,  // AuthorPubkey -> option index

    // Set true by EndPoll; once true, Vote events are rejected.
    ended: bool,
}
```

`state-digest(state_bytes) -> state_bytes`: the canonical-bincode encoding of `PollState` *is* its own digest, mirroring counter's and echo's "identity digest" pattern (per `tests/fixtures/counter-state-apply/src/lib.rs:239` + `tests/fixtures/echo-state-apply/src/lib.rs:217`). BTreeMap canonical iteration makes this safe; if we used HashMap the digest would diverge across peers and the convergence proof would break.

**No prompt field — design note.** Per [mvp.md §15.2](2026-05-09-myrhiza-master-design/mvp.md), `PollState` carries no prompt field; the view does not render poll text. The originating user surfaces poll context out-of-band (URL, chat, app-level metadata). v1.1 candidate: an optional `prompt: String` field at `CreatePoll` genesis if user research surfaces the need — the encoder, view, and dispatch would each gain a small touch-up. Not in scope for v1.

**State-size bound**: `MAX_OPTIONS = 16`, `MAX_OPTION_LABEL_LEN_BYTES = 64`, `MAX_VOTES` unbounded (one per distinct author pubkey on the topic). At 16 options × 64 bytes + a votes map of 1000 voters × (32 + 4) bytes ≈ 36 KB of state. Below the 64 KB heap cap the WASM fixtures' bump allocator declares.

### 4.3 Event vocabulary

Three event variants, app-payload-discriminated. The kernel only sees `payload: Vec<u8>` (per [convergence.md §4](2026-05-09-myrhiza-master-design/convergence.md) — payload is opaque bytes). Discriminator byte + body:

```
CreatePoll  discriminator = 0x00
  body = canonical-bincode of:
    { options: Vec<String> }     // creator is event.author, not in payload
  validity: only as genesis event (seq=1 founder with empty prior_state).
            Wrapped by the kernel's GenesisV1 envelope per §4.4.

Vote        discriminator = 0x01
  body = u32 BE (option_index)
  validity: state.ended must be false; option_index < state.options.len()

EndPoll     discriminator = 0x02
  body = empty (zero bytes after discriminator)
  validity: event.author must equal state.creator
```

Wire layout for non-genesis events (`Vote`, `EndPoll`):

```
payload[0]   = discriminator (0x01 or 0x02)
payload[1..] = body bytes
```

Genesis events wrap the `CreatePoll` body inside Myrhiza's `GenesisV1` envelope per [convergence.md §4.6](2026-05-09-myrhiza-master-design/convergence.md): `{seed: [u8; 32], founder_pubkey: AuthorPubkey, app_payload: bytes}`. The `app_payload` is the `0x00 ‖ bincode(options)` byte string. `state-apply`'s genesis discriminator (`seq == 1 && prior_state.is_empty()`) follows counter's exact pattern from `tests/fixtures/counter-state-apply/src/lib.rs:191`; we mirror that pattern verbatim.

### 4.4 Permission gate: EndPoll

The single permission gate. Full trace already given in §4.1.3. Summary:

- Kernel's job: verify signature, chain integrity, place in topo-sort.
- State-apply's job: read `event.author` from the canonical envelope (using the same byte-offset decode counter does); read `state.creator`; compare; emit Reject if mismatched.

`state-propose` may ALSO perform this check before signing (defense-in-depth) — see §4.5.2. State-apply is the load-bearing layer; propose is a courtesy check whose only purpose is to surface "you cannot end a poll you did not create" as an error on the originating peer before the event ever leaves the machine. The kernel pre-check (`state-apply` in dry-run) is the canonical answer; if propose's courtesy check disagrees, pre-check overrules.

### 4.5 The four components

Source-layout note: per §3.1/§3.2 the components live in either `examples/poll/src/{state,propose,interaction}.rs` (Path-A) or `tests/fixtures/poll-*/src/lib.rs` (Path-B). The shape and content of each component is path-independent.

#### 4.5.1 `state-apply` (~120 LOC; bump-allocator + hand-rolled bincode decoder, mirrors counter/echo)

Exports `apply(prior_state: list<u8>, event: list<u8>) -> tuple<verdict, list<u8>>` and `state-digest(state: list<u8>) -> list<u8>` per [world-state-apply.wit](../wit/myrhiza-kernel/wit/world-state-apply.wit).

Pseudocode (full implementation follows counter's pattern):

```
fn apply(prior_state, event_bytes):
    # Decode envelope; mirrors counter's byte-offset decoder
    parse author at envelope[0..40]   # serde_bytes len(8) + 32 bytes
    parse seq at envelope[40..48]
    parse payload at envelope[PAYLOAD_OFFSET..]

    if seq == 1 && prior_state.is_empty():
        # Genesis: must be CreatePoll
        wrapped = decode GenesisV1 from payload
        body = wrapped.app_payload
        if body[0] != 0x00:
            return Reject("genesis must be CreatePoll discriminator")
        options = decode Vec<String> from body[1..]
        if options.is_empty() || options.len() > MAX_OPTIONS:
            return Reject("CreatePoll: must declare 1..=MAX_OPTIONS")
        for label in options:
            if label.len() > MAX_OPTION_LABEL_LEN_BYTES:
                return Reject("CreatePoll: option label too long")
        # creator = founder_pubkey from GenesisV1 envelope
        state = PollState{
            creator: wrapped.founder_pubkey,
            options,
            votes: BTreeMap::new(),
            ended: false,
        }
        return Accept(canonical_bincode(state))

    # Non-genesis path
    state = decode PollState from prior_state
    if payload.is_empty():
        return Reject("event payload empty")

    match payload[0]:
        0x00 (CreatePoll non-genesis):
            return Reject("CreatePoll: only valid as genesis")

        0x01 (Vote):
            if state.ended:
                return Reject("Vote: poll has ended")
            if payload.len() != 5:    # discriminator + u32 BE
                return Reject("Vote: malformed body")
            option_index = u32 BE from payload[1..5]
            if option_index >= state.options.len():
                return Reject("Vote: option_index out of range")
            state.votes.insert(author, option_index)  # last-vote-wins
            return Accept(canonical_bincode(state))

        0x02 (EndPoll):
            if event.author != state.creator:
                return Reject("EndPoll: not poll creator")
            if payload.len() != 1:    # discriminator only
                return Reject("EndPoll: malformed body")
            state.ended = true
            return Accept(canonical_bincode(state))

        _:
            return Reject("unknown discriminator")

fn state_digest(state_bytes):
    state_bytes    # identity; canonical bincode of PollState is already stable
```

Like counter, this is `#![no_std]` cdylib with a bump-allocator and hand-rolled byte-offset decoding to avoid pulling float-Display paths through serde_core (per the comment block at `tests/fixtures/counter-state-apply/src/lib.rs:91-130` — same constraint).

Encoding-discipline call-out: the spec talks abstractly about `decode Vec<String>` and `canonical_bincode(state)`. Because the fixture is `no_std` + serde-free (to satisfy the float-ban lint), the actual implementation hand-rolls a compact canonical encoding for `Vec<String>` and `PollState` — length-prefixed u64-BE strings, BTreeMap iterated in key order, no float types anywhere. The wire format is locked by a state-tier golden-bytes test (test #1 above asserts the canonical-bincode bytes of the initial `PollState`). This is the same approach counter uses for its 8-byte i64 state — but the poll state is non-trivially structured, so the encoder is ~30 LOC.

#### 4.5.2 `state-propose` (~60 LOC)

Exports `propose(prior_state: list<u8>, intent: list<u8>) -> result<list<u8>, string>` per [world-state-propose.wit](../wit/myrhiza-kernel/wit/world-state-propose.wit).

App-internal intent vocabulary (opaque to kernel; matches counter's discriminator-byte pattern):

```
intent[0] = 0x00  CreatePoll
  intent[1..]  = bincode-encoded Vec<String> of options
intent[0] = 0x01  Vote
  intent[1..5] = u32 BE option_index
intent[0] = 0x02  EndPoll
  intent[1..]  = (empty)
```

Propose's job is to validate the intent against `prior_state` (defense-in-depth: pre-check via state-apply is the load-bearing check, but propose's reject surfaces the error before the event is signed):

```
fn propose(prior_state, intent):
    if intent.is_empty():
        return Err("intent must declare an event kind")

    match intent[0]:
        0x00 CreatePoll:
            if !prior_state.is_empty():
                return Err("CreatePoll: only valid as genesis intent")
            options = decode Vec<String> from intent[1..]
            if options.is_empty():
                return Err("CreatePoll: must declare ≥1 option")
            if options.len() > MAX_OPTIONS:
                return Err("CreatePoll: too many options (> MAX_OPTIONS)")
            # Pass through; kernel will wrap in GenesisV1 + add the seed.
            # Body bytes are returned as event-payload.
            return Ok(intent)   # 0x00 + bincode(options)

        0x01 Vote:
            state = decode PollState from prior_state
            if state.ended:
                return Err("Vote: poll has ended")
            option_index = u32 BE from intent[1..5]
            if option_index >= state.options.len():
                return Err("Vote: option_index out of range")
            return Ok(intent)   # 0x01 + u32 BE

        0x02 EndPoll:
            # Note: propose cannot see "is the local peer the creator?"
            # because it doesn't know the local author's pubkey. We
            # cannot perform the creator-only check here; that's
            # state-apply's job (after the event is signed by the
            # author and the kernel pre-checks via state-apply dry-run).
            # Propose just emits the bare discriminator; pre-check
            # will reject if the local author isn't the creator.
            state = decode PollState from prior_state
            if state.ended:
                return Err("EndPoll: poll already ended")
            return Ok(intent)   # 0x02

        _: return Err("unknown intent discriminator")
```

The asymmetry on `EndPoll`'s permission check (propose can't see "is the local author the creator?") is by design: the state-propose WIT world has no host-import that exposes the local `AuthorPubkey`. State-apply, by contrast, sees `event.author` because the kernel passes the signed envelope. This is the intended layering: propose is per-peer best-effort validation; state-apply is the canonical authority verdict per [architecture.md §3.5](2026-05-09-myrhiza-master-design/architecture.md). Pre-check (state-apply in dry-run) catches the "wrong author" case before the event is broadcast — see §4.1.3 trace.

> **Future ABI gap candidate** (out of B-6 scope, do not implement here): "propose can't see local author" is going to bite every permission-gated app — not just poll's `EndPoll`. A future host-import (e.g. `host.local_author() -> AuthorPubkey`) would let propose surface "you can't do that" errors earlier and avoid the round-trip through pre-check for known-impossible intents. Flagged as a candidate for a future ABI-additions spec; not part of B-6.

#### 4.5.3 `interaction` (~150 LOC)

Exports `view(state, peer_state) -> list<u8>`, `dispatch(action: string) -> result<list<u8>, string>`, plus the two completion-handler stubs per [world-interaction.wit](../wit/myrhiza-kernel/wit/world-interaction.wit).

**`view(state, peer_state)`**:

- Decode `PollState` from `state`.
- Decode local-author `[u8; 32]` from `peer_state` if `peer_state.len() == 32`, else treat as "you".
- Compute per-option vote counts by iterating `state.votes` and bucketing by value.
- Build the UTF-8 text block per §4.1.4 layout.

Float-free; uses the same i64-to-decimal helper pattern counter uses for vote-count digit formatting (per `tests/fixtures/counter-interaction/src/lib.rs:102-156`).

**`dispatch(action)`** — actions accepted:

```
"create <opt1> <opt2> ..."     create-poll with the given options (genesis)
"vote <option_index>"          vote for option N
"end"                          end the poll
```

Parses the action and emits the corresponding intent byte string (matching §4.5.2's vocabulary). Rejects unknown actions with an `Err(...)`.

Whitespace-tokenized for v1 (matches counter-interaction's parser style). Option labels in `create` cannot contain spaces in v1; quoted strings can land in B-8 polish.

**Completion handlers**: no-op stubs (mirrors counter's pattern at `tests/fixtures/counter-interaction/src/lib.rs:224-232`).

#### 4.5.4 `manifest.toml`

Declares the four components and the (zero-non-deterministic) host import surface:

```toml
[bundle]
name = "poll"
version = "0.1.0"

[components]
state_apply  = "components/state-apply.wasm"
state_propose = "components/state-propose.wasm"
interaction   = "components/interaction.wasm"
# no behavior component in v1 (per §2.2 deferral)

[capabilities]
# Counter's exact shape: helpers-only, no non-deterministic imports.
# state-apply manifest binds host.log + host.hash deterministic
# imports per the existing helpers_only_state_apply_manifest pattern
# (`crates/test-utils/src/manifest.rs`).
state_apply = ["host.log", "host.hash"]
state_propose = ["host.log"]
interaction = ["host.log"]
```

The manifest signs (per B-7.0's bundle-content-hash) over the BLAKE3 composite hash of all three components. The test-utils helper `helpers_only_three_component_manifest()` (already added in B-7.0) is reused for poll's signing flow.

## 5. Test infrastructure

### 5.1 State-tier (`crates/kernel/tests/poll_state_apply.rs` — new file)

One `#[test]` function per row in the §4.1.5 tables (cases 1–6, 6b, and 7–10). Each:

1. Instantiates the poll-state-apply WASM via `WasmtimeBackend::instantiate_state_apply` + `StateApplyHandle::new` (mirrors the pattern from `crates/kernel/tests/acceptance.rs::kernel_instantiates_and_applies_increment`).
2. Builds the test event via `myrhiza_kernel::event_builder::EventBuilder` (re-homed in B-7.0; pre-B-7 use `myrhiza_test_utils::EventBuilder`).
3. Calls `handle.apply(prior_state_bytes, event_bytes).await` (the handle's `apply` is sync wrt the WASM call but the wrapping is async per `StateApplyHandle`'s contract).
4. Asserts the verdict variant + (on Accept) the canonical-bincode bytes of the resulting `PollState`.

These tests run on every `cargo test -p myrhiza-kernel` and exercise only state-apply — no kernel runtime, no MemBus, no DAG. Cost ~milliseconds.

### 5.2 Kernel-tier (`crates/kernel/tests/poll.rs` — new file)

Tests K1–K3 per §4.1.5 table. Each spins up an in-process `Runtime` with a `MemNetwork`, drives events through the full propose → pre-check → apply path, observes via `RuntimeHandle::digest_watch` + `RuntimeHandle::dropped_at_apply`. Asserts:

- K1: final state-digest equals expected canonical bytes after 4 events (CreatePoll → Vote → ReVote → EndPoll).
- K2: after seeding three authors and authoring votes from each (voters' Vote events declare `deps = {creator's genesis event hash}`, exercising the non-empty-deps code path), the digest converges to the expected three-author tally.
- K3: bob's `EndPoll` on alice's poll appears in `dropped_at_apply` with verdict `Reject("EndPoll: not poll creator")`.

### 5.3 Coexistence (`crates/kernel/tests/coexistence.rs` — extend existing)

Add `poll_and_counter_coexist_no_event_crossing` per §4.1.6. Same shape as the existing `two_apps_coexist_no_event_crossing` but with poll's bundle in place of echo's. Asserts:

- Both runtimes' digests progress independently.
- Neither runtime's `dropped_at_apply` registers the other's events.
- Neither runtime's `peer_warnings` contains `SignatureInvalid` (would indicate cross-topic event leakage).

### 5.4 Skipped tiers

- **E2E (real iroh, cross-process)**: out of scope per §2.2. Counter's E2E in B-7 already proves the bundle-loader path; poll uses the same path with no plumbing changes.
- **Browser tier**: requires jco backend; out of v1 entirely per [mvp.md §15.5](2026-05-09-myrhiza-master-design/mvp.md).

## 6. Risks

### 6.1 Determinism trapdoors specific to multi-author voting

**Risk**: `votes` map ordering. If the canonical-bincode encoding of `BTreeMap` iterates in non-deterministic order across two peers, two peers with the same vote set produce different `state-digest` bytes → drift detection triggers a TUTTI-shaped warning (per [convergence.md §4.7](2026-05-09-myrhiza-master-design/convergence.md)).

**Mitigation**: BTreeMap iteration is sorted-by-key-bytes by Rust's std-lib contract — this is a load-bearing property and is exercised by the multi-author kernel-tier test K2 (which authors votes from three deterministic keypairs and asserts state-digest converges across all three runtime views). If this assertion fails the bug is in our encoder; the test is the canary.

**Runner-up considered**: `HashMap` with explicit pre-encode sort. Rejected: BTreeMap is structurally deterministic; explicit sort is an extra step that's easy to miss in a future "I'll refactor this" PR.

### 6.2 Genesis founder vs CreatePoll author confusion

**Risk**: state-apply must decide who the "creator" is when CreatePoll lands. The `GenesisV1` envelope carries `founder_pubkey` per [convergence.md §4.6](2026-05-09-myrhiza-master-design/convergence.md), and the event's outer `author` field is also present. Are they the same? Should state-apply trust the inner or the outer?

**Resolution**: per [convergence.md §4.6](2026-05-09-myrhiza-master-design/convergence.md) the kernel enforces `event.author == GenesisV1.founder_pubkey` for genesis events. State-apply reads `founder_pubkey` and stores it as `state.creator`. Subsequent `EndPoll` checks compare `event.author` (outer) against `state.creator` (inner from the long-ago genesis). The two are reconciled at kernel-validation time, not in state-apply.

**Mitigation**: state-tier test 1 (`genesis_create_poll_accepts`) asserts the stored `state.creator` matches the genesis envelope's `founder_pubkey`. The kernel-tier check that `event.author == founder_pubkey` lives in the existing B-1 chain-integrity rule; we rely on it.

### 6.3 Novel-surface gotchas — first fixture that tolerates non-empty deps

**Risk**: counter has already been exercised under multi-author authoring (per `crates/kernel/tests/convergence.rs::concurrent_multi_author_converges` — peer A + peer B both author increments after genesis). What counter has NOT been exercised on is **non-empty `deps`**: both the counter and echo fixtures hard-reject `deps_len != 0` (per `tests/fixtures/counter-state-apply/src/lib.rs:174-177` and `tests/fixtures/echo-state-apply/src/lib.rs:169-171`), and existing tests pass `BTreeSet::new()` for deps. Poll is the **first fixture that tolerates non-empty `deps`** — voters' Vote events declare `deps = {creator's genesis event hash}` so their per-author chains hang off the topic's shared causal anchor.

**Resolution**: poll's state-apply must **tolerate non-empty deps**. State-apply does not enforce DAG topology — that's the kernel's job (B-1 topo-sort + chain-integrity). State-apply just reads `event.author` and `event.payload`; deps are part of the envelope but state-apply does not need to inspect them. The byte-offset decoder skips past the deps array (8-byte length prefix + N × 40 bytes) to find the payload.

**Concrete change** vs the counter fixture: drop the `deps_len != 0` reject (counter-state-apply:174-177); skip past deps to PAYLOAD_LEN_OFFSET dynamically rather than at a fixed compile-time offset. This is a ~15-LOC change to the offset-decoder vs. counter's compile-time-constant offsets. Tested by K2 (multi-author with non-empty deps): voters MUST declare `deps = {creator's genesis event hash}` (not empty) so the non-empty-deps code path is genuinely exercised.

### 6.4 Vote-replay overwrites earlier votes — UX gotcha

**Risk**: a peer that votes A, then re-votes B, has their A discarded. If the UI doesn't make this clear, users will think they double-voted.

**Mitigation**: the view (§4.1.4) shows "your vote: 1 (No)" — only the current vote, not the history. A future UI module could surface "you changed your vote from Yes to No 30 seconds ago," but that's a higher-tier UX concern not blocking v1.

### 6.4b Vote-replay transient-tally during ingestion (deps-monotonicity)

**Risk**: per [convergence.md §4.4](2026-05-09-myrhiza-master-design/convergence.md) deps-monotonicity, state-apply must be valid against *any* state containing the event's declared `deps`. With last-vote-wins (§4.1.2), if peer A signs Vote(0) and then — before that propagates — signs Vote(1), a receiver materializing the events in a different topo order can see Vote(1) apply before Vote(0) arrives. Vote(0) then overwrites Vote(1) in `votes[A]` during ingestion. The receiver's transient tally flips between the two re-applies, visible to any UI subscribed to `digest_watch` mid-ingestion.

**Mitigation**: this is a *transient* anomaly, not a convergence bug. Final state still converges because topo-sort is canonical (Kahn's algorithm + `BTreeSet` lex order — see §4.1.2 source citation). Two peers with the same final event set arrive at the same final `votes[A]` regardless of arrival ordering. State-tier test 6b (`vote_replay_out_of_order_converges_to_lex_last`) is the canary: it applies two same-author Vote events in both orders against the same prior state and asserts the final state-bytes are identical. UX implications (a `digest_watch` subscriber sees the tally flip mid-stream) are documented but not mitigated in v1 — debouncing on the consumer side is the right layer for that concern.

### 6.5 State-size growth (Sybil-shaped)

**Risk**: nothing prevents a peer from authoring N forged-identity vote events. The `votes` map grows with each distinct author.

**Mitigation**: this is the structural Sybil-resistance question per [maintenance.md](2026-05-09-myrhiza-master-design/maintenance.md). For v1, the demo accepts this — polls are short-lived, in trusted social-graph contexts. The `votes` map's `BTreeMap` ordering keeps it deterministic regardless of size; the bump-allocator's 64 KB ceiling limits the per-call materialization. A flood of Sybil votes would exhaust the heap, returning `Reject("alloc failure")` — a graceful degradation, not a crash. Production deployments would couple to a participation module per [maintenance.md](2026-05-09-myrhiza-master-design/maintenance.md), which is post-v1 work per [`prior-art/sybil-resistance/README.md`](../prior-art/sybil-resistance/README.md) §"For Myrhiza".

### 6.6 Single-poll-per-topic UX

**Risk**: §4.1.1's decision means creating a new poll requires creating a new topic, which requires re-inviting all participants. This is awkward UX for "let's run a quick poll" use cases.

**Mitigation**: documented as v1 behavior. A future "poll-board" app could host multiple polls per topic; v1 ships single-poll. The B-7 harness's `--bundle` argument supports running multiple poll instances against different topics, so demo flow is feasible.

## 7. Prior-art consultation

Per [`using-prior-art`](../../.claude/skills/using-prior-art/SKILL.md), consulted folders + sections:

- [`prior-art/willow/state-machine.md`](../prior-art/willow/state-machine.md) §"Shipped today: `EventKind` (chat-specific)" + §"What Myrhiza inherits" — confirms the per-app `EventKind`-as-opaque-payload pattern poll follows. The deterministic topo-sort cite (§"Convergence property" leg 2: "Kahn's algorithm + `BTreeSet` lex order") is borrowed directly. The complementary "HashMap is not deterministic" claim is cited separately from [convergence.md §4.3](2026-05-09-myrhiza-master-design/convergence.md) (the "Why not hash WASM linear memory" sub-bullet).
- [`prior-art/willow/state-machine.md`](../prior-art/willow/state-machine.md) §"Moves into the per-app `state-apply` component" — validates that the permission-gate pattern (counter's Reset, poll's EndPoll) belongs in state-apply, not the kernel.
- [`prior-art/willow/apps.md`](../prior-art/willow/apps.md) §"MVP demo apps + acceptance criteria" — names "real-time poll" as a candidate proof-point alongside counter. B-6 implements the named direction.
- [`prior-art/willow/authority.md`](../prior-art/willow/authority.md) §"pre-check = apply mechanic" (cited in [B-7 spec §3.6](2026-05-21-plan-b-7-interaction-harness-design.md)) — the propose→pre-check→apply asymmetry §4.5.2 calls out (propose can't see local author; state-apply can see event.author) is grounded in this pattern. Pre-check is mechanically the same WASM function as apply; that's why moving the creator-only check to state-apply is correct.
- [`prior-art/holochain/determinism.md`](../prior-art/holochain/determinism.md) §"What's enforced" + §"Implications for Myrhiza" — Holochain's integrity-zome / coordinator-zome split is the same shape as our state-apply / state-propose split. Their pure-function discipline (no time, no random, no remote calls in validation) maps directly to our state-apply determinism rules. Validates our choice not to do per-vote network calls from state-apply.
- [`prior-art/holochain/apps.md`](../prior-art/holochain/apps.md) §"Implications for Myrhiza" — "Plan for the consumer-distribution gap" is the broader context that poll-as-demo helps address (showing the runtime supports a non-trivial multi-author app, not just counter's single-author chain).
- [`prior-art/croquet/programming-model.md`](../prior-art/croquet/programming-model.md) §10 — Croquet's Model/View split (consulted via B-7's citation chain) underwrites the `peer_state` use for "your vote": per-peer view state IS the precedent for showing "your vote" without changing shared state. **Runner-up paradigm called out**: lockstep deterministic-VM with a global reflector. Rejected for Myrhiza-wide reasons in [`prior-art/croquet/lessons.md`](../prior-art/croquet/lessons.md); poll inherits the rejection.
- [`prior-art/wasm-component-model/lessons.md`](../prior-art/wasm-component-model/lessons.md) §"World as unit of capability declaration" — validates that poll's three components each declare their own world and the manifest binds the host-import subset per-profile. No new world introduced by B-6; reuses the four worlds B-7 already wired.

**Gaps the prior-art does not cover** (candidate triggers for future `researching-prior-art` spawns):

- **No prior-art folder on voting protocols specifically.** Liquid democracy, quadratic voting, Schulze method are all academic but absent from the corpus. Not load-bearing for v1's binary-tally poll; flag as a candidate for spawn when a future cap-token-shaped voting module lands.
- **No prior-art on multi-author last-vote-wins state-CRDT patterns.** The decision in §4.1.2 borrows from chat-message-edit precedents (every chat app does last-edit-wins on messages by author) but no explicit prior-art folder documents the pattern. The CRDT survey folder ([`prior-art/crdts/`](../prior-art/crdts/)) covers Automerge / Yjs / Loro at the structural level; per-key-last-write-wins-by-author specifically (LWW-register keyed by author, no vector-clock) is its own micro-pattern worth ~one paragraph in a future revision.

## 8. Estimate

**2–3 days of focused work**, aligned with the [post-B-4 gap analysis](../reports/2026-05-21-mvp-gap-analysis.md) §"B-6: Poll app" estimate. Breakdown:

- **0.5 day** — three fixture crates (state-apply ~120 LOC, state-propose ~60 LOC, interaction ~150 LOC) + `Justfile` recipes + `tests/fixtures/built/poll-*.wasm` outputs (Path-B) OR `examples/poll/src/{state,propose,interaction}.rs` + `examples/poll/manifest.toml` (Path-A). Most of this is mechanical copy-from-counter; the novel part is poll's `apply` (~80 LOC of decode + match) and its hand-rolled BTreeMap canonical encoder (~30 LOC).
- **0.5 day** — `test-utils::build_signed_poll_bundle()` helper + `helpers::poll_handle()` (Path-B) or sdk-layer wiring (Path-A); harness `peer_state` plumbing per §4.1.4.
- **1.5 days** — state-tier tests (cases 1–6, 6b, 7–10) + kernel-tier tests (K1–K3, ~4 tests). Vote-replay out-of-order (test 6b) is the trickiest fixture-shape; budget for one debug pass on the canonical-encoder.
- **0.5 day** — coexistence test K4 + cross-test fmt/lint cleanup + spec-coverage table update + commit/PR shepherd.

Total: 3 days (the upper bound of the 2–3 day range). Path-A vs Path-B difference is ≤0.5 day of file-shuffling; the technical content is identical.

## 9. Acceptance criteria

B-6 ships when:

- [ ] Three poll WASM components build clean via `just build-fixtures` (Path-B) or `cargo build -p poll` (Path-A).
- [ ] `crates/test-utils/src/bundle.rs` (or SDK equivalent under Path-A) exposes a signed-poll-bundle builder.
- [ ] `crates/kernel/tests/poll_state_apply.rs` runs the state-tier tests covering §4.1.5 table cases 1–6, 6b, and 7–10, all green.
- [ ] `crates/kernel/tests/poll.rs` runs kernel-tier tests K1–K3, all green.
- [ ] `crates/kernel/tests/coexistence.rs::poll_and_counter_coexist_no_event_crossing` extends the existing coexistence test with a poll-vs-counter variant; green alongside the existing `two_apps_coexist_no_event_crossing`.
- [ ] `myrhiza-cli --bundle <poll-bundle-path>` runs against the B-7 harness — with the §4.1.4 `peer_state` contract addition wired — drives a full create → vote → end lifecycle from scripted stdin, and produces the expected canonical state-digest at the end.
- [ ] `just ci` passes (fmt + clippy `-D warnings` + tests).
- [ ] `docs/reports/2026-05-21-mvp-gap-analysis.md` updates implementation.md §20 item 16 from ❌ to ✅.

## 10. Sources

- [mvp.md §15.2](2026-05-09-myrhiza-master-design/mvp.md) — MVP shape: counter + poll.
- [mvp.md §15.3](2026-05-09-myrhiza-master-design/mvp.md) — multi-tier test hierarchy.
- [mvp.md §15.4](2026-05-09-myrhiza-master-design/mvp.md) — workspace shape.
- [implementation.md §20 item 16](2026-05-09-myrhiza-master-design/implementation.md) — poll app implementation step.
- [convergence.md §4.6](2026-05-09-myrhiza-master-design/convergence.md) — per-topic genesis + topic-ID derivation.
- [convergence.md §4.7](2026-05-09-myrhiza-master-design/convergence.md) — TUTTI-shaped drift detection (canary for §6.1 risk).
- [architecture.md §3.1, §3.5](2026-05-09-myrhiza-master-design/architecture.md) — four component profiles, state-apply ABI normative section.
- [determinism.md §5.1, §5.4](2026-05-09-myrhiza-master-design/determinism.md) — deterministic helper set + canonical bincode pinning.
- [docs/specs/2026-05-21-plan-b-5-coexistence-design.md](2026-05-21-plan-b-5-coexistence-design.md) — pattern for fixture-shaped second app; coexistence test scaffolding poll extends.
- [docs/specs/2026-05-21-plan-b-7-interaction-harness-design.md](2026-05-21-plan-b-7-interaction-harness-design.md) — three-component bundle path, `myrhiza-cli` harness, four interaction-component projections.
- [docs/reports/2026-05-21-mvp-gap-analysis.md](../reports/2026-05-21-mvp-gap-analysis.md) — gap analysis assigning B-6 the poll-app slice + estimate.
- `tests/fixtures/counter-state-apply/src/lib.rs` — fixture template (bump allocator, byte-offset decoder, genesis discriminator).
- `tests/fixtures/counter-state-propose/src/lib.rs` — propose template.
- `tests/fixtures/counter-interaction/src/lib.rs` — interaction template (i64 decimal formatter; whitespace-tokenized dispatch parser).
- `tests/fixtures/echo-state-apply/src/lib.rs` — second-app template (overwrite semantics; genesis-discriminator mirror).
- `wit/myrhiza-kernel/wit/world-state-apply.wit`, `world-state-propose.wit`, `world-interaction.wit`, `types.wit` — WIT worlds reused verbatim by poll.
