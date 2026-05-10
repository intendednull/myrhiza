**Date:** 2026-05-09
**Status:** draft

# Myrhiza — master design

The runtime spec. What we are building, what its shape is, what we
have committed to, what we have explicitly deferred, and how the v1
acceptance proof works.

This document is the canonical reference for any spec, plan, report,
or implementation work that touches the runtime. Child specs refine
specific subsystems — they cite this file and conform to it. When
this file changes, child specs review for impact.

## 1. Vision and scope

Myrhiza is a **P2P app runtime**. A small kernel hosts typed,
capability-mediated, content-addressed apps. Apps are bundles of
WebAssembly components. The kernel owns identity, peer protocol,
event/DAG primitives, the component loader, and the capability
arbiter. Everything else — chat, wikis, kanban, polls, whatever
someone builds in two years — is an app.

**The novel commitment**: peers are infrastructure. Storage,
replication, sync, replay buffering, snapshot custody — these are
work performed by participants, not deployed services. As more peers
participate in an app, the app's maintenance capacity grows. No
infrastructure deploy required to scale.

**The security commitment**: capabilities are the only way components
reach beyond their own memory. Apps cannot touch private keys, the
network, persistent storage, or other apps' state directly — every
operation is mediated by a kernel-arbitrated capability. WASM
execution is non-negotiable on every backend (native, browser,
mobile); compiling apps to native code for performance is explicitly
rejected.

**What this is not**:

- Not a chat client. Chat is one app among many.
- Not a plugin framework for a host application. Apps are the
  product; the kernel is the substrate.
- Not a CRDT library. Apps may use CRDTs internally; the kernel
  stays generic.
- Not a service to deploy. Peers are the runtime.

## 2. The three-tier architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                       KERNEL                             │
   │  Identity. Peer protocol. Event/DAG primitives.          │
   │  Component loader. Capability arbiter. Crypto            │
   │  primitives. Narrow native imports.                      │
   └──────────────────────────────────────────────────────────┘
                              ▲
                              │ host imports (WIT-typed)
                              │
   ┌──────────────────────────────────────────────────────────┐
   │              MODULES  (myrhiza-* WASM components)        │
   │  Cross-cutting concerns reusable across apps:            │
   │  - Participation: social-graph, tit-for-tat, ...         │
   │  - Permission: rbac, governance, invite-chain, ...       │
   │  - Crypto: mls, channel-key, double-ratchet, ...         │
   │  - State helpers: snapshot-cache, log-prune,             │
   │    crdt-{automerge,yjs,loro}, ...                        │
   │  - Identity: multi-device, behavior, ...                 │
   │  - UI: components, theme-tokens, accessibility, ...      │
   └──────────────────────────────────────────────────────────┘
                              ▲
                              │ component imports / wac composition
                              │
   ┌──────────────────────────────────────────────────────────┐
   │                         APPS                             │
   │  counter, poll, chat, kanban, wiki, etc.                 │
   │  Compose modules + add app-specific state-apply +        │
   │  state-propose + interaction + behavior components.      │
   └──────────────────────────────────────────────────────────┘
