**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Component profiles


## 3. Component profiles

Components within an app or module declare a runtime profile.
Profiles differ in determinism requirements and which host imports
they may bind.

| Profile | Purpose | Determinism | Where it runs |
|---|---|---|---|
| `state-apply` | Materialize event into state; authority verdict | **Strict** — pure function of `(prior state, event)` plus deterministic helper set | Every peer materializing the topic |
| `state-propose` | Build candidate event from intent | Loose — kernel re-checks via `state-apply` in dry-run | The peer originating the event |
| `interaction` | UI / user-facing surface | Non-deterministic OK; per-peer | Any peer with a UI / agent host |
| `behavior` | Bots, bridges, automations | Non-deterministic OK; per-(peer, instance) identity | Designated peer(s) |

A fifth role, **maintenance** (PR #636's 4th profile in earlier
framing), is not a separate profile. Peers performing maintenance
work do so by instantiating maintenance-shaped components — these
are usually `state-apply` (for replay buffering, snapshot
provision) or `behavior` (for archival, sync) profile components.
"Maintenance" is a deployment posture, not a runtime profile.

### 3.1 state-apply (strict purity)

The most constrained profile. A `state-apply` component is a pure
function over `(prior state, event)` returning a new state. The
kernel calls it during normal event ingestion (apply mode) and
during pre-check (dry-run mode against a hypothetical post-state).

**Permitted host imports**: only the deterministic helper set (see
[capabilities.md](capabilities.md) §7). All return values are pure functions of inputs given the event
payload alone. No clock, no randomness, no network, no filesystem,
no environment, no threads.

**Floats**: banned at v1. App authors use scaled integers. Future
relaxation possible via manifest declaration `state-apply.allow-floats
= true` in a future child spec.

**Fuel**: instruction-count-based budget. Running out terminates
uniformly across peers. Wall-clock timeouts are not used because
they would diverge across peer hardware.

**Why strict**: cross-peer convergence is the load-bearing property.
If two peers run the same `state-apply` against the same event log
and get different state hashes, the system has failed. Strict
purity is how we prove convergence by construction.

### 3.2 state-propose (loose)

Builds a candidate event from user intent. Runs once on the
originating peer; the kernel re-runs `state-apply` (dry-run) to
verify the candidate before signing and broadcasting.

**Permitted host imports**: `host.hlc` (current hybrid logical
clock), `host.random`, `host.seal` (capability-gated, for sealing
content under app-declared key handles), `host.log`, plus the
deterministic helper set.

**Why loose**: intent generation legitimately needs entropy and
clock. The kernel re-checks via `state-apply` so non-determinism
in propose cannot leak into agreed state.

### 3.3 interaction (non-deterministic, per-peer)

User-facing UI surface. Per-peer state (cursor position, scroll
state, draft text) lives here.

**Permitted host imports**: `host.broadcast`, `host.subscribe`,
`host.kv` (per-peer key-value store), `host.user-prompt`, the UI
app's `ui:*` interfaces (panel, list, message, form, menu, etc.),
`host.open` (decryption for display).

**Determinism**: not required. Interaction state is local to each
peer; there is no convergence guarantee.

### 3.4 behavior (non-deterministic, per-(peer, instance))

Bots, bridges, automations. Long-running processes that observe
events and emit new ones.

**Permitted host imports**: superset of interaction's plus
`host.http`, `host.timer`, `host.author-event` (with behavior-
scoped IdentityScope; see [identity.md](identity.md) §6).

**Identity**: per-(peer, instance). When a peer enables a behavior,
the kernel allocates a fresh IdentityScope under the peer's identity
with `instance: { peer, kind: behavior, name: <app-chosen> }`. Events
authored by the behavior are signed under this scope. The runtime
does not migrate behavior keypairs between peers; cross-peer behavior
continuity is an app-level concern (apps that need stable bot
identity across peers register an in-band mapping event).

### 3.5 Normative host import surface

The canonical reference for permitted host imports per profile.
Subsequent sections ([determinism.md](determinism.md) §5 deterministic helper set, [crypto.md](crypto.md) §9 crypto primitives)
expand on individual imports but do not contradict this table. When
this table changes, the master spec changes — host imports are an ABI
commitment.

