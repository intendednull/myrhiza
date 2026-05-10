**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Determinism


## 5. Determinism

The convergence proof rests on three legs: content-addressed events
(per [convergence.md](convergence.md) §4), deterministic topo-sort (per [convergence.md](convergence.md) §4.1), and pure `state-apply`
(this section).

### 5.1 Deterministic helper set

The exact set of host imports `state-apply` may bind. Each is a
pure function of its inputs given the event payload alone. No
peer-local return values; no information about who-can-decrypt;
no clock; no randomness.

The set is **normative** at v1 (any addition is a kernel minor
version bump; any removal or semantic change is breaking):

```wit
host.verify-signature(pubkey: list<u8>, msg: list<u8>, sig: list<u8>) -> bool
host.verify-payload-mac(envelope: list<u8>, key: borrow<key-handle>) -> bool
host.hash(bytes: list<u8>) -> list<u8>
host.install-key(handle: key-handle, sealed-distribution-blob: list<u8>) -> ()
host.now-hlc-from-event(event-bytes: list<u8>) -> hlc
host.log(level: log-level, msg: string) -> ()
```

**Resource-handle ownership:**

- `host.verify-payload-mac` takes the key as `borrow<key-handle>` —
  the caller retains ownership; the helper observes the binding
  without consuming it. Verification can be repeated against the
  same handle.
- `host.install-key` takes `key-handle` by value — the call
  consumes the handle binding (move semantics) because installation
  is a one-shot registration; the kernel-side bookkeeping owns the
  registered handle thereafter.

**v1 deferral of key-management helpers:** `host.verify-payload-mac`
and `host.install-key` are vocabulary-registered (still authored
capabilities in the deterministic helper set) but **deferred to plan
B at v1**. Manifests for `state-apply` that declare either capability
are rejected at install with `InstallError::DeferredToPlanB(name)`.
The names are reserved so plan B can land them without a vocabulary
churn; the WIT signatures above are normative for plan B's
implementation.

**Algorithm pins** (master-spec normative; do not defer to crypto
child spec):

- `host.verify-signature` — Ed25519 only. RFC 8032 strict (rejects
  non-canonical s-values, malleable signatures). This is non-
  negotiable due to Cremers ETK 2025 ([identity.md](identity.md) §6.2 + `prior-art/mls/critiques.md`).
  ECDSA is forbidden anywhere in the kernel surface.
- `host.hash` — BLAKE3, canonical 32-byte output. Pinning the algorithm
  is required because `state-digest()` ([convergence.md](convergence.md) §4.3) gossips the hash for
  convergence verification; algorithm divergence breaks convergence.

Notes on each helper:

- **`host.install-key` returns `()` deliberately.** A boolean indicating
  "this peer can decrypt" would peer-locally branch state-apply,
  breaking determinism. Whether this peer can use the key is queried
  separately from interaction profile via `host.can-open(handle)`.
  The kernel-side bookkeeping the call updates IS part of the
  deterministic state surface — kernel implementations record the
  (handle, sealed-distribution-blob) pair on every peer in the same
  way; only the per-peer X25519 keystore (which determines actual
  decryptability) is peer-local and not visible from state-apply.
- **`host.verify-payload-mac` proves key possession, not author identity.**
  Author identity comes from the outer Ed25519 signature on the event
  itself. Verifying a MAC tells you "some holder of the key bound to
  this handle sealed this," nothing more. The handle binding is itself
  a deterministic function of the event log (via `install-key`).
- **`host.now-hlc-from-event`** is a pure decoder over event bytes.
  HLC is signed into the event envelope by the originator and extracted
  here by every peer. The kernel never consults the wall clock when
  serving this helper.
- **`host.log`** is output-only and does not affect state-digest. Log
  content is **not part of the cross-peer convergence surface** —
  implementations write log lines to a peer-local sink; cross-peer
  log content is not required to match.

**Side-channel resistance**: kernel implementations of all six helpers
MUST be constant-time with respect to secret inputs. State-apply must
not be able to infer per-peer secret state via timing differences in
helper return.

