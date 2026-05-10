**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Open questions and accepted risks


## 19. Open questions / accepted risks

### Project-shape v2 (snapshot lifecycle, kernel skew, resource cleanup)

**Snapshot lifecycle at v1**: the kernel does NOT compute, store,
or distribute snapshots at v1. Bootstrap is full-event-log replay
from genesis. This is intentional simplification: v1 acceptance
criteria do not require snapshots, counter+poll have small enough
state that full replay is fast, and snapshot lifecycle (when create,
when evict, who provides, how to verify) is non-trivial. **Snapshot
support lands at v2** as a `myrhiza-state-snapshot-cache` module
(distinct from the kernel) that subscribes to events and provides
snapshots-on-request through the standard kernel host imports.
Module is opt-in per app.

**Wasmtime LTS kernel-version-skew**: when the kernel-signing-root
publishes a new kernel binary built against a newer Wasmtime LTS,
peers running the old kernel and peers running the new kernel may
disagree on fuel exhaustion outcomes. v1 mitigation: kernel announces
its Wasmtime fuel-table version in HeadsSummary; if peers detect
mismatch, the older peer surfaces a "kernel out of date; upgrade
recommended for convergence guarantee" warning. Active divergence
from kernel-version-skew is treated by drift detection ([convergence.md](convergence.md) §4.7) as a
flagged event with a specific "kernel-version-skew" reason rather
than generic "convergence drift."

**Resource-handle cleanup discipline**: the kernel MUST revoke all
outstanding resource handles when a component instance terminates by
any path (normal exit, fuel exhaustion, trap, fatal error,
operator-initiated kill). Component instance restart yields a fresh
handle table; previously-issued handles are no longer valid. v1
implementation tests: handle-revocation-on-instance-termination
under representative termination scenarios. Failure to revoke is a
v1 audit blocker.

### Performance and correctness

- **MLS performance under WASM**: ~2-5x slower than native MLS for
  group operations. The 2-5x figure is steady-state, post-warmup.
  Cold instantiation overhead can be ms-class per call; aggressive
  instance caching is required. **Benchmark MLS-in-WASM at expected
  group sizes before committing the `myrhiza-crypto-mls` module**
  to canonical. Reopen the kernel-baked-MLS option if module path
  doesn't make budget.
- **Wasmtime overhead figure honesty**: [browser-native.md](browser-native.md) §14.5 cites ~2-5% overhead
  for Wasmtime vs native code. This is the steady-state straight-
  line numeric figure. Hot-path state-apply with frequent host-import
  crossings (signature verify, hash, payload-MAC verify) sees higher
  overhead — host-call ABI translation costs dominate over WASM
  execution costs. Realistic figure: 2-15% depending on workload.
  Sandbox is non-negotiable; this is the cost.
- **Component instantiation overhead**: ~ms-class on Wasmtime; higher
  on jco. Aggressive caching of `Engine::precompile_component` +
  `InstancePre` reuse is required. v1 measurement on Safari iOS in
  particular is unverified; budget for surprises.
- **jco preview2 sync-only ABI**: submit-and-poll is the workaround;
  ergonomics are real. Preview3 async stabilization improves this
  but does not change v1. **Preview3 has been "almost ready" for
  ~3 years** per `prior-art/wasm-component-model/lessons.md`; treat
  the timing as uncertain.
- **Browser CM nested in browser-WASM (Leptos UI app loading nested
  CM components)**: not battle-tested at scale. Risk of weird bugs.
  Mitigation: counter + poll MVP exercises this path early; commit
  to early benchmarking on Safari iOS specifically.
- **Wasmtime version churn**: Cranelift fuel cost tables may shift
  between Wasmtime majors. Mitigated by Wasmtime LTS pin ([browser-native.md](browser-native.md) §14.2).
  LTS bump is a kernel MAJOR version bump (convergence-breaking;
  see [browser-native.md](browser-native.md) §14.2).