| Host import | state-apply | state-propose | interaction | behavior |
|---|---|---|---|---|
| `host.log(level, msg)` | permitted (output-only) | permitted | permitted | permitted |
| `host.hash(bytes)` (BLAKE3) | permitted | permitted | permitted | permitted |
| `host.verify-signature(pubkey, msg, sig)` (Ed25519) | permitted | permitted | permitted | permitted |
| `host.verify-payload-mac(envelope, key-handle)` | permitted | permitted | permitted | permitted |
| `host.install-key(handle, sealed-distribution-blob) -> ()` | permitted | permitted | permitted | permitted |
| `host.now-hlc-from-event(event-bytes)` | permitted | permitted | permitted | permitted |
| `host.author-event(scope, event-payload)` | denied | denied (kernel signs after pre-check) | denied | permitted (with behavior scope) |
| `host.hlc()` (peer-local HLC) | denied | permitted | permitted | permitted |
| `host.random(bytes)` | denied | permitted | permitted | permitted |
| `host.broadcast(topic, payload)` | denied | denied (kernel handles) | permitted | permitted |
| `host.subscribe(topic) -> handle` | denied | denied | permitted | permitted |
| `host.kv.get(handle, key)` | denied | denied | permitted | permitted |
| `host.kv.put(handle, key, val)` | denied | denied | permitted | permitted |
| `host.kv.delete(handle, key)` | denied | denied | permitted | permitted |
| `host.kv.list-prefix(handle, prefix)` | denied | denied | permitted | permitted |
| `host.user-prompt(prompt) -> response` | denied | denied | permitted | denied |
| `host.seal(handle, plaintext)` | denied | capability-gated | denied | capability-gated |
| `host.open(handle, ciphertext)` | denied | denied | capability-gated | capability-gated |
| `host.can-open(handle) -> bool` | denied | denied | permitted | denied |
| `host.x25519-ecdh(scope, peer-pubkey)` | denied | denied | denied | capability-gated |
| `host.hkdf-derive(input, info, length)` | denied | denied | denied | capability-gated |
| `host.aead-seal(key, nonce-handle, plaintext, ad)` | denied | denied | per-call gated | per-call gated |
| `host.aead-open(key, nonce, ciphertext, ad)` | denied | denied | per-call gated | per-call gated |
| `host.timer.{schedule,cancel}` | denied | denied | denied | permitted |
| `host.http.request(req) -> token` | denied | denied | denied | per-call gated |
| `host.clipboard.write(text)` | denied | denied | per-call gated | denied |
| `host.file-picker.show()` | denied | denied | per-call gated | denied |
| `host.navigation.top-level(url)` | denied | denied | per-call gated | denied |
| `host.push.register(...)` | denied | denied | per-call gated | denied |
| `host.clipboard.read()` | denied | denied | **denied at v1** | denied |
| `host.geolocation.read()` | denied | denied | **denied at v1** | denied |
| `host.microphone.record(...)` | denied | denied | **denied at v1** | denied |
| `host.camera.capture(...)` | denied | denied | **denied at v1** | denied |
| `host.screen-capture.record(...)` | denied | denied | **denied at v1** | denied |
| `host.sensor.{accelerometer,orientation,battery,...}` | denied | denied | **denied at v1** | denied |
| `ui:*` interfaces (panel, list, message, form, menu, etc.) | denied | denied | permitted | denied |

Cells:

- **permitted** — bound automatically when the profile loads.
- **capability-gated** — bound only if the calling component's
  manifest declares it ([capabilities.md](capabilities.md) §7.1).
- **per-call gated** — bound but each call rechecks the calling
  component's manifest ([capabilities.md](capabilities.md) §7.3).
- **denied** — never bound; importing it makes the component invalid
  for that profile (component-install lint rejects).

This is the v1 normative surface. Adding an import is an ABI change
(new minor version of the kernel WIT package). Removing or changing
semantics of an import is a breaking ABI change.

**Why state-propose does not have `host.author-event`**: propose
returns an unsigned candidate event payload to the kernel. The kernel
runs `state-apply` in dry-run mode against a hypothetical post-state
([convergence.md](convergence.md) §4.4 pre-check), and only if pre-check returns Accept does the kernel
sign the event under the user's IdentityScope and broadcast it. Propose
never sees a private key and never produces a signature. This makes
the propose-vs-apply gap structurally smaller — propose cannot bypass
pre-check by signing directly.

**Denied capabilities at v1** (clipboard read, geolocation, microphone,
camera, screen capture, sensors): these capabilities exist as host-
imports to make their absence explicit. Their *absence* in the
kernel WIT package means components attempting to import them fail
at component-install lint. If a future kernel minor adds any of
these, they MUST be `per-call gated` (never `capability-gated` or
`permitted`) because credential-exfiltration via clipboard read and
device-fingerprinting via sensors are well-known attack classes.

**Why behavior gets `host.author-event` rather than `host.sign-via-scope`**:
the kernel enforces that the signed payload is a structurally-valid
event under the app's WIT contract (envelope shape, deps array, payload
type). A compromised behavior cannot use the kernel's signing capability
to sign arbitrary non-event bytes (e.g. a fake bundle manifest, a fake
identity claim) under the user's behavior scope.

**Nonce handling for AEAD**: the kernel manages nonces. `host.aead-seal`
takes a `nonce-handle` (kernel-allocated, monotonically-derived) rather
than raw nonce bytes. `host.aead-open` takes raw nonce bytes (since the
ciphertext author is responsible for transmitting them). This eliminates
nonce-reuse-by-mistake on the seal path.