**Side-channel scope clarification**: the constant-time obligation
covers helper-internal computation over secret inputs (private keys,
key handles backed by symmetric secrets). It does NOT cover:
- Cache-timing leaks in Wasmtime's own execution of the WASM bytecode
  that consumes/branches on secret-derived values (mitigation via
  WASM cache-conscious crypto patterns; v1 audit obligation).
- Speculative-execution side-channels between components in the same
  Wasmtime instance (Wasmtime upstream issue; v1 accepts the residual
  risk; documented in [risks.md](risks.md) §19).
- Capability-gate dispatch timing (whether a specific origin is
  allowlisted for `host.http.request` may leak via timing). v1
  mitigation: kernel implements capability checks via constant-time
  set-membership lookups for the high-value-op list.

**`host.install-key` kernel-side bookkeeping**: the kernel maintains
a deterministic-state map `installed-keys: BTreeMap<KeyHandle,
SealedDistributionBlob>` per app instance. The map is updated by
`host.install-key` calls during state-apply replay; every peer
applies the same events in the same canonical order, so the map is
identical across peers. The map IS part of the deterministic state
surface but is NOT directly visible to state-apply via
`state-digest()`. Apps that want to expose key-handle state via
their digest must materialize the relevant subset into their own
app state via state-apply. Helpers `host.verify-payload-mac` and
(interaction-side) `host.can-open` consult this map but do not
expose its contents to state-apply.

### 5.2 Denied imports for state-apply

- No wall clock. No randomness. No network. No filesystem. No
  environment. No threads.
- No floats at v1 (per [architecture.md](architecture.md) §3.1). State-apply WASM modules importing or
  using float ops are rejected at component install time.
- No SIMD-float ops even if floats are eventually allowed; cross-platform
  divergence vectors.
- No nondeterministic instructions (e.g. `now-from-host-clock`).
- No tail-call ops (`return_call`, `return_call_indirect`). The
  Wasmtime default for `wasm_tail_call` differs across cranelift
  backends (on for x86_64/aarch64/riscv64, off for s390x and Winch
  in wasmtime 36) — silent cross-arch divergence. Engine pins the
  feature off; the byte-level lint defends in depth.
- No extended-const expressions in globals or data segments (the
  `extended-const` proposal). Engine pins the feature off so the v1
  const-expr surface is exactly MVP single-`*.const`.
- No exceptions, stack-switching, custom-page-sizes, or
  wide-arithmetic proposals at v1. Each is pinned off explicitly in
  the engine config so a future Wasmtime LTS bump cannot silently
  flip a default and shift the deterministic accept set.
- **Cranelift opt-level pinned to `Speed`**. Wasmtime 36's default
  matches, but opt-level participates in instruction selection —
  constant folding can elide trap sites, and register-allocation
  ordering can shift the in-bytecode position of a faulting
  instruction. A future LTS that flips the default to
  `SpeedAndSize`, or a peer building with a non-default
  `WASMTIME_OPT_LEVEL` env override that filters into `Config`
  construction, would silently shift trap boundaries on pathological
  components. The pin closes that divergence window; bumping the
  level is a kernel-major version bump.

The exhaustive feature-pin discipline lives in
`crates/wasmtime-backend/src/engine.rs::deterministic_config`; every
`Config` setter the workspace's wasmtime cargo features expose is
called there, and the kernel-major version bump rule applies to any
change in that pin set.

### 5.3 Fuel and resource limits

Instruction-count fuel budget per state-apply invocation. Running
out terminates uniformly across peers.

**v1 normative defaults** (must be pinned at master-spec level
because cross-peer fuel determinism depends on every peer running the
same fuel-cost-table AND the same per-invocation budget):

- **state-apply per-event fuel budget**: 10,000,000 (10M) Wasmtime
  fuel units per `apply()` call. Sufficient for ~10^6 typical
  instructions on Wasmtime LTS reference fuel-cost-table.
- **state-propose per-event fuel budget**: 50,000,000 (50M) units
  (5x apply; loose-determinism profile may use complex logic).