```

**Kernel** is the privileged layer. Owns secrets, brokers all I/O,
arbitrates every cross-component call. Compiles to native (Wasmtime
host) or browser (jco-shimmed JS+wasm host).

**Modules** are reusable WASM components encapsulating cross-cutting
concerns. They look like apps to the kernel — same WASM Component
Model, same manifest format, same distribution channel — but are
designed to be pulled in by other apps as dependencies. App authors
declare module deps in their `manifest.toml`; the kernel intersects
capability declarations and links at instantiation time.

**Apps** are user-facing bundles. They compose zero or more modules
and add their app-specific component code (state-apply, state-propose,
interaction, behavior).

The tier separation is conceptual, not enforced — modules and apps
are mechanically the same shape (WASM components with manifest +
signature). The distinction is intent: modules are designed for
reuse; apps are designed for end users.

### 2.1 Why three tiers

**Why modules and not just apps**: cross-cutting concerns
(participation enforcement, RBAC, MLS, snapshot management) recur
across many apps. Without modules, every app reinvents these
patterns. With modules, each pattern is authored once, audited
once, distributed once.

**Why modules and not kernel features**: cross-cutting concerns
evolve faster than kernel ABI. Pinning MLS in the kernel locks
Myrhiza to one MLS implementation; pinning RBAC in the kernel
locks one permission model. Modules let the ecosystem evolve
without breaking kernel ABI.

**Why kernel and not just modules**: identity custody, capability
arbitration, deterministic state-apply replay, network plumbing,
content addressing — these need privileged access to native
resources (private keys, sockets, filesystem). They cannot be
modules without breaking the sandbox model.

The three tiers correspond to three trust boundaries: kernel is
trusted absolutely; modules are sandboxed but typically authored
or audited by the project; apps are sandboxed and may come from
anywhere.

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
§7). All return values are pure functions of inputs given the event
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
`host.http`, `host.timer`, `host.sign-via-scope` (with behavior-
scoped IdentityScope; see §6).

**Identity**: per-(peer, instance). When a peer enables a behavior,
the kernel allocates a fresh IdentityScope under the peer's identity
with `instance: { peer, kind: behavior, name: <app-chosen> }`. Events
authored by the behavior are signed under this scope. The runtime
does not migrate behavior keypairs between peers; cross-peer behavior
continuity is an app-level concern (apps that need stable bot
identity across peers register an in-band mapping event).

## 4. Convergence and state-apply

**Decision**: event-log replay is the convergence paradigm.

Per-author signed Merkle DAG. Each event has author, sequence number
(monotonic per-author starting at 1), `prev` (hash of this author's
previous event), `deps` (array of cross-author causal heads), opaque
payload bytes, and Ed25519 signature.

Peers gossip events via iroh-gossip. Each peer's local DAG indexes
events by hash, by author chain, and by topic membership. Materialization =
deterministic topo-sort over the DAG, replaying each event through the
app's `state-apply` component.

### 4.1 Topological ordering

Topo-sort uses Kahn's algorithm with a `BTreeSet<&EventHash>` ready
set. Concurrent events (no causal order between them) are
tie-broken by **EventHash lexicographic byte comparison**, not by
HLC timestamp. This is content-causal-plus-lex-hash ordering.

HLC timestamps are signed into events but **not used for DAG
ordering, merge, or topo-sort tie-break**. They are materialized
into derived state where useful (channel-activity timestamps,
ephemeral idle thresholds, display ordering hints).

### 4.2 Sync protocol

`HeadsSummary`-style delta exchange. A peer's `HeadsSummary` is the
compact (author → seq + hash) map of its DAG. Peers exchange
summaries; the side with newer events streams missing events to the
side with older events.

Out-of-order delivery is buffered in a `PendingBuffer` with two
eviction policies (independent): age-based (default 1 hour TTL) and
capacity-based (default 10,000 entries with per-author sub-cap of
`max_entries / 50` to thwart Sybil-shaped flooding).

Snapshots are cached materialization; they are not authoritative.
Bootstrap = fetch a snapshot at known event hash, then catch up by
replaying events past that hash.

### 4.3 Cross-peer convergence proof

Convergence is verified by hashing each app's exported `state-digest()`
function output. The digest is canonical bytes under a deterministic
encoding using sorted collections (`BTreeMap` / `BTreeSet`). The
kernel hashes the digest output and gossips the hash; mismatches
surface as bugs.

**Why not hash WASM linear memory**: allocator behavior, struct field
padding, `HashMap` iteration order would diverge trivially across
peers. App-canonical digest is the load-bearing piece; format
selection (bincode today / postcard envisioned / other) is open and
deferred to the determinism-enforcement child spec.

### 4.4 Pre-check unification

`state-apply` runs in two modes:

- **Apply mode**: peer ingests an incoming event; runs `state-apply`
  against current state; commits new state if return is `Accept`.
- **Pre-check mode (dry-run)**: originating peer runs `state-apply`
  against a hypothetical post-state before signing the event. If
  the return is anything other than `Accept`, the kernel **rejects
  the user action and does not sign**. Pre-check fails closed.

Pre-check is **mechanically the same WASM function as `state-apply`**,
called by the kernel in dry-run mode. Not a convention. The same
deterministic helper set, the same fuel posture, the same denied
non-deterministic imports apply.

This makes pre-check / apply divergence structurally impossible.
Apps cannot accidentally let through events that other peers will
reject — there is one code path, one verdict.

### 4.5 Future direction: scaling

Event-log replay scales linearly. Every materializing peer carries
the full log for the topic. Big apps (large peer counts, large
state, public read access) hit this ceiling.

**Master spec acknowledges this as the named-but-deferred scaling
problem.** The most-likely v2+ evolution is layering DHT-shape
sharding for storage and responsibility on top of event-log canonical
ordering. Other paths are preserved:

- **Author-bounded scale only at v1**. Many apps (chat, kanban, wiki
  with bounded authors) fit comfortably on event-log replay alone.
- **Snapshot-as-bootstrap with log-pruning**. Eg-walker style log
  compaction is research-grade; log truncation past a snapshot is
  well-understood for some shapes.
- **Cooperative pinning**. Maintenance-shaped components opt-in;
  apps that need durability ship with persister modules; users who
  install run them.
- **Read-replica through a separate channel**. Read-heavy apps
  materialize from log on dedicated peers and gossip materialized
  state directly.

**Decision criteria for picking the v2+ answer**: when the first
Myrhiza app hits the scaling ceiling, measure where the actual
bottleneck is (storage cost / replay time / bandwidth / participation
enforcement) and pick the answer that addresses that bottleneck.
Don't speculatively ship sharding before the bottleneck is real.

### 4.6 CRDT use cases

Apps that want commutative-merge semantics (collaborative text,
real-time docs) embed a CRDT inside their own `state-apply`. The
kernel stays generic — it does not bake any CRDT into the event
substrate.

This is a documented usage pattern, not an architectural feature.
The CRDT's internal state must be canonically encoded (sorted
collections) for state-digest determinism. Future modules
(`myrhiza-state-crdt-{automerge,yjs,loro}`) ship the CRDT-in-state-
apply pattern as a reusable component for apps that want it.

## 5. Determinism

The convergence proof rests on three legs: content-addressed events
(per §4), deterministic topo-sort (per §4.1), and pure `state-apply`
(this section).

### 5.1 Deterministic helper set

The exact set of host imports `state-apply` may bind. Each is a
pure function of its inputs given the event payload alone. No
peer-local return values; no information about who-can-decrypt;
no clock; no randomness.

Provisional set (refined in determinism-enforcement child spec):

```wit
host.verify-signature(pubkey: list<u8>, msg: list<u8>, sig: list<u8>) -> bool
host.verify-payload-mac(envelope: list<u8>, key-handle: key-handle) -> bool
host.hash(bytes: list<u8>) -> list<u8>
host.install-key(handle: key-handle, sealed-distribution-blob: list<u8>) -> ()
host.now-hlc-from-event(event-bytes: list<u8>) -> hlc
host.log(level: log-level, msg: string) -> ()
```

Notes:

- `host.install-key` returns `()` deliberately. A boolean indicating
  "this peer can decrypt" would peer-locally branch state-apply,
  breaking determinism. Whether this peer can use the key is queried
  separately from interaction profile via `host.can-open(handle)`.
- `host.verify-payload-mac` proves *key possession*, not *author
  identity*. Author identity comes from the outer Ed25519 signature
  on the event itself.
- `host.now-hlc-from-event` extracts the HLC from the event envelope.
  It does not consult the wall clock.

### 5.2 Denied imports for state-apply

- No wall clock. No randomness. No network. No filesystem. No
  environment. No threads.
- No floats at v1 (per §3.1). State-apply WASM modules importing or
  using float ops are rejected at component install time.
- No SIMD-float ops even if floats are eventually allowed; cross-platform
  divergence vectors.
- No nondeterministic instructions (e.g. `now-from-host-clock`).

### 5.3 Fuel and resource limits

Instruction-count fuel budget per state-apply invocation. Running
out terminates uniformly across peers. Default values are deferred
to a determinism-enforcement child spec; they should be generous
enough that legitimate apps do not hit the limit and tight enough
that adversarial apps cannot DoS the kernel.

Pre-check (§4.4) shares state-apply's fuel budget per invocation.
Whether pre-check has its own per-event budget separate from apply
is an open child-spec question.

Memory caps per component instance are also enforced. Defaults
deferred.

### 5.4 Encoding for state-digest

Apps export `state-digest()` returning canonical bytes for
cross-peer convergence verification. The encoding must be
deterministic.

**v1 default**: `bincode` over `BTreeMap` / `BTreeSet` (matches
Willow's shipped precedent). The format choice is open; what is
load-bearing is the **sorted-collection discipline**, not the
specific format.

PR #636 proposed `postcard` as the canonical form going forward.
That proposal is forward-looking, not historical. The
determinism-enforcement child spec picks the format; the master
spec commits the discipline.

## 6. Identity primitive

A single kernel primitive covers user identity, multi-device
identity, behavior identity, and MLS LeafNode identity.

### 6.1 IdentityScope

```wit
record identity-scope {
    long-term: identity-handle,
    instance: option<instance-binding>,
}

