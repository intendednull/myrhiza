**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — open problems Myrhiza inherits, with canonical sources

What Willow + PR #636 named without resolving. Each entry: short
problem statement, why it matters, and the literature / sibling-corpus
folders Myrhiza will consult when designing the answer.

See also: [README.md](README.md), [runtime-vision.md](runtime-vision.md),
[lessons.md](lessons.md).

## Distributed maintenance + Sybil-resistant participation enforcement

`research-notes-distributed-maintenance.md` (157 lines, sibling to PR
#636 master) frames maintenance work as a **fourth class of
components** alongside state / interaction / behaviour: persister,
snapshot provider, sync provider, replay buffer. A peer's
"participation" is the set of maintenance components it has instantiated
plus the capacity it has declared.

The hard problem is **enforcement under Sybil**: a custom client that
does not run maintenance components, multiplied by spinning up many
identities, free-rides on honest participants. Self-reported
participation is gameable. Refusal-to-serve-non-participants is the
enforcement primitive Willow's note proposes; the open question is the
metric and Sybil-defense underneath it.

**Willow's structural advantage:** the existing permission/invite trust
graph already carries a Sybil-relevant signal. Most P2P systems
bootstrap social graphs they don't have. Willow has one for free. PR
#636 explicitly calls this out as a unique advantage worth exploiting.

**Canonical sources:**

- **BAR Gossip** (Aiyer, Alvisi, Clement, Cowling, Dahlin, Riché 2005)
  — the right academic frame for "honest peers + free-riders +
  actively-malicious peers." See follow-ups: FlightPath, BAR Fault
  Tolerance.
- **EigenTrust** (Kamvar, Schlosser, Garcia-Molina 2003) — global trust
  via gossip eigenvector. Sybil-vulnerable but canonical reference.
  Variants: PowerTrust, PeerTrust.
- **SybilGuard / SybilLimit** (Yu et al. 2006, 2008) — social-graph-
  based Sybil resistance. Directly relevant if Myrhiza uses
  permission/invite graph as social-graph input.
- **Whanau** (Lesniewski-Laas 2010) — Sybil-proof DHT.
- **BitTorrent's choking algorithm** (Cohen 2003) — local pairwise
  reciprocity, the most successful deployed answer. Sybil-tolerant
  per-connection. Limitation: symmetric workloads.
- **PropShare** — BitTorrent variants with formal analysis.
- **Tribler's BarterCast** — local-view reputation in deployed P2P.
- **Holochain's validator-selection / DHT-responsibility model** —
  every node validates entries it's "responsible for." Closest existing
  system structurally; lessons probably translate directly.
  (Cross-reference: `prior-art/holochain/`.)
- **IPFS Bitswap** — ledger-based reciprocity for block exchange.
  Closer in spirit to BitTorrent than to a reputation system.
- **Adar & Huberman, "Free Riding on Gnutella" (2000)** — the canonical
  measurement paper. ~70% of Gnutella users contributed nothing.
  Establishes the problem.
- **Filecoin proofs of replication / proofs of spacetime** —
  cryptographically verifiable storage. Heavy machinery; only relevant
  if strong durability guarantees are wanted. Lighter alternative:
  audit-style challenge-response over stored data.

**Myrhiza decision points:** (a) tit-for-tat (Bitswap-ish) vs reputation
(EigenTrust-ish) vs social-graph Sybil resistance (SybilGuard-ish) vs
DHT-responsibility (Holochain-ish) — likely a hybrid; (b) whether to
use the permission/invite graph as the social-graph input — strong yes
candidate; (c) whether the maintenance-as-fourth-profile framing
survives or collapses into "deployment role of behaviour."

## Multi-device identity (and behaviour identity, unified)

PR #636 calls out behaviour-identity (per-(peer, behaviour-instance)
keypairs custodied by the kernel) as **structurally the same problem as
multi-device user identity**: long-term identity, short-lived
per-device signing key. Both want one mechanism, not two.

Today's Willow has neither — a user is one Ed25519 keypair on one
device. The seal-gift-wrap deferral spec
(`docs/specs/2026-04-24-seal-gift-wrap-dms.md`) calls out multi-device
as non-negotiable for any future MLS adoption.

**Canonical sources:**

- **Signal's PNI / sealed-sender** — production multi-device identity
  with sender anonymity for delivery.
- **MLS** — group state under a long-term identity with multiple device
  leaves; cross-reference `prior-art/mls/`.
- **DIDs (Decentralized Identifiers, W3C)** — abstract identity layer
  with key rotation.
- **AT Protocol's DID + signing-key rotation** — a deployed P2P shape
  for "user identity is not equal to active signing key."
- **Holochain's agent ID + capability tokens** — cross-reference
  `prior-art/holochain/`.