- **Memory cap per component instance**: 64 MB.
- **Maximum event payload size**: 1 MB.
- **Maximum DAG deps array size**: 64.
- **Maximum WASM operand stack**: 512 KiB (`524,288` bytes). Pinned
  so deeply-recursive components hit the same trap boundary on every
  peer; matches Wasmtime 36's current default but the pin makes the
  value participate in convergence guarantees rather than tracking
  upstream's whim.

**Pre-check shares apply's per-event fuel budget**. Pre-check fuel
exhaustion = pre-check fail-closed (event not signed). The shared
budget intentionally penalizes apps with expensive validation logic
— pre-check + apply combined cannot exceed 10M units, so apps
designing expensive checks see the cost on the originating peer
first.

**Why these defaults at master-spec level**: deferring to a child
spec means two kernel implementations could pick different defaults,
and convergence diverges at fuel exhaustion (peer A applies, peer B
traps; peer A advances state, peer B doesn't). The defaults MUST be
the same across all v1 implementations. Future kernel majors may
revise defaults; doing so is a kernel-major version bump.

**Per-host-call fuel costs**:
- `host.hash(bytes)` — `n * 5` units where n is byte-length (BLAKE3
  reference cost).
- `host.verify-signature(...)` — 5,000 units (Ed25519 verify cost).
- `host.verify-payload-mac(...)` — 1,000 units (MAC verify).
- `host.install-key(...)` — 100 units.
- `host.now-hlc-from-event(...)` — 50 units.
- `host.log(level, msg)` — `100 + n` units.

These are calibrated for the Wasmtime LTS reference fuel-cost-table.
Bumping Wasmtime LTS may require recalibration as a kernel major
bump (per [browser-native.md](browser-native.md) §14.2).

### 5.4 Encoding for state-digest

Apps export `state-digest()` returning canonical bytes for
cross-peer convergence verification. The encoding must be
deterministic.

**v1 commitment**: `bincode 1.3.x` with the explicit `Options` chain
`bincode::DefaultOptions::new().with_fixint_encoding().with_big_endian()`
(or equivalent precise pin), backed by `serde 1.0.x` (any 1.0 minor),
over `BTreeMap` / `BTreeSet` collections.

The `Options` chain MUST be pinned exactly because bincode 1.3 has
multiple `serialize`/`deserialize` entry points with different defaults
(function-level `bincode::serialize` vs `Options::with_*` builder).
Two correct implementations following different idiomatic patterns
can produce different bytes — the convergence-divergence the spec is
designed to prevent. This is a **firm pin**, not a
default — changing it is an ABI break that requires a new kernel
major version. Apps must canonically encode their state via this
combination; `HashMap`, `HashSet`, and other unordered collections
are forbidden in any field that contributes to `state-digest()`.

**Why pin instead of defer**: `state-digest` is the convergence
verification primitive ([convergence.md](convergence.md) §4.3). Two kernel implementations picking
different formats produce different digest bytes for identical
state, causing convergence false-positives and breaking the cross-
peer agreement check at acceptance criterion #2 ([mvp.md](mvp.md) §15.1). Format
must be specified at master-spec level.

**Why bincode 1.3.x specifically**: it is what Willow ships
(`prior-art/willow/state-machine.md`); it is byte-deterministic over
`BTreeMap`/`BTreeSet`; it is mature and audited. Bincode 2.x has a
different default config; pinning to 1.3.x avoids that drift.
`postcard` was considered but the format choice does not justify
the migration cost without a forcing reason — bincode is sufficient
for v1.

**The load-bearing discipline is sorted collections.** `BTreeMap`
or `BTreeSet` everywhere in any field reachable from `state-digest()`.
`#[serde(skip)]` for any unordered indices. Future kernel majors
may relax format choice (e.g. allow apps to opt into postcard via
manifest declaration) without changing the discipline.

**Event envelope encoding**: events themselves (the wire bytes
hashed to produce `EventHash`) use the same bincode 1.3.x +
explicit Options chain. `host.now-hlc-from-event(event-bytes)`
operates on these canonical bytes — two peers receiving the same
event hash see identical envelope bytes and decode identical HLCs.
The kernel rejects events that fail strict-canonical-decode (any
byte string that doesn't round-trip is invalid).