record instance-binding {
    peer: peer-handle,
    kind: instance-kind,
    name: string,
}

variant instance-kind {
    device,
    behavior,
    mls-leaf,
    custom(string),
}
```

`long-term` is the durable user/author/member identity. `instance`,
when present, is the short-lived per-(peer, instance) signing scope
nested under that long-term. `instance: none` means the operation is
performed by the long-term identity directly.

The kernel custodies all private keys. Components see only opaque
`identity-handle` and scope handles. To sign, components call:

```wit
host.sign-via-scope(scope: identity-scope, msg: list<u8>) -> sig
```

The kernel verifies the calling component is authorized to use the
scope (per §7), looks up the appropriate private key, signs, and
returns the signature. Private keys never enter component memory.

### 6.2 Use cases under one primitive

| Use case | long-term | instance |
|---|---|---|
| User signing (single device) | User Ed25519 | none |
| User multi-device | User Ed25519 | `kind: device, name: "laptop"` |
| Behavior bot | Owner Ed25519 | `kind: behavior, name: "discord-bridge-1"` |
| MLS member, current epoch | Member Ed25519 | `kind: mls-leaf, name: "epoch-42"` |
| App author signing a release | Author Ed25519 | none |

**Cremers ETK 2025 constraint**: `long-term` MUST use Ed25519 (which
is SUF-CMA secure). ECDSA is EUF-CMA only and fails MLS FCGKA security.
This applies even to non-MLS scopes for forward compatibility.

### 6.3 Deferred to child specs

- Device-add and device-revoke flow (multi-device).
- MLS LeafNode lifecycle integration (`myrhiza-crypto-mls` module).
- Recovery semantics when long-term key is lost.
- Cross-peer behavior continuity (apps that want stable bot identity
  across peers register an in-band mapping event mapping
  peer-side behavior keypair to an app-level role; enforced by the
  app's own pre-check).
- Quantum-safe signature migration when post-quantum schemes mature.

## 7. Capability model

Three layers of gating, each at a different boundary, plus
typed resource handles for non-forgeable inter-component refs.

### 7.1 App boundary — manifest declares ambient set

Every app's `manifest.toml` declares its ambient capability set:
which host imports it may call, which UI surfaces it may bind, which
modules it depends on. The manifest is signed (per §10) so the
declared set cannot be modified after publication.

At install time, the kernel renders a capability summary to the user
(bech32m-encoded author identity, version, declared capabilities).
The user confirms or rejects.

### 7.2 Module boundary — manifest intersection at link time

When an app declares a module dep, the module brings its own
capability declarations (what host imports it requires to function).
The kernel **intersects** the app's ambient set with the module's
required set at component instantiation:

```
M_effective = A_ambient ∩ M_required
```

A module can never exceed the calling app's grants. An app cannot
grant a module more than the module declared needing. This catches
both directions: malicious modules declaring excessive imports, and
apps trying to over-grant.

If the intersection is empty for a required capability — i.e. the
module needs something the app didn't declare — installation fails
with a precise error.

### 7.3 Per-call gating on high-value ops

Specific privileged operations are re-checked at every call against
the **calling component's** manifest:

- Clipboard write
- File picker invocation
- Top-level navigation
- Push notification registration
- AEAD seal/open with sensitive keys
- Network egress to specific origins (when interaction calls out to
  third-party services)

The list is curated; what counts as "high-value" is a child-spec
concern. The mechanism is uniform: the WIT contract for each such op
includes a per-call gate annotation; the kernel reads the calling
component's manifest at invocation time and rejects calls that
exceed its declared scope.

This catches social-engineering attacks: an interaction component
asking a UI module to write to clipboard cannot escalate beyond the
*interaction component's* clipboard grant, even if the UI module
itself has clipboard access.

### 7.4 Resource handles for non-forgeable refs

WASM Component Model resource handles (free from §8's full-CM ABI
choice) are the unit of explicit capability transfer. Apps pass
scoped handles to modules to grant fine-grained access:

```wit
// app passes only this private channel handle to module M
let channel-handle = host.create-private-channel();
my-module.process(channel-handle);
```

The module cannot forge equivalent handles. It can only use what
was passed.

This pattern complements §7.1–7.3: ambient grants set the bounds,
intersection scopes the module, per-call gates protect high-value
ops, and resource handles enable explicit fine-grained transfer.

### 7.5 Defense in depth

The four layers catch different attack classes:

| Attack | Caught by |
|---|---|
| Malicious app over-declares capabilities | User reviews at install (§7.1) |
| Malicious module declares more than it needs | Manifest intersection (§7.2) |
| Compromised module attempts privilege escalation | Manifest intersection + per-call gate (§7.2 + 7.3) |
| Module forges capability ref | Resource handle non-forgeability (§7.4) |
| Social engineering across components | Per-call gate (§7.3) |

The cost of all four is small — manifest schema + intersection check
at install; resource handles come free with full CM; per-call gating
is a WIT annotation. The benefit is comprehensive containment of
modules, which is essential because modules are pulled in by apps and
may come from third parties.

## 8. ABI and composition

### 8.1 Decision

**Full WebAssembly Component Model from day one**. WIT-bindgen for
the SDK (Rust). Wasmtime native runtime. jco-transpiled glue + core
wasm in browser.

### 8.2 Why full CM and not Extism

PR #636 leaned Extism v1 → CM v2 for ship-faster reasoning. Myrhiza
rejected this:

- Extism cannot express WCM resource handles, borrows, world
  composition, or futures/streams. Migration is a real refactor for
  app authors — not a regenerate-bindings event.
- Every Myrhiza app and module written before migration would be
  rewritten. Including Willow when it eventually refactors. Double-
  rewrite cost is unacceptable.
- We do not have a chat-product-keep-alive deadline that justified
  PR #636's ship-faster framing.
- Submit-and-poll (§8.5) gives us sync-ABI ergonomics regardless;
  full CM does not lose anything to Extism on that axis.

### 8.3 Cross-component composition

Components compose via typed WIT resource handles. A module exports
a WIT interface; an app (or another module) imports it. Resource
handles are non-forgeable refs (§7.4). The kernel arbitrates every
cross-component call.

```
component A imports: my-app:counter
                          ↓
