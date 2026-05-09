**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — PR #636 master runtime spec, framed for Myrhiza adoption

This file is a curated reading of Willow PR #636 ("docs(runtime): master
spec for Willow App Runtime", branch `claude/wasm-plugin-system-WyY1p`,
**draft, not merged**). PR #636 is the Myrhiza proto-spec — the moment
Willow stopped being chat and reframed itself as a P2P app runtime.
Read this before any Myrhiza spec that touches kernel/app boundaries.

See also: [README.md](README.md), [lessons.md](lessons.md),
[open-problems.md](open-problems.md), [state-machine.md](state-machine.md),
[determinism.md](determinism.md), [authority.md](authority.md).

## The reframe

The conversation that produced PR #636 started as "a WASM plugin system
for Willow." That framing was rejected mid-conversation as "too small."
A plugin system implies a host application with a fixed feature set,
optionally extensible. Willow's destination is the inverse: a **small
kernel where the application itself is a composition of typed,
sandboxed, content-addressed components**.

Chat is not a feature of Willow that has plugins. Chat is one app
running on Willow. Wikis are another. So is a kanban board. **Myrhiza
inherits this reframing whole** — Myrhiza is the runtime; Willow's chat
becomes one app among many.

> **Myrhiza framing question:** PR #636 framed "the kernel" against an
> existing chat-monolith Willow product. Myrhiza is the kernel from day
> one. The "what stays the same / what changes" sections of PR #636 are
> migration-shaped; Myrhiza reads them as "what the kernel is, full
> stop." Some PR #636 sentences ("the web client becomes the default UI
> app") only parse if you remember Willow's pre-runtime shape.

## Kernel responsibilities

The kernel provides exactly what every P2P app needs and no more:

- **Identity & signatures.** Ed25519 keypairs. Private keys live only in
  the kernel; components describe events and the kernel signs.
- **Peer protocol.** iroh, gossip, blob fetch, topic membership.
- **Event/DAG primitives.** `Event { author, prev, deps, payload, sig }`,
  `EventDag<P>` generic over opaque payload bytes, `PendingBuffer`, sync
  summaries, HLC.
- **Component loader & capability arbiter.** Instantiates WASM
  components, brokers every inter-component call, enforces capability
  declarations.
- **Narrow native imports.** DOM, network egress, persistent storage —
  bound only to specific component classes that have the capability.

Everything else is a component. Chat semantics, UI, themes, integrations,
bridges, future "the server has roles" features — all components.

## The four component profiles

PR #636's defining table. **Myrhiza CLAUDE.md has already lifted this
verbatim** — it is the highest-leverage decision in the entire spec.

| Profile | Determinism | Imports | Where it runs |
|---|---|---|---|
| **`state-apply`** | **Strict** — bit-identical across peers | Deterministic helper set: signature verification, payload-MAC verification, content hashing, key-handle installation, HLC extraction, log | Every peer materializing the topic |
| **`state-propose`** | Loose (runs once on originator) | `host.hlc`, `host.random`, `host.seal` (capability-gated), `host.log` | The peer originating the event |
| **`interaction`** | Non-deterministic OK | `host.broadcast`, `host.subscribe`, `host.kv`, `host.user-prompt`, UI app's `ui:*` | Any peer with a UI / agent host |
| **`behavior`** | Non-deterministic OK | + `host.http`, `host.timer`, `host.identity` (own keypair, gated) | Designated peer(s) |

A maintenance-component "fourth profile" appears in
`research-notes-distributed-maintenance.md` — see
[open-problems.md](open-problems.md). Whether it is structurally distinct
or just a deployment role of behaviour is unsettled.

State components have **two entry points** on the same WASM module:
`apply` runs everywhere deterministically; `propose` runs only on the
originating peer to construct an event payload, after which the kernel
signs and broadcasts.

## Apps as bundles of components

An app is the user-facing distribution unit. Concretely:

```
chat-server/                            (the bundle)
├── manifest.toml                       (version, hashes, capabilities, interfaces)
├── state.wasm                          (deterministic; required by any materializing peer)
├── interaction.wasm                    (typed view + commands; loaded if peer has a UI)
├── behavior-discord-bridge.wasm        (optional)
└── schema.wit                          (interface contract)
```

**Properties:** hash-pinned, signed by author, fetched by hash via
iroh-blobs, lazy-loaded per profile, hash-cached after first load.
A peer in five servers does not instantiate all five interaction
components at startup; the kernel loads on first use.

State components materialize as soon as the peer subscribes to a topic
(so it can apply incoming events). Worker-computed snapshots can carry
peers through warm-up.

## Pre-check equals apply

**The most load-bearing semantic in the whole spec.** Already lifted
into Myrhiza CLAUDE.md.

> Pre-check is mechanically the same WASM function as `state-apply`,
> called by the kernel in dry-run mode against a hypothetical
> post-state.

Apps export one authority predicate; the kernel calls it once before
signing on the originator (with the proposed event applied to a scratch
copy of state) and again on every peer during real `apply`. Compare-
acceptance is **enforced because it is the same export**, not by
convention. Pre-check therefore runs under the `state-apply` runtime
profile — same deterministic helper set, same fuel posture, same denied
non-deterministic imports.

