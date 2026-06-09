**Date:** 2026-06-08
**Status:** active
**Subject:** M1b — multi-topic interaction via `host.subscribe` (sandboxed UI aggregates N topics' converged state)

# M1b — `host.subscribe` + multi-topic interaction

## 1. Problem & goal

B-13 landed the *produce* half of app I/O (kernel-mediated `propose_and_author`). This slice lands the *receive + aggregate* half for the interaction profile: a single sandboxed **interaction** (UI-projection) component that renders state drawn from **multiple topics at once** — the Discord-style multi-channel surface where one screen shows many rooms, each live over the network.

Today: one `Runtime` = one event DAG = one topic, and the interaction component's `view(state, peer-state)` is driven only by the offline B-7 CLI harness against a single topic. There is no way for a UI to observe more than its own topic, and no way for a *networked* (gossip-connected) peer to feed an interaction component at all.

Goal: a kernel-mediated **`host.subscribe(topic)`** capability that lets a sandboxed interaction component observe other topics' converged state, receive per-topic state updates as events arrive over gossip, and aggregate them **in-sandbox** into one rendered view.

### Non-goals (this slice)

- Untrusted-app topic *authority* / cross-peer delegated subscriptions (each peer authorizes its own subscriptions — B-10 assumption holds).
- Windowed/lazy subscription at Discord scale (the active set is *modeled* as a window; the v1 policy subscribes to all enumerated topics — see §10).
- A DHT topic→bootstrap-peers resolver (v1 uses bootstrap hints carried with topic references — see §9).
- The structured `ui:*` render contract (still types-only; v1 view bytes stay opaque per B-7 §3 Choice C).
- The behavior profile (no backend variant exists; out of scope).

## 2. Decision: `host.subscribe`-centric, not a trusted coordinator

The multi-channel UI could be built two ways:

- **Trusted coordinator (rejected runner-up).** A trusted native layer (kernel or per-embedder shell) holds N `Runtime`s and aggregates their state for the UI. The render component stays minimal, but the *aggregation logic* — the complex, app-varying part (merge, windowing policy, composition) — lives in trusted code, and every embedder re-implements it.
- **`host.subscribe` capability (chosen).** The kernel exposes one small, fixed, mediated primitive; **all aggregation logic moves into the sandbox**. Trusted code grows by a generic, app-agnostic amount; the variable part is untrusted WASM.

Chosen for trusted-surface minimization, an open "any app" architecture, and fidelity to Myrhiza's thesis ("capabilities are the only host surface; apps reach the host through declared imports"). A trusted aggregation coordinator is *less* aligned with that thesis than a subscribe capability. The runtime cost is a real ABI change (a new host import), taken deliberately per CLAUDE.md.

The two architectures share the same plumbing — N per-topic convergence engines + a delivery path. They differ only in (a) what *drives* the subscription set (sandboxed app via `host.subscribe`, not trusted embedder config) and (b) where aggregation lives (sandbox, not trusted code).

### 2.1 Sequencing

M1b is the *receive + aggregate* half of app I/O; B-13 was the *produce* half. This spec, its [plan](../plans/2026-06-08-m1b-host-subscribe-multitopic-interaction.md), and the prior-art corpus (§3) land together on the `docs/m1b-subscription-prior-art` branch. **Implementation is sequenced after that docs PR merges, on a base that includes B-13** — the read-only per-topic engine reuses `Runtime`, and acceptance tests drive the authoring side via B-13's `RuntimeHandle::propose_and_author` (or the pre-B-13 authoring helper if implementation precedes B-13's merge). Note: `Runtime::start` has **no** `propose` parameter (B-13 added `propose_and_author` as a `RuntimeHandle` method); a read-only engine is constructed with `author_key: None`.

## 3. Prior art consulted

Corpus established for this spec (`docs/prior-art/`, all `[active]`, 2026-06-08):