### Security

- **Author key compromise**: phishing-shape attack surface. Mitigated
  by user-visible bech32m author identity at install + revocation
  topic auto-subscription ([distribution.md](distribution.md) §10.7) + visual hash icon ([distribution.md](distribution.md) §10.5 step 6).
  Future direction: key transparency log + petname registry —
  deferred to identity-binding child spec.
- **Identity binding gap**: pubkey-as-identity is the v1 model. There
  is no notion of "this pubkey belongs to specific human X." Phishing-
  shape attacks rely on this gap; users must out-of-band verify
  unfamiliar author identities. Future direction: petnames, web-of-trust,
  key transparency. v1 documents the gap explicitly.
- **No sigstore transparency log**: trust comes from author identity +
  user judgment. Trade accepted; matches P2P framing. Pairs with
  identity binding gap as a known v1 limitation.
- **Side-channel resistance in deterministic helper set**: [determinism.md](determinism.md) §5.1 mandates
  constant-time implementations with respect to secret inputs. v1
  audit obligation: kernel implementations of `host.verify-signature`,
  `host.verify-payload-mac`, `host.aead-{seal,open}`, `host.x25519-ecdh`
  use constant-time crypto crates (ed25519-dalek, x25519-dalek,
  chacha20poly1305 in Rust have constant-time implementations).
- **DoS in helper set**: `host.hash` and `host.verify-signature` consume
  host CPU disproportionately to WASM instruction cost. A malicious
  app could call them with large payloads to drain fuel asymmetrically.
  Mitigation (deferred to fuel-cost-table child spec): per-host-call
  fuel costs proportional to wall-clock cost, not WASM-instruction
  count.
- **Pre-check fuel exhaustion as soft DoS**: pre-check shares apply's
  per-event fuel budget. A malicious event with deep validation logic
  can consume budget that downstream peers also pay. Open question:
  separate per-event fuel for pre-check vs apply. Deferred but flagged.
- **jco shim in browser TCB**: jco's preview2 transpiler generates
  the JS that bridges WASM components to browser APIs. A jco
  resource-handle-lifecycle bug could leak handles across components.
  v1 commitment: pin a specific jco version per kernel release with
  deterministic build verification; jco upgrades are kernel ABI
  advisories.
- **Snapshot security at bootstrap**: out of v1 scope. v1 ships no
  snapshots ([convergence.md](convergence.md) §4.2 + Project-shape v2 above); bootstrap is full event
  log replay. When the v2+ `myrhiza-state-snapshot-cache` module
  ships, snapshot-fetch must re-validate by replaying the log up to
  the snapshot's anchor hash on first install — snapshots are never
  trusted for state contents, only as a bootstrap optimization.
- **Operator-deployed infrastructure + invitation flow**: operators
  needing to host many apps face a tension with social-graph Sybil
  resistance — they cannot be invited to every customer's social
  graph. v1 acknowledged limitation; future direction: capability
  attestation patterns (operator publishes "I run maintenance for
  these app shapes" attestation; apps opt to accept attestation-
  based participation in lieu of invitation).
- **Manifest TOCTOU**: capability declaration parsed at install,
  intersected at instantiation. Bundle update flow MUST re-run install
  (per [distribution.md](distribution.md) §10.5 step 7). Silent in-place bundle update is forbidden by
  spec.
- **Replay attack on submit-and-poll completion handlers**: kernel
  enforces that only kernel-issued tokens can re-enter components
  via `on-completion`. Tokens are unforgeable (kernel-side opaque
  HMAC). v1 implementation MUST verify token before dispatching
  completion.
- **Capability summary fatigue**: MetaMask Snaps lesson — users
  habituate to permission prompts. Mitigations in [distribution.md](distribution.md) §10.5: 2-second
  minimum render time on high-value-op approval; visual hash icons;
  highlighted "first time installing from this author" markers.
  Insufficient long-term; future direction: trust-rating heuristics.