component B exports: my-app:counter  (the counter app)
```

`wac` (WCM composition tool) is supported for build-time composition.
Runtime composition is also supported through the kernel's component
instantiation pathway — apps may load module components dynamically
based on user choice.

### 8.4 No cross-component shared memory

No direct memory sharing between components. Every interaction is
typed, bounded, and refusable. The kernel is the call broker.

### 8.5 Submit-and-poll for inherently async surfaces

Browser jco preview2 does not support async at the WIT boundary.
state-apply is sync by definition. Kernel calls that wrap async
surfaces (gossip broadcast, blob fetch, HTTP, persistent KV, timers)
follow a submit-and-poll pattern:

```
let token = host.network.broadcast-submit(topic, msg);
// later, kernel calls back into a profile-appropriate handler
on-completion(token, result);
```

The component returns immediately; the kernel re-enters via an
exported `on-completion(token, result)` handler when the operation
finishes. Back-pressure is preserved (a slow operation does not stall
the component's actor mailbox).

When jco preview3 stabilizes async at the WIT boundary, the
kernel-side adapter migrates without API churn for app authors.

### 8.6 Coarse-grained interfaces

Interaction components return view models in per-surface units (one
channel timeline, one member list, one composer state). Returns are
version-tagged so the host can skip recomposition on no-op state
changes; large lists are paged. Behavior components observe and emit
in batches.

No tight inner-loop callbacks across component boundaries. Cross-
component calls have measurable cost (component instantiation, ABI
translation, capability gate check); coarse granularity amortizes
the cost.

## 9. Crypto primitives and key custody

### 9.1 Kernel custody

All secret material lives in the kernel:

- Private signing keys (per IdentityScope, §6).
- Symmetric channel/group keys.
- Ratchet state.
- MLS group state when adopted.

Components hold opaque handles to keys; the kernel custodies bytes.
Secrets do not enter component memory in their raw form.

### 9.2 Primitive crypto host imports

Provisional WIT contract (refined in crypto-and-key-custody child
spec):

```wit
// signature primitives
host.sign-via-scope(scope: identity-scope, msg: list<u8>) -> sig
host.verify-sig(pubkey: list<u8>, msg: list<u8>, sig: list<u8>) -> bool

// key agreement
host.x25519-ecdh(scope: identity-scope, peer-pubkey: list<u8>) -> secret-handle

// key derivation
host.hkdf-derive(input: secret-handle, info: list<u8>, length: u32) -> secret-handle

// authenticated encryption
host.aead-seal(key: secret-handle, nonce: list<u8>, plaintext: list<u8>, ad: list<u8>) -> list<u8>
host.aead-open(key: secret-handle, nonce: list<u8>, ciphertext: list<u8>, ad: list<u8>) -> result<list<u8>, error>

// hashing (also a deterministic helper for state-apply per §5.1)
host.hash(bytes: list<u8>) -> list<u8>
```

Algorithm choices: Ed25519 (signing), X25519 (ECDH), ChaCha20-Poly1305
(AEAD), HKDF-SHA256 (KDF), BLAKE3 (hashing). Provisional; final
selection is in the crypto child spec.

### 9.3 MLS as a module

The official `myrhiza-crypto-mls` module ships as Myrhiza's canonical
group encryption solution when the first MLS-needing app emerges.
The module implements RFC 9420 entirely in WASM, calling the kernel
crypto primitives for cryptographic operations and the kernel
IdentityScope primitive for member/leaf signing keys.

Kernel does not bake any specific MLS implementation. Module authors
may compete; users may choose alternatives. Post-quantum migration
is a module swap, not a kernel ABI change.

The kernel-baked MLS path (PR #636's `host.mls.*` host imports)
remains open as a future-additive ABI change if module-based MLS
proves insufficient. v1 commits the module path.

### 9.4 Other crypto modules

Common patterns ship as additional modules:

- `myrhiza-crypto-channel-key` — symmetric channel-key encryption
  (Willow-shape).
- `myrhiza-crypto-double-ratchet` — Signal-style DM ratchets when DM
  apps emerge.
- `myrhiza-crypto-sealed-content` — NIP-44/59-shape sealed payloads.

All compose the same primitive crypto host imports.

## 10. Apps, modules, and bundle distribution

### 10.1 Bundle shape

Apps and modules use the same shape:

```
bundle/
├── manifest.toml          author pubkey + version + capabilities
│                          + module deps + signature
├── components/
│   ├── state-apply.wasm
│   ├── state-propose.wasm
│   ├── interaction.wasm
│   └── behavior.wasm      (optional)
├── ui-assets/             (optional; static UI assets if present)
└── signature              Ed25519 over (manifest_hash + content_hash
                           + version + author_pubkey)
```

Modules use the same shape but may not include `state-propose` or
`behavior` profiles depending on what they implement.

### 10.2 Manifest schema (provisional)

```toml
[app]
name = "counter"
version = "0.1.0"
author-pubkey = "wpeer1abc..."     # bech32m-encoded Ed25519 pubkey
description = "Simple shared counter"

[capabilities]
host-imports = [
    "host.sign-via-scope",
    "host.broadcast",
    "host.subscribe",
    "host.kv",
]
ui-surfaces = ["ui:panel", "ui:button"]
high-value-ops = []                # explicit opt-in for things like clipboard