**Failure-closed.** When pre-check panics, exhausts fuel, traps, or
loops, the user-action is rejected and the event is *not* signed.
Failing open is forbidden because rejected events accumulate in the
per-author DAG and cannot be removed without breaking the chain.

## Cross-peer convergence

The kernel verifies convergence by hashing a **canonical state digest**
the app exports (`state-digest()`), **not a hash of WASM linear
memory**. Memory-hash would diverge trivially due to allocator
behaviour, struct field padding, or `HashMap` iteration order.

The encoding rules belong in a child spec; the master commitment is
that convergence is checked against an app-canonical digest. **Caveat
on PR #636's "existing-codebase precedent" claim**: shipped Willow
encodes via **bincode** with sorted `BTreeMap`/`BTreeSet` collections
(`crates/state/src/event.rs:532`, `sync.rs:100`), not postcard. The
sorted-collection discipline is the load-bearing piece; PR #636 proposes
postcard as the canonical `state-digest` form going forward. Treat
"postcard precedent" in PR #636 as forward-looking, not historical.

## Crypto and key custody

The pivotal design choice is **secrets do not enter component memory in
their raw form**. Components hold opaque handles; the kernel custodies
bytes.

- **Private signing keys**: kernel-only.
- **Symmetric channel/group keys, ratchets, MLS group state**:
  kernel-custodied on behalf of an app instance, app refers via opaque
  handles.
- Typed crypto host imports bound to handles — `host.seal`, `host.open`,
  `host.verify-payload-mac`, `host.install-key`. Each is profile-gated:
  `seal` on propose/behaviour, `open` on interaction, `verify-mac` and
  `install-key` (deterministic) on apply.
- **Key generation and rotation events are app-defined.** Kernel
  doesn't know what "channel" or "epoch" means; it only knows handles.

State-`apply` never sees plaintext message content and never sees a
return value indicating local decryption capability. It records that
the handle exists; whether *this* peer can use it is a separate
interaction-profile query (`host.can-open(handle)`).

## UI is an app

The Leptos web client becomes "the default UI app." `ui:*` is an
**interaction contract for app-to-UI integration, not a portable UI
substrate**. Each UI app implements the contract in its own idiom.

Plausible UI apps over time: `willow-ui-tui`, `willow-ui-mcp`,
`willow-ui-mobile-native`, `willow-ui-dioxus`. App authors target the
WIT contract; their interaction components work against any UI app
that exports the interfaces they import.

**Custom-pixel surfaces** (whiteboard, code editor, network-graph
visualiser, 3D voice room) are an explicit out-of-band escape hatch,
not part of `ui:*`. On web, sandboxed iframes embedded by the default
UI app, communicating via a kernel-mediated postMessage capability.
On other platforms, platform-specific.

> **Myrhiza framing question:** "UI is an app" carries weight inside
> Willow because the Leptos client *was* the product. For Myrhiza, the
> default UI is *not yet shipped*; the framing should perhaps be
> "Myrhiza has no built-in UI; UIs are apps from day one." Re-evaluate
> the privileged-default-UI carve-out for Myrhiza's context.

## Inter-component composition

Components compose by importing each other's exposed interfaces,
**always mediated by the kernel**. Cross-component calls are typed,
bounded, and refusable. There is no direct memory-shared linkage
between components; the kernel is the capability arbiter, the call
broker, and the resource-handle resolver.

## Submit-and-poll for async

Browser jco does not support async; v1 ABI is sync. Kernel calls that
wrap inherently async surfaces (gossip broadcast, blob fetch, HTTP,
persistent KV, timers) follow a **submit-and-poll** pattern: the
component calls a sync host function returning a `request-token`; the
kernel later re-enters the component via an exported
`on-completion(token, result)` handler in the appropriate profile.

The ergonomic cost is real — apps cannot use familiar `async`/`await`
flow control. SDK macros are expected to hide token-juggling for common
patterns.

## ABI commitments

WIT-shaped semantics is the eventual interface ABI. Two v1 candidates:

- **(A) Full WebAssembly Component Model from day one.** wit-bindgen,
  wasmtime native, jco-transpiled glue + core wasm in browser.
  Ecosystem-aligned. Cost: heavier toolchain, browser CM still maturing,
  ~350 KB JS shim floor, no async on browser side.
- **(B) Extism for v1, WIT-shaped where possible.** Ship faster on a
  simpler runtime. Every host-call signature chosen to be
  WIT-expressible. Cross-component composition in v1 is
  **kernel-brokered RPC by opaque ID only** — Extism has no notion of
  imported/exported resource handles, borrowed lifetimes, world
  composition, or futures/streams. Migration to full Component Model
  later is **a real refactor for app authors** (resource handles
  replace ID lookups, imported interfaces replace kernel-broker calls,
  borrows replace clone-and-pass).

PR #636 leans (B) tentatively. **Myrhiza inherits the open question.**

## Behaviour identity