### Ecosystem

- **Anonymous participation excluded by social-graph Sybil resistance**:
  documented; apps that need anonymity use other modules (tit-for-tat,
  storage proofs).
- **Module ecosystem bus factor**: official `myrhiza-*` modules we
  author, we maintain. Mitigated: encourage third-party alternatives;
  module ecosystem stays open even when official modules ship.
- **Browser peer story is relay-bound**: WebTransport-as-iroh-transport
  not a current path; WebRTC not pursued. Browser peers depend on iroh
  relays. Mitigation: relay-bridged QUIC is the only shipped path; v1
  does not pretend NAT-traversal works in pure browser.
- **iroh pre-1.0 churn**: iroh is currently 1.0-rc; API has been
  volatile (`prior-art/iroh/lessons.md` flags constant pre-1.0 API
  churn). v1 pins iroh to a specific version (TBD at implementation
  start); upgrade pain is budgeted explicitly.
- **Cherry-picked precedents disclosure**: §1 cites Agoric and Willow
  as "production-validated" for event-log replay. Agoric is a
  blockchain (consensus-given ordering); Willow is at hundreds-of-
  users scale. Neither validates "event-log replay scales as P2P
  infrastructure for write-heavy public-read apps." See [convergence.md](convergence.md) §4.5 scaling
  section for explicit ceiling acknowledgment.

### Determinism enforcement

- **Cremers ETK 2025 enforcement**: Ed25519 mandatory for IdentityScope
  long-term identity. Enforced **structurally** (not advisory) — the
  kernel does not expose any signing API that takes an algorithm
  parameter (`host.author-event` is always Ed25519). Manifest
  declaring non-Ed25519 is rejected at install.
- **Float-ban scope**: lint at WASM byte level; rejects modules
  containing `f32.*` / `f64.*` instructions in any function reachable
  from a state-apply export. Unreachable float ops in dead code are
  permitted (the linter follows reachability). Cargo-component build
  recipe for state-apply components includes `RUSTFLAGS="-Cno-float"`
  as a safeguard.
- **Resource-handle persistence**: WCM resource handles are per-
  instance and non-durable in v1. Component restart loses handles;
  apps must re-acquire from kernel state. Future direction: Agoric
  `baggage`-style upgrade convention for handle persistence across
  upgrades.
- **Behavior identity continuity across peer failures**: the runtime
  does not migrate behavior keypairs between peers. Apps that need
  stable bot identity across peer failures register an in-band mapping
  event; SDK macros default to making this binding explicit (so
  app authors don't accidentally ship behaviors that lose identity
  on restart).

### Project-shape

- **Schedule risk**: 24-32 weeks honest range; 16-20 was optimistic.
  v1 scope-reduction fallback ([mvp.md](mvp.md) §15.5) cuts jco / behavior / per-call
  gating to v1.5 if mid-project measurement shows slip. Fallback is
  preserved as recoverable.
- **Single architectural ancestor (Willow) at small scale**: Myrhiza
  inherits architectural lessons but cannot rely on Willow as
  validation at scale. v1 acceptance test (counter+poll) is a smoke
  test, not scale validation.
- **iroh strategy shift risk**: Number 0 has redirected before
  (relay infra ownership, ticket changes, FFI mothballing). v1
  pinning to a specific iroh version is the immediate mitigation;
  long-term mitigation requires kernel network-trait abstraction
  preserved as a design seam (planned in [implementation.md](implementation.md) §20).
- **"Novel angle" precision**: §1 frames "peers as infrastructure"
  as novel. Holochain and Pears have framed this similarly. The
  actually-novel piece is the **combination**: WCM + capability
  discipline + no-CRDT-in-kernel + author-bounded-scale-at-v1. No
  single prior project has shipped this combination. Honest
  positioning: not a new pitch, a new combination.