[modules]
"myrhiza-permission-rbac" = "1.0.0"
"myrhiza-state-snapshot-cache" = "0.5.0"
```

Schema evolution is a child-spec concern. Manifest TOML format is
human-readable and tooling-friendly.

### 10.3 Distribution

Bundles distributed via iroh-blobs by content hash. No central
registry. Discovery is out-of-band at v1: hashes shared via links,
QR codes, in-app share. Future-direction (deferred to child spec):
in-band catalog gossip for app/module discovery.

### 10.4 Signing

Author Ed25519 signs `(manifest_hash + content_hash + version +
author_pubkey)`. The signature is part of the bundle. The author
public key is embedded in the manifest.

Author identity reuses the IdentityScope primitive (§6). App
authors are users; user signing keys can sign app releases.
Production-grade authors typically use a separate IdentityScope
long-term identity for releases (separation of concerns).

### 10.5 Install flow

1. User receives bundle hash via out-of-band channel.
2. Kernel fetches bundle via iroh-blobs by hash.
3. Kernel verifies Ed25519 signature against author pubkey embedded
   in manifest.
4. Kernel renders capability summary: bech32m-encoded author
   identity, version, declared capabilities, module deps.
5. User confirms or rejects.
6. Kernel resolves module deps (recursively fetches and verifies),
   intersects capability declarations (§7.2), instantiates the app's
   components.

### 10.6 Versioning

Semver. Bundle hash changes per version (content-addressed). New
versions are new hashes; users opt into upgrades. The manifest
declares its semver version; module deps pin versions.

### 10.7 Revocation

Author retracts a bad version by publishing a revocation event signed
under the same author IdentityScope. Revocation distribution
mechanics are deferred to a child spec.

### 10.8 No central registry

No Myrhiza-operated registry. No sigstore dependency. No reliance on
any centralized service for app distribution. P2P-native distribution
is non-negotiable; matches iroh-blobs commitment and the project's
no-deployed-infrastructure framing.

## 11. Networking, sync, and relays

### 11.1 Transport

iroh — gossip, content-addressed blob fetch, dial-by-pubkey QUIC,
DERP-style relay-bridged NAT traversal. The locked load-bearing
transport dependency.

The kernel exposes a narrow networking surface to apps via
capability-gated host imports (broadcast, subscribe, blob fetch).
Apps do not see iroh directly.

### 11.2 Topic membership

Apps subscribe to topics. A topic is a content-addressed identifier
(BLAKE3 of canonical app identity + topic-app-state + topic name).
Membership in a topic = the peer is gossiping events on that topic.

### 11.3 Sync protocol

`HeadsSummary` delta exchange, per §4.2. Future work:
`HistorySyncComplete` EOSE-style signal so peers know when backfill
finished (Willow precedent); negentropy-shape range reconciliation
for very large topics (deferred).

### 11.4 Relays

Relays are dumb topic bridges. They do not inspect payloads, do not
materialize state, do not run WASM. Their role is to bridge browser
peers (TCP/WebSocket) with iroh-native peers (QUIC), and to provide
NAT-traversal hole-punching assistance.

A peer that wants to act as a relay for an app must be granted
explicit permission (via the app's authority model — usually a
`SyncProvider`-shape grant). Without permission, the peer functions
as a regular peer, not a relay.

### 11.5 Topic-ID rotation through dumb relays

Apps that rotate topic IDs (e.g. for unlinkability via Willow's
epoch-key-rotation pattern) face a coordination problem: how do
existing members tell the relay where the next topic lives without
publishing the next topic ID on a public channel.

The kernel is not in this loop. Apps coordinate rotation through
in-band events on the existing topic before rotation. The exact
protocol is deferred to the relay-and-rotation child spec.

### 11.6 Browser peer connectivity

Browser peers connect via iroh-relay-bridged QUIC. Pure-browser
WebTransport-as-iroh-transport is not a current path (it would defeat
dial-by-pubkey identity). WebRTC is a Holochain-tx5-shape detour we
do not pursue.

## 12. Maintenance and participation

### 12.1 No worker class

There is no "worker" as a peer-class. Every peer participating in an
app can perform maintenance work for that app — that is what
participation means. Some peers contribute more (operator-deployed
infrastructure, dedicated archival peers); some contribute less
(mobile clients on metered connections); most do the default amount
automatically.

### 12.2 Maintenance modules

Maintenance work is encapsulated in **maintenance-shaped modules**
(WASM components in the module ecosystem). Common shapes:

- Persister (durable storage of event log).
- Snapshot provider (cached materialization for fast bootstrap).
- Sync provider (serves events to peers behind on heads).
- Replay buffer (recent-events cache for fast catch-up).

Maintenance modules use the standard module-ecosystem distribution +
signing + capability gating mechanism (§10, §7).

### 12.3 Default client behavior

Peers participating in an app automatically instantiate cheap
maintenance modules for that app (sync provider, replay buffer
scoped small). Expensive modules (full archival persister,
dedicated relay) are gated by per-app user UI: "How much do you
want to contribute to this app?"

### 12.4 Operator-deployed infrastructure

An operator may run a peer configured with all maintenance modules
instantiated. This peer is not architecturally distinct from a user
peer — it is a peer that opted into more modules. It must be
invited into the social graph of any app it serves (see §12.5);
without invitation, the participation primitive may refuse to
route work to it.

This preserves the Willow pattern (deployed relay / replay /
storage workers) as one valid deployment shape, but not the only
one.

### 12.5 Sybil-resistant participation

The primary direction is **social-graph Sybil resistance**:
leverage apps' existing permission/invite trust graphs. A peer
contributing maintenance work to an app must be inside that app's
trust graph; fake identities not invited cannot inject themselves.

The participation primitive is itself a module:

- `myrhiza-participation-social-graph` (primary direction)
- `myrhiza-participation-tit-for-tat` (bandwidth-bound roles)
- Other variants as warranted

Apps choose modules based on threat model. Apps without a membership
model (anonymous bulletin boards) use alternate modules or accept
non-Sybil-resistant participation.

### 12.6 Anonymous participation

Excluded by social-graph approach. Apps that need anonymous
contributors use different modules (tit-for-tat for bandwidth
reciprocity; storage proofs for high-stakes durable data) or
accept the threat-model implications.

### 12.7 Future research direction

Master spec acknowledges these as named-but-deferred:

- What maintenance modules ship as official `myrhiza-*` modules first?
- Default-instantiation heuristic for cheap-vs-expensive triage.
- Capability advertisement (peer signals "willing/able to host module
  X") — operator-config at v1; in-band gossip future.
- Resource limit defaults (fuel + memory per maintenance module
  instance).
- Fair-share scheduling between topics on a single peer.
- Reputation aggregation as overlay on social-graph.
- Bridge between operator-deployed-infrastructure and social-graph
  invitation discipline.

v1 ships zero maintenance modules. The framework is named; the
implementation lands when the first scaling demand emerges.

## 13. UI: framework, not app substrate

PR #636 framed the UI surface as "another app." Master spec adopts
the framing with explicit honesty about its caveats.

### 13.1 UI as app

The default UI app is shipped in-tree (initially `myrhiza-ui-leptos`
for Leptos-based browser/native UI). It exports `ui:*` interfaces
(panel, list, message, form, menu, button, input, ...) that other
apps' interaction components import.

Other UI apps may be authored:

- `myrhiza-ui-tui` — terminal, ratatui rendering.
- `myrhiza-ui-mcp` — agent host, structured-data rendering for an
  LLM.
- `myrhiza-ui-mobile-native` — Compose/SwiftUI shell, future.
- `myrhiza-ui-dioxus` — when Dioxus Blitz matures.

### 13.2 UI app capability surface

A UI app must bind a broad capability surface (DOM, focus, IME,
clipboard, file pickers, navigation, viewport, push, IndexedDB,
service workers, drag-and-drop on web; equivalent on native). The
master spec acknowledges:

- The default UI app is privileged. It is in the TCB for its own
  chrome and DOM.
- The default UI app is **not** in the TCB for arbitrary callers'
  intents. Per-call gating (§7.3) protects against caller social
  engineering.

The "UI is just another app" framing is honest only when the UI
app's privilege is bounded by the runtime. Per-call gating bounds
it.

### 13.3 Custom-pixel surfaces

Whiteboards, code editors, network-graph visualizers, 3D voice
rooms, custom physics — these need custom-pixel control beyond
what `ui:*` interfaces express. Solution:

- On web: sandboxed iframe with postMessage protocol (kernel-mediated
  capability).
- On native: platform-specific equivalent.
- On TUI / MCP: rendered as "unavailable on this surface."

The escape hatch is web-shaped on purpose. GPU-driven UI substrates
(e.g. Bevy as a surface plugin once its web tooling matures) compose
here, not as replacements for the default UI app.

### 13.4 ui:* WIT contract

The `ui:*` interfaces are an interaction contract for app-to-UI
integration, not a portable UI substrate. They define how an
interaction component declares the views it wants rendered, the
commands it accepts, and the contextual integration points it offers
— not how those views are painted.

Each UI app implements the contract in its own idiom. Reusing one
set of interaction components across UIs is the goal; UI apps that
do not export an interface (e.g. a TUI without `ui:rich-card`) cause
graceful degradation, not breakage.

The exact `ui:*` contract is a child-spec concern.

## 14. Browser and native: dual-stack at v1

### 14.1 Decision

Both Wasmtime native and jco browser ship at v1. Kernel internals
abstract over a stable internal trait that both backends satisfy.

### 14.2 Wasmtime native

Default backend on macOS / Linux / Windows. Mobile (iOS / Android)
uses Wasmtime's AOT path (cranelift + winch baseline compiler) since
iOS prohibits JIT.

WebView desktop wrappers (Tauri, Wails) embed the native kernel
binary, not the browser kernel. This preserves native iroh transport
in the desktop app.

### 14.3 jco browser

Browser path. jco preview2 is the v1 target; preview3 when stable
migrates in-place (no API churn for app authors).

Constraints:

- Sync ABI only at preview2. Submit-and-poll (§8.5) is the workaround.
- ~350KB JS shim floor accepted as the cost of browser parity.
- Browser peers use iroh-relay-bridged QUIC for connectivity.

### 14.4 Why dual-stack at v1

- Browser is the project's pitch surface. Native-only-v1 undersells.
- "v1.5 fast-follow" framing for browser risks indefinite slip.
- Architecture pressure on backend abstraction is healthy from day
  one (avoids painful retrofit).
- Willow refactor onto Myrhiza targets v1; Willow is browser-shipped.

### 14.5 Native ≠ trusted-Rust apps

Critical clarification: "native" means the kernel runs as a native
Rust binary. Inside that kernel, **apps still run as WASM components
via Wasmtime**. The sandbox model requires WASM execution on every
backend. Compiling apps to native code for performance is explicitly
rejected — the only way to guarantee "WASM code can never access
more than what it's granted" is to run everything through the WASM
execution environment.

Performance trade accepted: ~2–5% Wasmtime overhead vs native code.
Sandbox is non-negotiable; this overhead is the cost of the security
model.

## 15. MVP

### 15.1 Acceptance criteria (lifted from PR #636)

The MVP must demonstrate:

1. The kernel loads and instantiates a WASM state component from a
   bundle fetched via iroh-blobs.
2. The component applies events deterministically; multiple peers
   running the same component bytes converge to the same state hash
   (verified via `state-digest`).
3. A UI app loads an interaction component for that state, projects
   a view, submits a command, observes the resulting state change.
4. A second app instance (different state component, different
   topic) coexists on the same peer; events do not cross.
5. Capability declarations actually gate access — a component cannot
   import an interface its manifest does not declare.
6. A behavior component runs on a designated peer, observes events,
   and logs them.

### 15.2 MVP shape: counter + poll

Two minimal apps coexisting in the same kernel.

**Counter app**:
- State: `{ value: u64 }`
- Events: `Increment(by: i32)`, `Decrement(by: i32)`, `Reset`
- Permission gate on `Reset` (admin only).
- Optional behavior component: `auto-reset-at-midnight` running on a
  designated peer (acceptance criterion #6).

**Poll app**:
- State: `{ options: Vec<String>, votes: Map<peer, option_index>,
  ended: bool }`
- Events: `CreatePoll(options)`, `Vote(option_index)`,
  `EndPoll(creator-only)`
- Permission gate on `EndPoll` (only poll creator).

Both apps live in `examples/counter/` and `examples/poll/`. Each ~50–
150 LOC state-apply + ~100–200 LOC interaction. Total ~300–700 LOC
across both apps + manifests.

### 15.3 Test infrastructure

Multi-tier test hierarchy lifted from Willow:

- **State tier** (instant): unit-test each app's state-apply directly
  with crafted events. No kernel, no I/O.
- **Kernel tier** (fast): kernel + MemNetwork + apps in-process.
  Verifies per-app namespace, convergence, capability gating.
- **E2E tier** (slow): real iroh transport, two peer processes on
  loopback or two machines.
- **Browser tier** (slow): jco-shimmed kernel, headless Firefox,
  multi-tab convergence.

Test files live in `tests/{unit, integration, e2e}/`. The
`coexistence.rs` e2e test is the load-bearing acceptance test —
both apps in same kernel, no event-crossing, capability gating
verified.

### 15.4 Workspace shape

```
myrhiza/
├── Cargo.toml                 workspace root
├── crates/
│   ├── kernel/                runtime (host)
│   ├── sdk/                   app-author surface (state-apply / propose
│   │                          / interaction macros, manifest tools)
│   ├── network/               iroh wrappers, network trait
│   ├── storage/               event log, snapshot cache
│   ├── crypto/                primitive crypto host imports
│   └── ...
├── examples/
│   ├── counter/               wasm32 component, depends on sdk
│   │   ├── Cargo.toml
│   │   ├── manifest.toml
│   │   └── src/{state, propose, interaction, behavior}.rs
│   └── poll/                  same shape
└── tests/
    ├── unit/                  per-crate
    ├── integration/           kernel-with-MemNetwork, single peer
    └── e2e/
        ├── counter.rs
        ├── poll.rs
        ├── coexistence.rs     ⭐ load-bearing acceptance test
        ├── multi_peer_convergence.rs
        └── capability_gating.rs