- **`wasm-async-streaming/lessons.md`** — delivery shape. **Borrow:** sync-acquire + host-invoked guest-callback (`on-subscription-*` export), portable across Wasmtime *and every browser today* (Safari ships no JSPI; stable as of 2026-06). **Reject (runner-up):** Component-Model `stream<T>` resource — native-ideal but JSPI-gated in-browser; kept as a documented future native optimization under the same app-facing handle (per `abi.md §8.5`'s migration promise). **Reject:** recycling single-use submit-and-poll tokens for per-message delivery (replay-protection breaks; no "submit" for unsolicited inbound).
- **`streaming-capabilities/lessons.md`** — handle shape. **Borrow:** model the subscription as a WIT `resource` (unforgeable handle-table index for free; `drop` = unsubscribe/revocation signal); the **caretaker/revoker** pattern (Miller, *Robust Composition*) for killing a live subscription; the **delegation (grant, may live in state) vs invocation (per-message delivery, peer-local)** split. **Open:** WIT-resource ↔ capability-token mapping for *network-crossing* caps (deferred; v1 subscriptions are peer-local).
- **`topic-discovery/lessons.md`** — how the app learns a foreign topic id. **Borrow:** in-state `m.space.child`-style enumeration (a parent topic's converged state lists child topic ids + bootstrap hints); enumeration may live in deterministic state, the *subscribe act* may not. **Defer:** the rotating-BEP44 / provider-record topic→peers resolver (runner-up: Kademlia `GET_PROVIDERS`).
- **`matrix-sliding-sync/lessons.md`** — scale. **Borrow (later):** the active subscription set is a *moving window* with per-topic teardown/rehydrate (MSC3575/4186); v1 models the window type but its policy is "all enumerated topics."

Determinism / capability constraints honored from the master design: `architecture.md §3.5` (state-apply binds only deterministic helpers), `capabilities.md §7.2` (`M_eff = A_ambient ∩ M_required`), `abi.md §8.5` (submit-and-poll async contract), `convergence.md §4.3/§4.6` (per-topic digest; bundle-derived topic id).

## 4. The capability

`host.subscribe` is already registered in the vocabulary as `CapabilityClass::HostImport` (`crates/manifest/src/vocabulary.rs:47`) and listed in the `architecture.md §3.3` interaction permitted-imports table. This slice *binds* it for the interaction profile; it does not invent it.

### 4.1 WIT (added to `host-non-deterministic`, imported by the interaction world)

```wit
/// A live subscription to a foreign topic's converged-state feed.
/// Peer-local and non-deterministic. Dropping the handle unsubscribes.
resource subscription {
    /// Stable correlation id, echoed by the interaction world's
    /// `on-subscription-update` delivery export.
    id: func() -> u64;
}

/// Subscribe to a foreign topic's converged-state feed. The kernel
/// ensures a per-topic convergence engine for `topic`, begins (or
/// resumes) gossip sync, and delivers state via the interaction
/// world's `on-subscription-update` export. `topic` is a 32-byte
/// content-addressed topic id. Returns a handle, or an error string
/// (capability denied, unknown/unreachable topic, resource cap hit).
subscribe: func(topic: list<u8>) -> result<subscription, string>;
```

The interaction world (`wit/myrhiza-kernel/wit/world-interaction.wit`) gains one export, mirroring the existing `on-broadcast-completion` shape:

```wit
/// Delivery of a subscribed topic's converged state. Called by the
/// kernel on initial sync and on every subsequent converged-state
/// change of the subscribed topic. `sub-id` correlates to
/// `subscription.id()`. The component stores these per-topic states in
/// its own linear memory and composes them in `view`. Peer-local;
/// MUST NEVER enter any state-digest.
export on-subscription-update: func(sub-id: u64, topic: list<u8>, state: list<u8>);
```

`view(state, peer-state) -> list<u8>` and `dispatch(action) -> result<list<u8>, string>` are **unchanged.** The aggregated multi-topic state lives in the component's linear memory (peer-local — acceptable, interaction is non-deterministic), accumulated from `on-subscription-update` calls; `view`'s `state` argument remains the component's *primary* topic state. This is the key simplification: no view/dispatch ABI change, no framed-multi-state blob.

### 4.2 Acquire / deliver / revoke

- **Acquire (sync).** The component calls `subscribe(topic)`. The host-side `func_wrap` records a pending subscription `(sub_id, topic)` in `HostState` and returns the `subscription` handle (carrying `sub_id`) immediately. It performs **no network I/O** — the backend stays network-free; the kernel services the request out-of-band (this is the submit-and-poll discipline of `abi.md §8.5` applied to subscription *establishment*).
- **Establish (kernel, async).** After the interaction call returns, the kernel drains pending subscriptions: for each topic it ensures a per-topic convergence engine (a read-only `Runtime` — §6) and registers a forwarder from that engine's `digest_watch` to this instance's `on-subscription-update`.
- **Deliver (kernel → guest callback).** On initial sync and each subsequent converged-state change, the kernel invokes `on-subscription-update(sub_id, topic, state)` on the *same long-lived* interaction instance.
- **Revoke.** The kernel observes a drop via the WIT resource **destructor** — Wasmtime invokes it both when the guest drops the `subscription` handle *and* on instance teardown, so both routes unify through one path. The destructor records a pending-unsubscribe `sub_id` that the kernel drains exactly as it drains acquires; the subscription manager then stops delivery, tears down the forwarder, and decrefs the per-topic engine (releasing it at zero references). A subsequent guest use of a dropped handle **traps** (Component Model resource semantics). Per the caretaker pattern, revocation is a first-class kernel-side operation routed through the destructor — not a polled afterthought, and not reliant on the guest calling an explicit `unsubscribe`.

## 5. Why this delivery shape (not the alternatives)

| Option | Verdict |
|---|---|
| **sync-acquire + guest-callback export** (chosen) | Portable Wasmtime + all browsers today; no JSPI dependency; extends the existing `on-*-completion` export idiom. |
| Component-Model `stream<T>` resource | Native-ideal but JSPI-gated in browser (Safari gap). Kept as a future native fast-path under the *same* `subscription` handle. |
| Recycle submit-and-poll request-tokens | Rejected: single-use tokens are replay-protected and there is no "submit" per inbound message; forcing a stream through them churns the 256-token cap or breaks replay protection. |

Per-message delivery carries **full converged state** for the topic in v1 (mirrors `view(state, …)`, which already takes whole state). Delta encoding is a deferred optimization (§12).

## 6. Kernel wiring — per-topic engines + the subscription manager

The chosen architecture reuses the per-topic `Runtime` as the convergence atom (Layer 1, unchanged) and adds a generic, app-agnostic mechanism:

- **Per-topic engine.** A subscription to topic `T` ensures a **read-only `Runtime`** for `T` on this peer (no `author_key`, no `propose`): it subscribes to `T`'s gossip topic, applies events, and publishes converged state via its existing `digest_watch` (`runtime.rs` apply/replay → `digest_watch_tx.send(state)`). Multiple subscriptions to the same `T` share one engine (refcounted). This reuses the proven multi-`Runtime`-per-peer coexistence (`iroh_coexistence.rs`).
- **Subscription manager (new, per interaction instance).** Holds `sub_id → (topic, engine handle, forwarder task)`. Drains `HostState.pending_subscriptions` after each interaction call; for each, ensures the engine and spawns a forwarder that awaits the engine's `digest_watch` and calls `instance.on_subscription_update(sub_id, topic, state)`. Owns teardown on drop/termination.
- **Driving the instance.** The kernel now holds the long-lived interaction instance and invokes its exports (`on-subscription-update`, and `view` when the embedder renders). This is the generic "kernel drives the interaction component" mechanism — *not* app-specific aggregation, which stays in the guest.

The subscription manager **is** the same plumbing a trusted coordinator would need; the difference is it's driven by the sandboxed app's `host.subscribe` calls and delivers to the guest rather than to trusted aggregation code. Aggregation (what to do with N states) is the guest's job.

### 6.1 Embedder-facing view delivery

The component accumulates subscribed states via callbacks; the embedder obtains rendered bytes by calling `view()` on the kernel-held instance. v1 exposes a `RuntimeHandle`-style accessor to (a) call `view(peer_state) -> bytes` on demand and (b) a `watch` channel that re-publishes view bytes after each `on-subscription-update` (the "watch + render" delivery approved earlier). The real UI sink / `ui:*` contract remains deferred (M3/M4).

## 7. Determinism & sandbox boundary (hard lines)

1. **`state-apply` rejects `host.subscribe`.** It is non-deterministic; `ambient_set(Profile::StateApply)` must never include it, so a state-apply (or state-propose) manifest declaring it fails validation. Enforced by the gating rework (§8) and locked by test.
2. **Subscription state never enters `state-digest()`.** Which topics a peer subscribed to, which updates arrived, and delivery order are peer-local. They live only in the component's linear memory and the kernel's subscription manager — never in any canonical state. (Two peers with identical event history but different subscription sets must converge identically.) In this v1 slice the isolation is **structural, not merely disciplinary**: interaction binds no authoring or broadcast capability (§7.3), so subscription-derived state in component memory cannot influence any authored event, and `state-apply` — the only profile that produces a digest — cannot bind `host.subscribe` at all (§7.1). A future SDK that lets interaction author MUST keep subscription-derived state separate from any bytes folded into an event or digest, or peers diverge silently.
3. **No authoring as a delivery side-effect.** `on-subscription-update` is projection-only. Event authoring stays explicit through `propose → author` (kernel-mediated; "propose never signs", `architecture.md §3.5`). A subscription callback that authored would diverge on per-peer delivery ordering. In v1 the interaction ambient set adds **only** `host.subscribe` among non-deterministic imports (not `host.broadcast`, not `host.author-event`), so a callback holds no capability to author or broadcast, and the export returns unit — the no-side-effect property is structural. (Were `host.broadcast` bound for interaction later, a broadcast from a callback is a peer-local output, never convergence-affecting; `host.author-event` must never be bound to interaction.)
4. **Enumeration in state is fine; the subscribe act is not.** A parent topic's converged state *may* list child topic ids (deterministic, replayable). Calling `host.subscribe` on one of them is the non-deterministic, peer-local act.
5. **Capability is static, per-component manifest.** The grant ("may subscribe") is a static manifest declaration; the component cannot escalate at runtime (`capabilities.md §7.3`).

## 8. ABI touch-point checklist (dependency-ordered)

The gating module already names this work: its doc comment (`crates/wasmtime-backend/src/gating.rs:10-17`) calls the `profile` parameter "the documented extension point for when the surfaces diverge." This slice activates it. **Correction to the deep-research brief:** because `host.subscribe` is `HostImport`-classified, `validate_manifest`'s class check (`gating.rs:93-98`) rejects it for every profile today — so the change is a *coordinated* ambient + validate + bound + wire + prewalk edit, not an `ambient_set` tweak alone.

1. **WIT — `host-non-deterministic.wit`:** add `resource subscription { id: func() -> u64; }` and `subscribe: func(topic: list<u8>) -> result<subscription, string>;`. **WIT — `world-interaction.wit`:** add the `on-subscription-update` export.
2. **`ambient_set` (gating.rs:45) — branch on profile.** Return the deterministic-helper set for all profiles, **plus `host.subscribe` for `Profile::Interaction`**. Invariant (test-locked): `ambient_set(StateApply)` and `ambient_set(StatePropose)` contain only `DeterministicHelper`-class caps.
3. **`validate_manifest` (gating.rs:84) — ambient-membership gate, not class gate.** For a declared host-import: `DeferredToPlanB` short-circuit → `UnknownImport` if `classify` is `None` → `UnauthorizedImport` if not in `ambient_set(profile)` → else OK. This is **behavior-preserving** for all existing cases (today every profile's ambient is det-helpers-only, so a `HostImport` cap is still rejected everywhere except interaction's new `host.subscribe`). Keeps `manifest_with_apply_only_capability_declared_for_propose_rejects` green.
4. **`bound_imports` (gating.rs:124)** — already intersects declared ∩ ambient, so `host.subscribe` is bound for interaction automatically once it is in the ambient set. No structural change.
5. **`wire_linker` (gating.rs:173) — bind `subscribe` + the `subscription` resource for interaction.** Add an instance block for `myrhiza:kernel/host-non-deterministic@1.0.0`, register the `subscription` resource (`Linker::resource`), and `func_wrap` `subscribe` (record pending + return handle), gated on `bound_imports.contains("host.subscribe")`. This is the meatiest backend edit (resource lifecycle + host-state plumbing).
6. **`prewalk_imports` (engine.rs:124) — permit the non-deterministic instance for interaction.** Add `myrhiza:kernel/host-non-deterministic@1.0.0` to the interaction allowlist and audit its function children map to `bound_imports` entries (keep the type-only audit for `host-ui-surfaces`).
7. **Backend trait — `InteractionInstance`** (`crates/backend/src/lib.rs:157`): add `call_on_subscription_update(&mut self, sub_id: u64, topic: &[u8], state: &[u8]) -> Result<(), BackendError>`; implement in `crates/wasmtime-backend/src/interaction_instance.rs`. Add `HostState.pending_subscriptions: Vec<(u64, Vec<u8>)>` + a `sub_id` counter, drained by the kernel.
8. **Kernel** (`crates/kernel/`): the subscription manager (§6), read-only `Runtime` construction, the forwarder task, and the embedder-facing view accessor/watch (§6.1). New `RuntimeError`/handle variants as needed.
9. **Versioning + fuel.** Adding functions/exports to existing interfaces is a **kernel minor** bump (`vocabulary.rs` header convention). `subscribe` is pinned to a nominal fixed per-call cost (`SUBSCRIBE_FUEL_COST`, e.g. 100 units; cf. `determinism.md §5.3`). Interaction fuel is not convergence-load-bearing (interaction is non-deterministic per-peer), but the cost is pinned so independent kernel builds agree.

## 9. Topic discovery (v1)

v1 does **not** build a DHT resolver. Topic ids reach the app two ways:

- **Root topic:** out-of-band — the app's primary topic, plus any topic id carried in an install ticket / bundle reference, bundled with a **bootstrap hint** (a few known `NodeId`s).
- **Child topics:** **in-state enumeration** — a parent topic's converged state contains child topic ids (and bootstrap hints) as ordinary app data (the `m.space.child` pattern). The component reads them from `view`'s `state` and calls `host.subscribe`.

The kernel resolves `topic → bootstrap peers` from the carried hint (the iroh-gossip `subscribe(topic, bootstrap)` call already requires caller-supplied bootstrap). The rotating-BEP44 / provider-record resolver is deferred (`topic-discovery/lessons.md`); v1's hint-carrying matches how revocation/publication topics already bootstrap (B-11/B-12).

### 9.1 v1 scoping (capability-by-reachability)

The manifest grant is the **boolean** `host_imports["host.subscribe"] = true`. A static per-topic allowlist of concrete topic hashes is **infeasible** at author time (topic ids are runtime-derived BLAKE3 hashes), so v1 does not attempt one. Scoping is **by reachability**: a component can only `subscribe` to topic ids it can *name*, and it learns child ids only via in-state enumeration from topics it already follows. You cannot subscribe to what you cannot name. A future attenuation (topic-namespace caveats, per `streaming-capabilities`) can tighten this.

**v1 implementation note — the kernel does not validate topic-id membership.** A component can pass any 32-byte value to `subscribe` and receive a handle; an unreachable real topic and a fabricated one are indistinguishable to the caller (both simply never sync), so there is no existence oracle. Consequences: (a) apps MUST NOT map untrusted user input directly to topic ids — safe discovery is in-state enumeration or explicit user-approved references; an interaction component that turns a user-typed string into a topic id is a misuse; (b) fabricated or abandoned subscriptions are bounded by the outstanding-subscription cap (§11), capping resource exhaustion. A kernel-side topic registry / membership check is deferred (M3+).

## 10. Windowing & scale (v1 models, defers)

The subscription manager's active set is typed as a **window** from day one (an explicit "active subscriptions" set with add/remove), so the sliding-window scale path (`matrix-sliding-sync`) slots in without an API break. v1 **policy** is trivial: subscribe to every enumerated topic the app requests; no eviction. The Discord-scale case (user in 50 servers, 10 on screen → windowed subscribe/unsubscribe with per-topic teardown/rehydrate) is deferred to a follow-up once the corpus's sliding-sync model is applied. Back-pressure: the kernel bounds per-subscription buffering and paces `on-subscription-update`; an undrained backlog is dropped-newest-coalesced (state is idempotent — latest wins), never unbounded.

## 11. Error handling

- `subscribe` returns `result<subscription, string>`: `Err` on capability-denied (defense-in-depth; gating should already have rejected at install), unknown/zero-length topic, no bootstrap route, or outstanding-subscription cap hit.
- Capability misuse by `state-apply`/`state-propose` fails at **manifest validation** (`UnauthorizedImport`), never reaching runtime.
- A per-topic engine that cannot reach peers still returns a handle; it simply delivers no updates until it syncs (no error — mirrors a cold gossip topic).
- Dropped-handle reuse traps (resource semantics).

## 12. v1 scope boundary

**In:** the `host.subscribe` capability + `subscription` resource handle; the coordinated gating rework (ambient/validate/bound/wire/prewalk) binding it for interaction only; `on-subscription-update` guest-callback delivery; per-topic converged-state delivery via read-only per-topic `Runtime`s + the subscription manager; the determinism gating (state-apply/propose reject); embedder-facing on-demand `view` + post-update watch; a multi-topic acceptance fixture proving N topics aggregate into one view that re-renders when any topic's state changes (MemNetwork), plus one iroh smoke test.

**Seamed for later (modeled, not built):** sliding-window subscription policy (window type exists; policy = all-enumerated); DHT topic→peers resolver (v1 = bootstrap hints); `stream<T>` native fast-path (handle is forward-compatible); delta-encoded delivery (v1 = full state); topic-namespace capability attenuation (v1 = reachability scoping); structured `ui:*` render contract.

## 13. Testing strategy

- **Gating (wasmtime-backend):** `host.subscribe` accepted for `Interaction`, rejected (`UnauthorizedImport`) for `StateApply` and `StatePropose`; all existing gating tests stay green (esp. `manifest_with_apply_only_capability_declared_for_propose_rejects`); ambient-invariant test (state-apply/propose ambients are det-helpers-only).
- **Backend:** an interaction fixture importing `host.subscribe` instantiates + links; `on-subscription-update` round-trips; dropped-handle reuse traps.
- **Kernel acceptance (MemNetwork):** a multi-topic interaction fixture subscribes to two sibling topics, both fed by separate authors; assert `view` output aggregates both topics' state and re-renders after an event applies on *either*; subscription set never affects either topic's `state-digest` (determinism); drop unsubscribes (no further delivery).
- **iroh smoke (`network-iroh`):** two peers, peer A subscribes to peer B's topic, B authors, A's interaction view reflects it over real gossip. One test.
- **Determinism guard:** two peers with identical event history but different subscription sets converge to identical per-topic digests.
- No fixed sleeps — use the existing `poll_until` settle helper. Fixtures build only in a non-nested worktree (regenerate via the primary checkout); regenerate `tests/spec-coverage.md`.

## 14. Open questions / deferred decisions

1. **`subscription` resource vs opaque `u64` handle.** v1 uses a WIT `resource` (unforgeable for free; `drop` observable). If resource-lifecycle plumbing across the kernel boundary proves heavy in the plan's spike, the documented fallback is an HMAC-tagged opaque id validated kernel-side (`streaming-capabilities` notes both); the app-facing contract is unchanged either way. **Resolve in plan T0 spike.**
2. **Read-only `Runtime` vs a lighter per-topic materializer.** v1 reuses `Runtime` (proven, no new convergence code). If its lifecycle is too heavy for many subscriptions, a slimmer materializer is a later optimization. **Reuse for v1.**
3. **`peer_state` per subscribed topic.** v1 renders subscribed topics from their converged `state` only; per-topic `peer_state` (scroll, unread) is deferred to the windowing follow-up (Croquet `viewId` precedent).
4. **Where the component first calls `subscribe`.** v1: during `dispatch` (an explicit "open these channels" action) or a one-time init dispatch; no implicit auto-subscribe. **Plan picks the fixture's trigger.**

## 15. Risks

- **Scope creep into the windowing/scale problem.** Mitigation: the window type is modeled but its policy is trivial; sliding-sync is explicitly out.
- **Resource-handle lifecycle complexity** (WIT `resource` across host/guest + kernel teardown). Mitigation: T0 spike; opaque-id fallback documented.
- **Backend↔kernel layering.** `host.subscribe` needs network, which the backend must not own. Mitigation: the backend only records pending intent in `HostState`; the kernel services it — same discipline as submit-and-poll.
- **Many per-topic `Runtime`s** at scale (memory/overlay cost). Mitigation: refcounted shared engines; windowing (deferred) bounds the active set; honest cap + log when hit.
- **Determinism regression** if subscription state leaks into a digest. Mitigation: the determinism-guard test; `state-apply` cannot bind the cap at all.

## Sources

- Prior-art corpus: [`prior-art/wasm-async-streaming`](../prior-art/wasm-async-streaming/), [`prior-art/streaming-capabilities`](../prior-art/streaming-capabilities/), [`prior-art/topic-discovery`](../prior-art/topic-discovery/), [`prior-art/matrix-sliding-sync`](../prior-art/matrix-sliding-sync/).
- Master design: `2026-05-09-myrhiza-master-design/{architecture.md §3.3/§3.5, abi.md §8.5, capabilities.md §7.2/§7.3, convergence.md §4.3/§4.6, determinism.md §5.1/§5.3}`.
- Code: `crates/manifest/src/vocabulary.rs:47`, `crates/wasmtime-backend/src/gating.rs:10-17,45,84,124,173`, `crates/wasmtime-backend/src/engine.rs:124` (prewalk), `crates/backend/src/lib.rs:157` (InteractionInstance), `wit/myrhiza-kernel/wit/world-interaction.wit:10,14`, `crates/kernel/tests/iroh_coexistence.rs`.
- Predecessor: [B-13 kernel-mediated authoring](2026-06-03-b-13-kernel-mediated-authoring-design.md) (the produce half).