**Myrhiza decision points:** unified mechanism for (user × device) and
(peer × behaviour-instance); whether short-lived signing keys emit a
"key registration" event app-layer or kernel-layer; revocation
semantics; backward-compat with single-key apps.

## MLS adoption

Willow defers MLS to a future "MLS-over-Willow" spec
(`docs/specs/2026-04-24-seal-gift-wrap-dms.md`). Today's Willow uses
plain ChaCha20-Poly1305 with X25519 key exchange + per-channel keys
rotated via app-defined `RotateChannelKey` events.

PR #636 places MLS group state on the kernel side
(`host.mls` typed capability surface bound to an app's group handle).
Apps emit MLS Welcome / Commit / Application events through ordinary
state propose; the kernel-side MLS engine processes them under the
requesting peer's identity.

**Canonical sources:** the `prior-art/mls/` corpus is the entry point.
Key references inside: RFC 9420 (MLS), RFC 9750 (architecture), MLS WG
extensions, OpenMLS implementation.

**Myrhiza decision points:** whether to commit to MLS at v1 or design
a smaller native crypto contract first; how `host.mls` interacts with
app-defined rotation events; persistence of MLS state across kernel
restart; key-package distribution.

## Hot reload / component update

Deferred to v2 in PR #636: "Component update is restart for v1."
Restart-as-update has real cost — open WebRTC sessions, in-flight
broadcasts, snapshot warm-up state — but the alternative (live module
swap with state migration) is research-grade.

**Canonical sources:**

- **Erlang/OTP hot code loading** — the canonical reference for live
  code swap with state migration. Relevant patterns:
  `code_change/3`, supervisor restart strategies. Cross-reference
  `prior-art/` if an Erlang folder exists.
- **WebAssembly Component Model + wasmtime instance pre-instantiation**
  — what the substrate offers natively.
- **Spin / wasmCloud** restart strategies — cross-reference
  `prior-art/spin/` and `prior-art/wasmcloud/`.

**Myrhiza decision points:** whether v1 restart-only is acceptable;
state-handoff protocol when v1.0 → v1.1 of an app's state component
ships; rollback when the new version's pre-check rejects the previous
version's events.

## Cross-app authority composition

PR #636: "out of scope for v1, but what shape should the v2 hooks
take?" The shape of the question: app A wants to authorize app B's
events on a shared topic, or two apps' state components want to share a
permission grant. Willow has no precedent — there is one app today
(chat) and authority is internal to it.

**Canonical sources:**

- **OCapN / Spritely** — capability-secure object composition with
  delegated authority. Cross-reference `prior-art/spritely-ocapn/`.
- **Agoric Endo** — composable capability hardening.
  Cross-reference `prior-art/agoric-endo/`.
- **WebAssembly Component Model resource handles** — the substrate's
  native answer if v2 ABI is full CM. Cross-reference
  `prior-art/wasm-component-model/`.

**Myrhiza decision points:** whether v1 forbids cross-app authority
entirely (clean but limiting), allows it only via app-defined
co-export ("app A imports app B's authority predicate"), or builds a
kernel-level capability-passing primitive.

## Snapshot portability across component upgrades

When an app's state component goes v1.0 → v1.1, do existing snapshots
remain valid? Today: no — the snapshot is an opaque byte image of v1.0's
internal state, and v1.1's `apply` may not understand it.

**Canonical sources:**

- **Cap'n Proto / Protobuf schema evolution** — backward-compatible
  serialization rules.
- **Postcard** (Willow's existing canonical encoder) — versioning
  conventions.
- **Holochain's DHT migration story** — cross-reference
  `prior-art/holochain/`.
- **CRDT migration patterns** in Automerge / Yjs — cross-reference
  `prior-art/crdts/`.

**Myrhiza decision points:** require apps to export an explicit
snapshot-migration function from v(N) → v(N+1); or require apps to
re-replay from genesis on upgrade (clean, slow); or version snapshots
with the component hash and refuse cross-version load.

## Topic-ID rotation through dumb relays

The existing epoch-rotation spec
(`docs/specs/2026-04-24-epoch-key-rotation.md`) tells the relay "this
is a rotation event, here's the next topic ID." Under PR #636's runtime
the relay no longer runs app code, so it cannot be told this directly.
Naive public discovery would defeat the rotation's unlinkability
property — non-members would learn the next topic ID.

The likely shape is **members announcing the next topic to the existing
relay session before rotation**, but the exact protocol is deferred to
a relay-and-rotation child spec. PR #636's master commitment is only
that the kernel is not in this loop.

**Canonical sources:**

- **Tor's hidden-service descriptor rotation** — unlinkability via
  rotated descriptors with member-only knowledge of the next descriptor
  ID. Closest analogue.
- **Iroh relay capability-doc** (`docs/specs/2026-04-24-relay-capability-doc.md`)
  — already the right shape for "relay advertises what it can bridge."
- **Signal's pre-key rotation** — production-scale rotation under
  forward-secrecy constraints.
- Cross-reference `prior-art/iroh/` for transport-layer constraints.

**Myrhiza decision points:** rotation protocol shape; member discovery
without leaking; relay reconnect across rotation boundaries.

## Worker capability advertisement

Parallel to relay-capability-doc: should workers advertise which
app-component hashes they host? Lets peers discover "a worker that
materializes my chat-server app" without out-of-band config.
Alternative: stays operator-config (workers join topics via static
configuration; peers discover them via gossip-membership).

**Canonical sources:** existing `relay-capability-doc.md` as the
template; iroh's discovery primitives.

**Myrhiza decision points:** advertise-by-default vs opt-in; signed
advertisements vs gossip-only; abuse surface (worker
flooding-advertisements for topics it cannot really host).

## Resource-limit defaults

Per-instance fuel cap, memory cap, blob-store quota, KV quota. PR
#636: "what defaults?" — left open. Defaults matter because most apps
will accept them; the wrong default ships an exploitable
denial-of-service vector or a too-restrictive ceiling that breaks
real workloads.

**Canonical sources:**

- **wasmtime epoch-based interruption** — the substrate's native
  fuel-equivalent.
- **Spin's resource limits** — cross-reference `prior-art/spin/`.
- **wasmCloud capability-provider quotas** — cross-reference
  `prior-art/wasmcloud/`.
- **Cloudflare Workers** — production scale data on real-world WASM
  resource ceilings.

**Myrhiza decision points:** default fuel-per-event-apply; memory
cap per state component; behaviour-component memory cap (typically
larger because behaviour holds long-running state); operator override
mechanism.

## Pre-check fuel budget independence from apply

Pre-check runs under `state-apply` profile but **only on the
originating peer**. Does it share `apply`'s per-event fuel cap, or
budget separately? Pre-check is failure-closed, so an adversarial app
that always-exhausts pre-check is a self-DoS for that peer; less
catastrophic than `apply`-time exhaustion. But pre-check on the
originator runs in user-interactive context — too-tight a budget makes
the UI feel laggy; too-loose lets a malicious app tar-pit user input.

**Myrhiza decision points:** shared budget (simple, conservative) vs
separate budget (better UX, more knobs); whether pre-check exhaustion
should be reported to the user differently than apply exhaustion.

## Handle namespace ownership

Two apps installing keys under the same opaque handle on one peer —
collision, namespacing per-app instance, or kernel-arbitrated
allocation? PR #636 calls this out without resolving.

**Canonical sources:**

- **Capability-systems literature** — handle scoping rules (per-process,
  per-component-instance, per-grant). Cross-reference
  `prior-art/spritely-ocapn/`.
- **Unix file descriptors** — per-process namespacing as deployed prior
  art.

**Myrhiza decision points:** per-app-instance namespace (clean,
mainstream); kernel-allocated globally-unique handles (more bookkeeping
but supports cross-app handle passing); collision-as-error (forces
explicit conflict resolution).

## Behaviour coordination primitives

When two peers run instances of the same behaviour for redundancy,
**leader election** and **dedup of emitted events** are needed. PR #636
leaves both to apps: "apps that need single-emitter semantics implement
leader election in their own state component."

The open question: should the runtime offer a kernel-level coordination
primitive, or stay strict?

**Canonical sources:**

- **Raft / Paxos** — classical leader election.
- **Erlang/OTP `pg2`, distributed registries** — practical primitives
  for "one of N peers owns this role."
- **Kubernetes leader election** — production deployment shape.
- **Holochain capability tokens for role-binding** — cross-reference
  `prior-art/holochain/`.

**Myrhiza decision points:** kernel-offered shared primitive (cleaner
ergonomics, more kernel surface) vs app-implemented (matches kernel
minimalism, every app reinvents leader election); hybrid where the
kernel offers a "behaviour-coordination" capability that apps can wire
into their state component.

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- `/tmp/willow-pr-636.diff` — sections "Open questions deferred to
  child specs", "Constraints we accept", "Child specs (planned)".
- `docs/specs/2026-04-27-willow-runtime/research-notes-distributed-maintenance.md`
  — full file; the participation/Sybil discussion lives here.
- `docs/specs/2026-04-24-seal-gift-wrap-dms.md` — multi-device-identity
  deferral.
- `docs/specs/2026-04-24-epoch-key-rotation.md` — topic-rotation
  current state.
- `docs/specs/2026-04-24-relay-capability-doc.md` — the
  capability-advertisement template.
- Cross-references: `prior-art/holochain/`, `prior-art/mls/`,
  `prior-art/iroh/`, `prior-art/spritely-ocapn/`,
  `prior-art/agoric-endo/`, `prior-art/spin/`, `prior-art/wasmcloud/`,
  `prior-art/wasm-component-model/`, `prior-art/crdts/`.