```

**Dependency direction** (load-bearing constraint): `examples/` →
`crates/sdk`. Kernel crates **never** depend on examples. Examples
never appear in `crates/`. Violation = bug.

### 15.5 Estimated v1 scope

Counter + poll MVP as scoped above is approximately:

- ~16-20 weeks engineering effort (dual-stack v1 with both Wasmtime
  and jco backends).
- Critical path items: kernel core; state-apply ABI; manifest schema;
  capability gating; iroh network adapter; jco backend abstraction;
  IdentityScope primitive; signing + bundle distribution; counter +
  poll example apps; e2e test coexistence.cs.
- Deferred from v1: maintenance modules; MLS; multi-device flow;
  scaling solutions; behavior component (acceptance criterion #6 may
  ship as v1.1 if it slips).

## 16. Migration: Willow → Myrhiza

Willow continues to develop independently. Eventually, Willow refactors
onto Myrhiza — chat becomes one app among many on the runtime, the
Leptos web client becomes a `myrhiza-ui-leptos` instance, and Willow's
worker binaries (replay, storage, relay) become maintenance modules.

The migration is not a fork. Willow's existing codebase ships chat to
users; Myrhiza is a separate runtime project. Decisions in Willow
inform Myrhiza (the prior-art folder `prior-art/willow/` captures the
mapping); Myrhiza decisions are made fresh, re-evaluating each Willow
choice rather than blindly inheriting.

When Willow refactors onto Myrhiza, the chat product becomes
`willow-chat` — a Myrhiza app. Its state-apply contains the chat
semantics that today live in `willow-state`. Its interaction component
consumes the `myrhiza-ui-leptos` UI app. Its identity, encryption, and
permission concerns use Myrhiza primitives + modules
(IdentityScope, `myrhiza-permission-governance`,
`myrhiza-crypto-channel-key` or future `myrhiza-crypto-mls`).

Migration timing: target v1 (browser available from v1 ship).
Migration mechanics are a separate spec when Willow is ready.

## 17. Future-direction items (named-but-deferred)

The master spec commits the *direction* on these items so v1 design
does not paint corners. Implementation lands in child specs when
demand emerges.

### Scaling
- Event-log replay scales linearly. Likely v2+ evolution: DHT-shape
  sharding layered on top. Other paths preserved (cooperative
  pinning, log-pruning, derived-state replication). Decision criteria:
  measure the bottleneck before committing.

### Distributed maintenance
- Default-instantiation heuristic for cheap-vs-expensive maintenance
  modules.
- Capability advertisement (peer signals "willing/able to host module
  X") — operator-config at v1; in-band gossip future.
- Resource limit defaults.
- Fair-share scheduling between topics on a single peer.
- Bridge between operator-deployed-infrastructure and social-graph
  invitation discipline.

### Identity
- Multi-device device-add/revoke flow.
- Recovery semantics (lost device).
- Cross-peer behavior continuity.
- Quantum-safe signature migration.

### Crypto
- `myrhiza-crypto-mls` module (when first MLS-needing app emerges).
- Other crypto modules (channel-key, double-ratchet, sealed-content).
- Quantum-safe primitives.

### Capability model
- High-value-op list for per-call gating.
- Cross-app authority composition (out of scope at v1).
- Capability vocabulary in manifest schema.

### Distribution
- Bundle revocation (author retracts bad version).
- In-band catalog gossip for app/module discovery.
- Supply-chain hardening (dependency review tooling).

### Networking
- Topic-ID rotation through dumb relays (relay-and-rotation child
  spec).
- `HistorySyncComplete` EOSE-style signal for backfill completion.
- Negentropy-shape range reconciliation for very large topics.

### Determinism
- Float opt-in path (manifest `state-apply.allow-floats = true`).
- Snapshot portability across component-version upgrades.
- State-digest encoding format selection (bincode vs postcard vs
  other).
- Pre-check fuel budget independence from apply.

### Interaction
- `ui:*` WIT contract details.
- Custom-pixel surface escape hatch on non-web platforms.
- Hot-reload (deferred to v2).

### Module ecosystem
- Versioning + semver discipline child spec.
- Bus-factor on official `myrhiza-*` modules.
- Module audit / curation policy.

## 18. Tradeoffs surfaced

| Decision | Runner-up | Why rejected |
|---|---|---|
| Event-log replay (paradigm) | Validating DHT (Holochain) | 6+ year unfinished sharding story; throws out Q3 progress |
| Full Component Model day-one | Extism v1 → CM v2 | Double-rewrite cost for app authors and Willow |
| Dual-stack v1 | Native-first + browser v1.5 | v1.5 slip risk; browser is project pitch |
| Module ecosystem (3-tier) | Kernel-baked features | Module-level evolution faster than kernel ABI; vendor lock-in avoided |
| Layered cap gating | Per-call only | Modules need containment; layered defense in depth |
| `IdentityScope` unified | Separate per-domain | Triple design + impl cost; PR #636 names structural similarity |
| MLS as module | Kernel-baked MLS | Vendor lock-in (one impl); kernel surface bloat; PQ migration kernel break |
| Ed25519 + iroh-blobs (P2P) | OCI + sigstore | Centralizes what we made P2P; Sigstore Public Good single point |
| Float ban in state-apply v1 | Spec-pinned floats | NaN canonicalization + SIMD divergence vectors; debugging painful |
| WASM on every backend | Native compilation for performance | Defeats sandbox model; capability discipline requires WASM execution |
| No worker class | Worker-as-peer-class | Architecturally honest; doesn't close paths; v1 ships zero |
| Counter + poll MVP | Single app or chat MVP | Coexistence (#4) requires two apps; chat recapitulates Willow |

## 19. Open questions / accepted risks

- **MLS performance under WASM**: ~2-5x slower than native MLS for
  group operations. Acceptable because group ops are not hot-path,
  but worth measuring before scale.
- **jco preview2 sync-only ABI**: submit-and-poll is the workaround;
  ergonomics are real. Preview3 async stabilization improves this
  but does not change v1.
- **Browser CM nested in browser-WASM (Leptos web app loading nested
  CM components)**: not battle-tested at scale. Risk of weird
  bugs. Mitigation: counter + poll MVP exercises this path early.
- **Author key compromise**: phishing-shape attack surface. Mitigated
  by user-visible bech32m author identity at install + revocation
  pattern in child spec.
- **Anonymous participation excluded by social-graph Sybil
  resistance**: documented; apps that need anonymity use other
  modules.
- **No sigstore transparency log**: trust comes from author identity
  + user judgment. Trade accepted; matches P2P framing.
- **Operator-deployed infrastructure + invitation flow**: needs
  explicit social-graph integration, not yet specified.
- **Module ecosystem bus factor**: official `myrhiza-*` modules we
  author, we maintain. Mitigated: encourage third-party alternatives.
- **Browser peer story is relay-bound**: WebTransport-as-iroh-transport
  not a current path; WebRTC not pursued. Browser peers depend on iroh
  relays.
- **Cremers ETK 2025 enforcement**: Ed25519 mandatory for IdentityScope
  long-term identity. Lint check at component install.

## 20. Implementation outline (handed off to writing-plans)

Implementation plan lives at
`docs/plans/2026-05-09-myrhiza-master-design.md`. High-level critical
path:

1. **Workspace scaffold** + initial crate structure (`kernel/`,
   `sdk/`, `network/`, `storage/`, `crypto/`, `examples/counter/`,
   `examples/poll/`, `tests/`).
2. **Core types**: `IdentityScope`, `IdentityHandle`, `EventHash`,
   `Event`, `Topic`, `BundleHash`, manifest schema types.
3. **Wasmtime backend**: kernel runs a Wasmtime host; component
   instantiation; capability-gated host import dispatch.
4. **State-apply ABI**: WIT for state-apply profile; deterministic
   helper set host imports; fuel budget; float ban lint.
5. **Event/DAG primitives**: per-author Merkle DAG storage;
   topo-sort; PendingBuffer.
6. **iroh integration**: gossip wrapper, blob fetch wrapper, network
   trait abstraction (production iroh + MemNetwork test double).
7. **HeadsSummary sync**: protocol implementation.
8. **Crypto primitives**: host imports backed by Rust crypto crates
   (ed25519-dalek, x25519-dalek, chacha20poly1305, hkdf, blake3).
9. **Manifest + capability gating**: TOML parser; intersection check
   at instantiation; per-call gate plumbing.
10. **Bundle distribution + signing**: Ed25519 over manifest+content
    hash; iroh-blobs publication and fetch.
11. **Counter app**: state-apply, propose, interaction, manifest.
12. **Poll app**: same shape.
13. **State-tier tests** for both apps.
14. **Kernel-tier tests** (kernel + MemNetwork): convergence,
    capability gating, coexistence.
15. **E2E test suite**: counter, poll, coexistence, multi-peer
    convergence, capability gating.
16. **jco backend abstraction**: kernel internals split into Wasmtime
    backend + jco backend behind a stable internal trait.
17. **jco backend implementation**: generate JS+wasm shim; iroh-relay
    bridge for browser transport.
18. **Browser-tier tests**: headless Firefox, multi-tab convergence.
19. **SDK ergonomics**: macros and tooling for app authors.
20. **Optional v1.1**: counter app's auto-reset behavior component
    (acceptance criterion #6).

## 21. Sources

- All 12 prior-art folders under `docs/prior-art/`:
  agoric-endo, crdts, croquet, holochain, iroh, mls, pears, spin,
  spritely-ocapn, wasmcloud, wasm-component-model, willow.
- `docs/references/local-first.md` — anchor index of papers and
  talks.
- `docs/reports/2026-05-09-myrhiza-design-space/` — preparation
  phase exploration:
  - `README.md` (master question list)
  - `convergence-and-determinism.md`
  - `wasm-and-abi.md`
  - `identity-crypto-caps.md`
  - `networking-sync-maintenance.md`
  - `ui-distribution-mvp.md`
  - `brainstorming-decisions.md` — running decision log
- Willow PR #636 (master runtime spec, draft):
  [github.com/intendednull/willow/pull/636](https://github.com/intendednull/willow/pull/636)
- Willow repo: [github.com/intendednull/willow](https://github.com/intendednull/willow)
- `CLAUDE.md` — Myrhiza dev guide (locked decisions inventory).