When a peer enables a behaviour, the kernel generates and custodies a
fresh Ed25519 keypair scoped to **(peer, behaviour-instance)**. Events
authored through `host.broadcast` are signed under that identity, not
the user's. The runtime does **not** migrate behaviour keypairs between
peers; cross-peer behaviour continuity is an app-level concern.

PR #636 calls out: "**this is structurally the same problem as
multi-device user identity**" (long-term identity, short-lived
per-device signing key) and notes both should share a kernel-level
mechanism rather than be invented twice. Myrhiza inherits this
unification work.

## Worker trust shifts

Today's workers run trusted in-tree Rust. Under the runtime, a worker
subscribed to N topics may be executing N distinct, **third-party-
authored, attacker-influenceable** WASM state components simultaneously.
DoS resistance, fuel scheduling, per-instance memory caps, fair-share
between topics, and operator-level deny-lists become load-bearing
operational concerns.

This is one of the most consequential shifts and is split out into the
"Worker as untrusted-WASM execution host" child spec.

## What stays the same

- Event-sourced per-author Merkle DAG with prev/deps causal links.
- Identity rooted in Ed25519 signatures.
- iroh for transport (gossip + blob fetch).
- Relays remain dumb topic-bridges; they do not materialize state and
  do not run WASM.
- Workers remain peers, just generalized.
- Dual-target compilation discipline (native + WASM) survives at the
  kernel layer; for application code it is replaced by "build once to
  wasm, loaded by whichever kernel a peer is running."

## MVP shape

The smallest end-to-end demonstration that the runtime is real:

1. Kernel loads and instantiates a WASM state component from an
   iroh-blobs bundle.
2. Component applies events deterministically; multiple peers running
   the same component bytes converge to the same state hash.
3. UI app loads an interaction component, projects a view, submits a
   command, observes the resulting state change.
4. Second app instance (different state component, different topic)
   coexists on the same peer; events do not cross.
5. Capability declarations actually gate access.
6. Behaviour component runs on a designated peer, observes events, and
   logs them.

**The demo app is an open question.** Candidates: shared-counter
(~50 LOC state, ~100 LOC interaction); single-channel chat that doesn't
reuse `ServerState`; real-time poll. Job: prove the kernel doesn't know
about chat while still exercising the determinism + interaction loop.

## Planned child specs

PR #636 names these as anticipated child specs, in roughly the order
they become useful. **Myrhiza will write its own versions of each**:

- Kernel boundary
- ABI & runtime backends (the (A) vs (B) decision)
- WIT interfaces (`ui:*`, `state:*`, `behavior:*`, `host:*`)
- Capability model & install UX
- Distribution, signing & versioning
- App SDK ergonomics
- Determinism enforcement
- State materialization on workers
- Worker as untrusted-WASM execution host
- Relay and topic-ID rotation
- Crypto and key custody boundaries
- Runtime and actor coexistence
- MVP demo app
- Chat-server migration (Willow-specific; Myrhiza re-frames as "first
  app on Myrhiza")

## What Myrhiza has already lifted

Verbatim or near-verbatim in `/mnt/storage/projects/myrhiza/CLAUDE.md`:

- 4-profile table (state-apply / state-propose / interaction /
  behaviour) with strict determinism on `state-apply`.
- "Pre-check is mechanically the same WASM function as `state-apply`,
  called by the kernel in dry-run mode."
- "Capabilities are the only host surface."
- "Determinism is a load-bearing property" framing.

## What's still open for Myrhiza

- ABI choice (A vs B). PR #636 leaned B; Myrhiza has not committed.
- MVP demo app shape.
- Relay topic-ID rotation protocol.
- Behaviour-identity / multi-device-identity unified custody.
- Worker capability advertisement.
- Resource limit defaults.
- Handle namespace ownership rules.
- Snapshot portability across component-version upgrades.
- Hot reload (PR #636 defers to v2).
- Maintenance-as-fourth-profile vs maintenance-as-behaviour-deployment.

See [open-problems.md](open-problems.md) for canonical sources to
consult when designing.

## What should be re-evaluated for Myrhiza's framing

- **"UI is an app" as the in-tree default.** PR #636 privileged the
  Leptos client. Myrhiza has no incumbent UI; the privileged-default
  carve-out should be re-justified, not inherited.
- **Migration framing throughout.** PR #636's "what stays the same /
  what changes" sections assume a chat monolith to migrate from.
  Myrhiza reads these as "what the kernel is."
- **The chat-server-migration child spec is Willow-specific.** Myrhiza's
  equivalent is "first app on Myrhiza," which is a fresh design, not a
  migration.

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)
- PR #636: [intendednull/willow#636](https://github.com/intendednull/willow/pull/636)

## Sources

- `/tmp/willow-pr-636.diff` (full PR diff, 843 lines).
- `docs/specs/2026-04-27-willow-runtime/README.md` — master runtime
  design (674 lines).
- `docs/specs/2026-04-27-willow-runtime/research-notes-distributed-maintenance.md`
  (157 lines).
- Cross-checked against
  `/mnt/storage/projects/myrhiza/CLAUDE.md` (Component Profiles section,
  Dev Guidelines).
