**Date:** 2026-05-09
**Status:** brainstorming input
**Subject:** Myrhiza design space — networking, blob distribution, sync, workers, distributed maintenance, Sybil-resistant participation

This file mines the prior-art corpus for the load-bearing networking
cluster of Myrhiza's master spec. Iroh is locked as transport; the question
is what shape the kernel exposes around it, and how the runtime answers
the open distributed-maintenance + Sybil problem PR #636 deferred.

The single biggest open problem is **distributed maintenance + Sybil-
resistant participation**: Willow's permission/invite trust graph is a
genuine advantage; the literature (BAR Gossip, EigenTrust, SybilGuard,
Whanau, BitTorrent choking, Bitswap, Holochain DHT-responsibility,
Filecoin storage proofs) gives 20+ years of designs but no plug-and-play
answer. Other clusters (state-apply determinism, identity, ABI) live in
sibling reports.

## Domain index

1. Gossip topology + blob distribution
2. Relay model
3. Sync protocol
4. Worker trust model
5. Distributed maintenance — component vs kernel built-in
6. Sybil-resistant participation enforcement (the load-bearing one)
7. Worker fuel + resource limits
8. Browser peer connectivity
9. Topic-ID rotation through dumb relays

---

## 1. Gossip topology + blob distribution

iroh-gossip (HyParView + Plumtree) and iroh-blobs (BLAKE3 + Bao verified
streaming) are committed by transport choice. The decision is what
**policy** the kernel imposes above them: who joins a topic, who broadcasts,
who fetches what, fairness.

### Option A — Open topic, application-authenticated (Willow's shipped shape)

- **Mechanism.** Any peer subscribes to any 32-byte topic ID. iroh-gossip
  forms swarm transitively from one bootstrap peer. Authentication is
  payload-level: every event is Ed25519-signed by author, verified at
  receiver before `state-apply` is called.
- **Pros.** Maps directly to iroh primitives (no kernel-level membership
  layer). Topic-ID derivation is public-defaulting (`BLAKE3(name)`) and
  gives apps full control over discovery secrecy. Already proven in
  Willow's chat-server multi-peer e2e suite.
- **Cons.** Anyone in the topic can broadcast anything; rate limiting
  and content filtering are app-layer. Sized for thousands of peers per
  topic, not millions (HyParView's active view is ~5, passive ~30 —
  Plumtree gives high probability of delivery, not guarantee). No
  built-in spam control.
- **Sources.** `prior-art/iroh/gossip.md`, `prior-art/willow/networking.md`,
  HyParView (Leitão 2007), Plumtree (Leitão 2007).
- **Precedent.** Willow ships this today. iroh-docs and iroh-willow both
  use per-namespace gossip topics.

### Option B — Permission-graph-gated membership (Sybil-relevant)

- **Mechanism.** Topic membership is enforced at the protocol layer: a
  peer can join a topic only if it holds a `JoinPermission` event (or
  invite token) verifiable against the topic's authority graph. Gossip
  neighbors refuse to add unauthorized peers to their active view.
- **Pros.** Naturally Sybil-resistant: forging identities doesn't get
  you into the gossip mesh without a permission grant. Composes with
  Willow's existing permission/invite trust graph (the unique
  advantage). Prevents one class of free-riders by construction.
- **Cons.** Now the kernel needs a notion of "topic authority" before a
  state component has run. Bootstrapping a new peer requires an
  out-of-band invite delivery. Membership grants must be replicated
  early in the gossip path or new peers hang.
- **Sources.** Holochain membrane proofs (`prior-art/holochain/lessons.md`
  §"Borrow item 6"), Willow's `EventKind::GrantPermission` /
  `EventKind::CreateInvite` (`willow/authority.md`).
- **Precedent.** Holochain ships membrane proofs as a per-DNA
  configurable hook; it is one of Holochain's better-validated patterns.

### Option C — Sharded gossip + DHT-responsibility (Holochain-shaped)

- **Mechanism.** Each peer declares a "storage arc" — a slice of an
  address space (BLAKE3-flavored). Peers gossip with neighbors whose
  arcs overlap; each event/blob is responsibility of the peers whose
  arcs cover its hash. Validation + storage cost is bounded per peer.
- **Pros.** Scale beyond "thousands per topic." Distributes maintenance
  load deterministically — no need for separate "worker" role.
- **Cons.** Holochain spent six years on this and partial-arc operation
  is **still** not the load-tested path. Eclipse attacks on neighbor
  selection are real. Arc resizing is heuristic, no convergence proof.
  Adds substantial protocol surface that iroh-gossip doesn't supply.
- **Sources.** `prior-art/holochain/networking.md` (Kitsune2 deep dive),
  `prior-art/holochain/open-problems.md` §3, §5, Whanau (Lesniewski-Laas
  2010).
- **Precedent.** Kitsune2 deployed; six years of detour the Holochain
  team owns honestly.

### Willow position

A. Willow has not shipped sharding and the chat-shape MAU class hasn't
forced the issue. PR #636 retains gossip-as-default.

### Re-evaluation question

Does Myrhiza's app-shape ambition (chat is one app; what about apps
with hundreds of thousands of peers per topic?) force C, or does the
sharding decision get deferred to "scale-out" specs after MVP? Holochain's
warning is unambiguous: the easy thing becomes the load-tested path.

### Blob distribution sub-domain

Iroh-blobs is the BLAKE3+Bao primitive (`prior-art/iroh/blobs.md`).
**Discovery is not solved upstream** — iroh-blobs explicitly delegates
"find a peer with this hash" to higher layers. Three options:

- Pull-only via gossip-distributed announce (Willow's pattern: gossip
  signals "new event" carrying blob hash references; receivers fetch
  blob over direct iroh stream).
- Bitswap-style ledger (IPFS): peers track per-peer "owe/owed" ledgers,
  prioritize sending to peers who've sent to them. Sybil-tolerant
  per-connection.
- DHT-announce on publish (Hyperswarm pattern): publisher announces
  `BLAKE3(blob) → peer-id` on a DHT.

Willow ships pull-from-gossip-announce. Bitswap is the most-deployed
fairness mechanism in P2P content distribution and the obvious
candidate for "honest peers should not pay disproportionate egress for
free-riders." See §6.

### Cross-stack reject: Pears' Hyperswarm + Hypercore

Hyperswarm's `hyperdht` ANNOUNCE/LOOKUP commands and blind-relay
fallback are a credible alternative substrate (Keet ships on this at
hundreds-of-thousands MAU). Hypercore is a single-author append-only
signed log (`prior-art/pears/hypercore-stack.md`); Autobase generalizes
to multi-writer linearization with a deterministic apply contract.

**Why rejected for Myrhiza:** (1) The whole stack is JS-on-native-
libsodium with no WASM build and no current Rust port — impossible to
host inside a WASM CM sandbox without giving the kernel an opaque
native dep. (2) protomux has no per-channel flow control; one slow
consumer back-pressures all other channels — a real problem when one
peer hosts many topic-state components concurrently. (3) Hypercore's
wire-incompatible major-version cuts (8→9, 9→10, 10→11) are an
ecosystem hazard. **What we inherit conceptually:** the multi-author
DAG generalizes Hypercore's single-author chain (we get this for free
from Willow's per-author DAG); content-addressed storage; sync via
heads exchange. **What we reject:** the specific stack.

---

## 2. Relay model

PR #636 commits to **dumb topic-bridge** (relay never inspects payloads,
never materializes state, never runs WASM; pure encrypted packet
forwarding for browser peers and TCP-only networks). The open question
is whether Myrhiza also offers a privileged "sync provider" role.

### Option A — Dumb bridge only (PR #636 stance)

- **Mechanism.** Relay is a TCP/WebSocket front-end that forwards iroh
  ALPN-tagged frames to a loopback `iroh-relay` and forwards encrypted
  gossip packets through. Never holds state, never decrypts, never runs
  app code. Already shipped in Willow's `crates/relay/` (1150 lines of
  code that does almost nothing per design).
- **Pros.** Operating relay fleet is cheap (CPU is just packet
  forwarding). Trust surface is minimal — a hostile relay can only
  affect availability (drop, delay, reorder), never correctness.
  Multiple relays for redundancy without coordination — gossip handles
  propagation across the mesh. DoS guards are simple (`MAX_TOPICS`,
  `MAX_TOPIC_LEN`, `BOOTSTRAP_IO_TIMEOUT`).
- **Cons.** Doesn't solve "newcomer joining a long-running topic needs
  full history and there's no warm peer to source it from." Relay
  cannot help with state warm-up.
- **Sources.** `prior-art/willow/networking.md` §"Relay = dumb
  topic-bridge", `prior-art/willow/workers.md` §"willow-relay",
  PR #636 lines 535-550.
- **Precedent.** Willow's `crates/relay/` already runs this way after
  the libp2p → iroh migration.

### Option B — `SyncProvider`-permissioned worker (Willow's worker layer)

- **Mechanism.** A peer holding a `SyncProvider` permission grant in
  the app's permission DAG materializes state, serves history catch-up
  (`HeadsSummary` deltas / `Snapshot` for far-behind peers), and
  responds to `WorkerRequest::Sync`. It IS a peer; the permission
  grants no kernel privileges, only an in-app "I am authoritative for
  catch-up" claim.
- **Pros.** Solves history catch-up cleanly. Workers are CRDT-shape
  caches: a hostile worker can drop/delay but cannot forge or rewrite
  (see §4). Composes with the four-actor pattern Willow shipped.
- **Cons.** Distinct from relays; operationally there are **two** kinds
  of always-on infrastructure to run. The permission grant is per-app,
  so cross-app sync providers require multiple grants.
- **Sources.** `prior-art/willow/workers.md` §"willow-replay",
  §"willow-storage", §"Permission model".

### Option C — Combine: relay-as-bridge + worker-as-component on the same host

- **Mechanism.** Operator runs one process that does both: dumb relay
  for transport, plus worker that loads maintenance components for
  apps it has consented to host. The kernel's worker-host model says
  "any peer can be a worker" — relay operators just opt in.
- **Pros.** One operational unit, two roles. Matches PR #636's
  maintenance-as-component reframing (§5).
- **Cons.** Relay's clean trust story muddies — now this process has
  privileged sync state too. Operator must be careful which apps they
  consent to host.
- **Sources.** PR #636 lines 619-625 (child specs), `prior-art/willow/
  workers.md` §"For Myrhiza".

### Willow position

Today: A and B as separate processes. PR #636: keeps A, generalizes B
into "any peer can be a worker for any topic it subscribes to" with
WASM maintenance components.

### Re-evaluation question

Should the master spec name relay and worker as **roles**, allowing
the same host to wear both hats, or as **distinct infrastructure
units**? Willow has shipped them separate; PR #636 reads as
collapsible into one. Worth explicit.

---

## 3. Sync protocol

Willow has consolidated on `HeadsSummary`-delta exchange after four
2026-04-24 sync specs landed. The candidates are well-mapped.

### Option A — `HeadsSummary` delta exchange (Willow shipped)

- **Mechanism.** `HeadsSummary { heads: BTreeMap<EndpointId, AuthorHead
  { seq, hash }> }`. A peer requests sync by sending its summary;
  responder ships ascending-seq events for any author where the
  responder is ahead. Per-author monotonicity (enforced by the DAG)
  means streaming `seq > known_max` ascending delivers a contiguous
  chain with no gaps and no fingerprint negotiation.
- **Pros.** O(authors) wire cost, not O(events). Per-author DAG
  enforces structural monotonicity → no equivocation → no need for
  range-based reconciliation. Ships in Willow's
  `crates/state/src/sync.rs`.
- **Cons.** Only works because Willow's data model is a strict
  per-author chain. Doesn't generalize to apps with non-chain payload
  shapes (e.g. a CRDT app where "authorship" isn't the right axis).
- **Sources.** `prior-art/willow/networking.md` §"Sync protocols",
  `willow/docs/specs/2026-04-24-negentropy-sync.md`.

### Option B — Range-based set reconciliation (Willow protocol IETF)

- **Mechanism.** Peers exchange Merkle-fingerprints over byte-ranges of
  the namespace; recursively descend on mismatches, ship missing
  records at leaves. Generalizes to opaque payloads — doesn't require
  per-author chain structure.
- **Pros.** Works for arbitrary set-of-records shapes. iroh-docs and
  iroh-willow both bet on this. Math is sound (Meyer 2022); perf is
  good in practice.
- **Cons.** More wire round-trips than `HeadsSummary` for the
  per-author-chain case. Implementation surface is larger. The
  upstream Willow protocol (IETF Aljoscha) is the academic reference.
- **Sources.** `prior-art/iroh/lessons.md` "Borrow item 9",
  `prior-art/iroh/willow.md`, RBSR (Meyer 2022).

### Option C — Hypercore append-only signed log (Pears)

- **Mechanism.** Per-author signed Merkle-tree-rooted log; sparse
  verifiable replication; sync-by-heads-comparison.
- **Pros.** Same shape as Willow's per-author DAG (we get the conceptual
  inheritance for free).
- **Cons.** Stack is JS-on-native-libsodium; no WASM build. Single-
  writer-per-core means N-cores per multi-writer instance. Wire-
  incompatible major version cuts are flag-day. Rejected as substrate;
  see §1.
- **Sources.** `prior-art/pears/hypercore-stack.md`.

### Option D — Holochain DHT-responsibility validation

- **Mechanism.** Every peer validates entries it's "responsible for" in
  the DHT (its arc). Sync is per-arc neighbor gossip with ring/sector
  decomposition (Kitsune2: spatial sectors × temporal rings).
- **Pros.** Distributes load. Validation correctness is per-neighborhood,
  not per-peer.
- **Cons.** Coupled to sharding (§1 option C) — only makes sense if
  Myrhiza adopts arc-based responsibility allocation. Eclipse attacks,
  unfinished partial-arc story.
- **Sources.** `prior-art/holochain/networking.md` §"Kitsune2 internals".

### Willow position

A is shipped and load-bearing. RBSR is "validate" in iroh-lessons but
not in shipped Willow.

### Re-evaluation question

Does Myrhiza generalize sync to opaque payload (B) or stick with
per-author DAG monotonicity (A)? PR #636 §"What changes about Willow"
keeps `Event`, `EventDag<P>`, sync as kernel — the per-author DAG
shape is preserved. **A is the inheritance.** B becomes interesting
only if the kernel needs to support state-models where author-chain
isn't the right invariant — likely deferred.

---

## 4. Worker trust model

Today's Willow workers are trusted in-tree Rust calling
`apply_incremental` on a fixed `ServerState`. PR #636 envisions WASM-
untrusted-third-party. This is the largest qualitative shift in the
PR.

### Option A — Trusted-Rust-in-tree (Willow today)

- **Mechanism.** `ReplayRole` and `StorageRole` are Rust impls of
  `WorkerRole`, linked into the worker binary, calling
  `apply_event` on a hard-coded state type.
- **Pros.** Simple. No fuel scheduling, no per-instance memory caps,
  no WASM substrate cost. Familiar.
- **Cons.** Multi-tenant operators can't host third-party apps without
  rebuilding workers. The trust model is "operator vouches for every
  app's state code," which doesn't scale to "anyone can publish an
  app."
- **Sources.** `prior-art/willow/workers.md` §"willow-replay",
  §"willow-storage".

### Option B — WASM-untrusted-third-party (PR #636)

- **Mechanism.** Worker subscribed to N topics may execute N distinct,
  third-party-authored, attacker-influenceable WASM `state-apply`
  components simultaneously. Worker process must enforce: DoS
  resistance (one buggy/malicious component cannot crash the worker),
  fuel scheduling (per-event instruction budgets, deterministic by
  spec so cross-peer convergence doesn't drift on under-fueled peers),
  per-instance memory caps, fair-share between topics, operator
  deny-lists.
- **Pros.** Multi-tenant operators host any app whose bundle hash
  passes their deny-list. Scales to "anyone can publish an app."
  Aligns with the runtime reframing.
- **Cons.** Substantial new operational surface. Fuel + memory caps
  must be deterministic-by-spec or peers diverge under different
  load. Fair-share scheduling between topics is non-trivial.
- **Sources.** PR #636 §"What changes about Willow", PR #636 lines
  619-625.
- **Precedent.** Spin / wasmCloud have shipped multi-tenant WASM hosts
  in production; Cloudflare Workers is the reference at scale.

### Option C — Worker capability advertisement (open child-spec)

- **Mechanism.** Workers advertise (signed) hosted app-component hashes
  via a `_myrhiza_workers` gossip topic. Peers discover "a worker that
  materializes my chat-server app" without out-of-band config.
  Parallel to Willow's `relay-capability-doc.md` pattern.
- **Pros.** Self-organizing — peers find workers automatically. Aligns
  with PR #636's "scaling becomes emergent" framing.
- **Cons.** Abuse surface: a malicious worker advertises hosting for
  topics it cannot really serve, forcing connecting peers to fail
  expensively. Mitigation: signed advertisements, behavior-store
  scoring (Holochain Kitsune2 pattern).
- **Sources.** PR #636 lines 648-651,
  `willow/docs/specs/2026-04-24-relay-capability-doc.md`.

### Willow position

Shipped: A. PR #636: B with C as open child-spec. Master-spec
commitment level is "B is the destination; specifics deferred."

### Re-evaluation question

Does Myrhiza's master spec commit to B at v1 or ship A first and
migrate? Willow shipped A and is paying the cost of migration; Myrhiza
starts fresh and can target B from day one — but B is heavier to
build. **Tentative read: B is the master-spec commitment, with capability
advertisement (C) deferred to a child spec.**

---

## 5. Distributed maintenance — component vs kernel built-in

PR #636's research notes propose **maintenance components as a fourth
profile** alongside state-apply / state-propose / interaction / behavior.
This is unsettled in the master spec: the notes flag the participation/
free-rider problem (§6) as load-bearing for whether the master spec
even names this profile.

### Option A — Maintenance as a fourth component profile (PR #636 lean)

- **Mechanism.** Persister, snapshot provider, sync provider, replay
  buffer are WASM components in an app's bundle. Loaded by peers that
  opt to contribute, with kernel-known capacity hints. A peer's
  "participation" = set of maintenance components instantiated +
  declared capacity. Scaling = more peers running maintenance →
  more capacity, automatically.
- **Pros.** No separate work-tracking subsystem. Apps can ship custom
  maintenance logic (e.g. domain-specific snapshot strategy). Default
  client behavior: opt-in to cheap roles by default, "how much do you
  want to contribute" UI for expensive ones (disk-heavy persister,
  bandwidth-heavy sync provider).
- **Cons.** Adds a fourth profile to the determinism table (PR #636's
  defining table has three; a fourth would need its own determinism
  rules — likely "non-deterministic OK; runs per-peer" like behavior).
  Apps can't trust that any given peer is running them.
- **Sources.** `willow-pr-636.diff` lines 681-843 (research notes),
  `prior-art/willow/workers.md` §"Maintenance components".

### Option B — Maintenance baked into the kernel

- **Mechanism.** Persister, snapshot provider, sync provider, replay
  buffer are kernel subsystems. Apps don't ship them; every peer's
  kernel knows how to do them generically over the opaque event DAG.
- **Pros.** Determinism story stays simple. Every peer participates
  uniformly. No apps-must-include-maintenance burden on app authors.
- **Cons.** Kernel surface grows. Custom domain-specific maintenance
  (e.g. "this app prefers eventual-consistency snapshots over
  strict") is impossible. Doesn't compose with PR #636's
  "state-component owns persistence shape" choice.
- **Sources.** Today's `willow-replay` and `willow-storage` are
  effectively this — generic over `EventDag<ServerState>`.

### Option C — Maintenance is a deployment role of behavior (collapse)

- **Mechanism.** The fourth profile collapses into behavior; "I am a
  persister" is just a behavior component running on a designated peer
  with the right host imports. No new profile needed.
- **Pros.** Smaller spec surface. Already the natural shape for
  long-running, non-deterministic, peer-specific work — which is what
  behavior is for.
- **Cons.** Loses kernel-known capacity hints (peer announces "I have
  10GB persister capacity" — that's a kernel-aware concept). Loses
  the fourth-profile crispness.
- **Sources.** `prior-art/willow/runtime-vision.md` §"The four
  component profiles" (note the open-ness about whether it's a fourth
  or just a deployment role).

### Willow position

Unsettled. PR #636 master spec defers; the research notes lean A but
flag dependence on §6.

### Re-evaluation question

The collapse-to-behavior (C) is conceptually clean; the dedicated
fourth-profile framing (A) is more discoverable and gives apps a clear
SDK surface ("export a `maintenance:persister` interface"). The
participation/Sybil decision (§6) is the gating constraint: if Myrhiza
adopts a participation primitive that needs kernel-aware capacity
declarations (e.g. "refuse-to-serve-non-participants" — see §6), then
A is preferred. If participation collapses into "behavior peers do
the work" with no kernel-aware accounting, C is preferred.

---

## 6. Sybil-resistant participation enforcement (the load-bearing one)

The hard problem PR #636 flagged for the next session. Free-rider
tolerance is high for chat-shape apps (most users contribute nothing
and that's been fine for Slack/Discord); the only adversary that
matters is **automated abuse at scale**. But Myrhiza's ambition isn't
limited to chat-shape: high-stakes workflows (financial, contractual,
data sovereignty) have lower free-rider tolerance.

The literature gives 20+ years of designs. **Pick one or hybrid.**

### Option A — Tit-for-tat / pairwise reciprocity (BitTorrent, Bitswap)

- **Mechanism.** Each peer tracks per-connection contribution ledger
  with each neighbor: "I've sent you N bytes / blocks; you've sent me
  M." When deciding whom to serve, prefer peers in the top contribution
  band; choke peers who haven't reciprocated. BitTorrent's choking
  algorithm uses a 30-second rotating top-K; Bitswap uses ledger-based
  block-exchange.
- **Pros.** **Sybil-tolerant per-connection** — cheating identities each
  get nothing individually until they upload. No global identity, no
  reputation gossip required. Most-deployed answer in P2P (BitTorrent
  swarms have routinely hit hundreds of millions of clients).
- **Cons.** Works for **symmetric workloads** (you have what I want, I
  have what you want). Maintenance work is asymmetric — a snapshot
  provider isn't asking the joiner for anything in return. Pure
  tit-for-tat doesn't fit cleanly for sync-provider / persister roles.
  Bandwidth-only metric ignores compute and storage cost.
- **Sources.** BitTorrent (Cohen 2003), PropShare, Bitswap (IPFS),
  `willow-pr-636.diff` lines 746-757, 807-810.

### Option B — Reputation / trust aggregation (EigenTrust, BarterCast)

- **Mechanism.** Peers track local-view contribution observations;
  gossip aggregates into global trust scores via eigenvector iteration
  (EigenTrust) or local-only (BarterCast). High-reputation peers get
  served preferentially.
- **Pros.** Aggregates information across the network — long-tail
  contributors get credit. Asymmetric workloads work fine
  (reputation isn't tied to direct exchange).
- **Cons.** **Sybil-vulnerable** without identity cost — minting more
  identities and gossiping favorable reports inflates score.
  EigenTrust is the canonical ref but "Sybil-vulnerable but canonical"
  is the literature's framing. Variants (PowerTrust, PeerTrust) add
  Sybil-hardening at the cost of complexity.
- **Sources.** EigenTrust (Kamvar et al. 2003), BarterCast (Tribler),
  `willow-pr-636.diff` lines 759-766.

### Option C — Social-graph Sybil resistance (SybilGuard, SybilLimit, Whanau) — **the unique-advantage option**

- **Mechanism.** Use Willow's existing permission/invite trust graph
  as the social-graph input. SybilGuard / SybilLimit (Yu et al.
  2006/2008) bound the number of Sybil identities that can attach to
  the honest region of a social graph. Whanau is a Sybil-proof DHT
  built on a trust graph.
- **Pros.** **Willow has this trust graph for free** — most P2P systems
  bootstrap a social graph they don't have. Permission/invite events
  in the DAG ARE the trust graph. Theoretically sound (the Yu et al.
  papers prove worst-case Sybil bounds under reasonable graph
  assumptions). PR #636 explicitly calls this out as "the unique
  advantage."
- **Cons.** Doesn't apply to public/permissionless apps (no invite
  graph means no social-graph signal). The trust graph reflects "who
  trusts whom for app authority" not necessarily "who is trustworthy
  for resource contribution" — different signals. SybilGuard/Limit
  algorithms have non-trivial implementation complexity.
- **Sources.** SybilGuard (Yu et al. 2006), SybilLimit (Yu et al.
  2008), Whanau (Lesniewski-Laas 2010), `willow-pr-636.diff` lines
  777-785, `prior-art/willow/open-problems.md` §"Distributed
  maintenance" (calls C the "strong yes candidate").

### Option D — DHT-responsibility / coordinated allocation (Holochain)

- **Mechanism.** Every node validates / stores entries it's
  "responsible for" in the DHT (arc-overlap with the entry's hash).
  Coordinated allocation — nobody chooses to free-ride because their
  arc deterministically picks their work for them.
- **Pros.** **Closest existing precedent structurally** — the runtime
  and the maintenance role are unified through arc-responsibility.
  Holochain has shipped this; the lessons probably translate directly.
- **Cons.** Coupled to sharding (§1 option C). Eclipse attacks on
  neighbor selection are real. Validation per neighborhood has
  non-trivial cost. Storage cost grows with arc size. The unfinished
  partial-arc story in Holochain is a six-year cautionary tale.
- **Sources.** `prior-art/holochain/networking.md`, `prior-art/
  holochain/open-problems.md` §3, §5, `willow-pr-636.diff` lines
  799-805.

### Option E — Storage proofs (Filecoin / Storj / Sia)

- **Mechanism.** Cryptographically verifiable storage claims (proofs
  of replication, proofs of spacetime). Peers periodically prove they
  still hold the data they claim.
- **Pros.** Strongest durability guarantee in the literature.
  Sybil-resistant by construction (storage cost is the resource gate).
- **Cons.** **Heavy machinery, almost certainly overkill** for any
  Myrhiza app with chat-shape free-rider tolerance. Filecoin's
  PoRep/PoSt are research-grade complexity; lighter audit-style
  schemes (challenge-response over stored data) are plausible but
  still substantial work.
- **Sources.** Filecoin, Storj, Sia, `willow-pr-636.diff` lines
  790-797.

### Option F — Hybrid (likely actual answer)

- **Mechanism.** Combine. Two plausible shapes:
  - **(A) + (C):** tit-for-tat for symmetric blob exchange (data-plane
    fairness); social-graph Sybil filter for which peers can request
    sync-provider services at all (admission control).
  - **(C) + audit-style storage proofs:** social-graph admission +
    challenge-response over claimed-stored data for accountability.

### Willow position

Unsettled — the next-session research agenda. PR #636 research notes
explicitly defer the master-spec section.

### Decision points the brainstorm must resolve

1. **Free-rider threat model.** Is Myrhiza's threat model "automated
   abuse at scale" (chat-shape, narrow) or "high-stakes adversarial
   participation" (financial, broad)?
2. **Social-graph yes/no.** Commit to leveraging the permission/invite
   trust graph as Sybil input, or explicitly reject? PR #636's
   research notes lean strongly toward yes; the open-problems doc
   names it the strong-yes candidate.
3. **Master-spec level vs child-spec.** Does Myrhiza ship a
   participation primitive at master-spec level, or defer the entire
   thing to a child spec and master-spec just commits to "we will
   solve this"?
4. **Asymmetric workload.** Pure tit-for-tat doesn't fit
   maintenance. The workable pattern is (admission control by
   social-graph) + (data-plane fairness by tit-for-tat where
   symmetric, by capacity advertising + audit where asymmetric).

### Sources for §6

- `willow-pr-636.diff` lines 681-843 — full research notes.
- `prior-art/willow/open-problems.md` §"Distributed maintenance".
- `prior-art/willow/workers.md` §"Maintenance components".
- `prior-art/holochain/open-problems.md` §1, §2, §3.
- `prior-art/iroh/lessons.md` §"Avoid: Sybil resistance is none, by
  design."
- BAR Gossip (Aiyer et al. 2005), BitTorrent choking (Cohen 2003),
  EigenTrust (Kamvar et al. 2003), SybilGuard/Limit (Yu et al.
  2006/2008), Whanau (Lesniewski-Laas 2010), Adar & Huberman
  "Free Riding on Gnutella" (2000).

---

## 7. Worker fuel + resource limits

PR #636 names the question, defers defaults to child specs.

### Option A — Per-instance fuel + memory, sync-budget shared with apply

- **Mechanism.** Each `state-apply` invocation gets a fixed
  instruction-count budget (wasmtime epoch-based interruption) and a
  fixed linear memory cap. Pre-check shares the budget. Fuel exceeded
  → trap → event rejected (failure-closed semantics from PR #636).
- **Pros.** Simple, conservative. Same defaults across all apps. Fuel
  is deterministic-by-spec → cross-peer convergence preserved under
  exhaustion (every peer rejects the same way).
- **Cons.** Conservative defaults fail real workloads with bursty
  applies. UI lag if pre-check shares budget with apply.

### Option B — Per-instance fuel + memory, separate pre-check budget

- **Mechanism.** Pre-check has a larger budget than apply (it runs on
  one peer, in user-interactive context, before signing). Apply has a
  tighter budget (must converge on every peer).
- **Pros.** Better UX — pre-check can run heavier validations.
- **Cons.** More knobs. Asymmetric budgets must still be deterministic
  to spec or peers diverge.

### Option C — Per-app-bundle fuel pool + per-instance allocation

- **Mechanism.** App declares total fuel budget per peer per
  unit-time. Worker allocates from the pool to topic instances.
  Fair-share when budget is exhausted.
- **Pros.** Apps can self-tune. Operators can deny apps that demand
  too much.
- **Cons.** Adds scheduler surface to the kernel. Cross-peer
  convergence requires deterministic allocation rule (hard).

### Willow position

Open. PR #636 §"Open questions deferred" lists "Resource limits:
per-instance fuel and memory budgets — what defaults?"

### Sources

- wasmtime epoch-based interruption (`prior-art/willow/open-problems.md`
  §"Resource-limit defaults"), Spin's resource limits, wasmCloud
  capability-provider quotas, Cloudflare Workers production data.

### Defaults — actual numbers from prior art

- Holochain Kitsune2 raised gossip rate cap from kitsune1's 0.5 Mbps
  recent / 0.1 Mbps historic to **10 Mbps for all data with 1 GB
  burst over 10s** (`prior-art/holochain/networking.md` §"Bandwidth
  and rate limiting"). Per-peer-pair, symmetric. Useful starting point
  for sync-bandwidth caps.
- Hyperswarm `connectionKeepAlive: 5000ms`, `randomPunchInterval:
  20000ms` — empirically tuned mobile defaults
  (`prior-art/pears/transport-comparison.md`).

---

## 8. Browser peer connectivity

Myrhiza's app-shape ambition includes browser apps. iroh handles WASM
transport differences internally per Willow CLAUDE.md, but the actual
pipe is constrained:

### Option A — Relay-bridged QUIC (iroh today)

- **Mechanism.** Browser-side iroh peer compiles to
  `wasm32-unknown-unknown` via wasm-bindgen, speaks WebSocket to relay
  servers (iroh-relay protocol is HTTP/HTTPS-upgrade-capable), runs
  in **relay-only mode** — direct connections and hole-punching are
  disabled because browsers can't open raw UDP. End-to-end encryption
  holds.
- **Pros.** Already shipped (iroh 0.32 alpha, 0.33 beta). Same
  `EndpointId` identity model as native peers. Already used by Willow
  client.
- **Cons.** Every byte through a relay → bandwidth cost on relay
  operator. No path-upgrade story. WebTransport-backed transport
  doesn't exist.
- **Sources.** `prior-art/iroh/transports.md` §"WebTransport / browser
  viability", `prior-art/iroh/mobile-and-wasm.md`.

### Option B — WebTransport (browser-native QUIC) — not available

- **Mechanism.** Browser-native WebTransport API speaks QUIC directly
  to peers presenting valid TLS certs. Theoretical fit for iroh's
  pubkey-as-identity, but WebTransport requires DNS names + valid TLS
  certs, which **defeats pubkey-as-identity for arbitrary devices**
  (the iroh 0.32 post explicitly notes this trade-off).
- **Pros.** Direct path possible (no relay).
- **Cons.** Doesn't actually exist as an iroh transport. The reverse
  direction (`web-transport-iroh`) is an experiment expressing
  WebTransport semantics over iroh, not using browser WebTransport
  as iroh's transport.
- **Sources.** `prior-art/iroh/mobile-and-wasm.md`.

### Option C — WebRTC fallback

- **Mechanism.** Browser peers use WebRTC data channels for direct
  paths. Phase 3 of Iroh's "Iroh & the Web" roadmap explores this;
  not shipped.
- **Pros.** Direct path between browser peers possible.
- **Cons.** Not shipped. WebRTC has its own NAT-traversal stack
  (ICE/STUN/TURN) that doesn't compose with iroh's. Holochain shipped
  WebRTC via tx5 then migrated to iroh — the lesson is "WebRTC was
  the long detour, not the answer."
- **Sources.** `prior-art/holochain/networking.md` §"Transport"
  (tx5 → iroh), `prior-art/iroh/mobile-and-wasm.md`.

### Willow position

A is shipped. Willow's chat client runs in browser via relay-bridged
QUIC.

### Implication for Myrhiza

**Browser-hosted Myrhiza apps are viable but relay-bound.** All peer
traffic for browser tabs goes through n0 relay infrastructure (or
Myrhiza-operated alternative). The relay-fleet ops decision is
load-bearing for the browser story. Two-tier delivery (iroh for live
realtime, push notifications via APNs/FCM for wake-from-cold) is the
realistic shape — Delta Chat's pattern.

### Sources

- `prior-art/iroh/mobile-and-wasm.md`, `prior-art/iroh/transports.md`,
  `prior-art/iroh/nat-traversal.md`.

---

## 9. Topic-ID rotation through dumb relays

PR #636 calls out: app-driven topic-ID rotation (epoch-rotation spec)
must work without leaking next-topic IDs publicly. The relay no longer
runs app code under the runtime model, so it can't be told the next ID
directly.

### Option A — Member-announced via pre-rotation session

- **Mechanism.** Before rotation, members announce the next topic to
  their existing relay session over an authenticated channel. Relay
  receives an opaque "after rotation, listen for topic X' as well as
  X" instruction; non-members never see X' on a public discovery
  channel.
- **Pros.** Preserves rotation unlinkability. Doesn't require relay
  to run app code. Member-driven — kernel is not in the loop.
- **Cons.** Members must coordinate the announce; one missed member
  breaks them out of the topic. Reconnect across rotation boundaries
  needs handshake design.
- **Sources.** PR #636 lines 535-550 (master commitment),
  `willow/docs/specs/2026-04-24-epoch-key-rotation.md` (existing
  spec needing re-shape), `prior-art/willow/open-problems.md`
  §"Topic-ID rotation".

### Option B — Tor-style hidden-service descriptor rotation

- **Mechanism.** Members compute the next topic ID from a shared
  secret + epoch counter; only members know the secret. Non-members
  cannot predict the next ID. Relays serve descriptors keyed by
  epoch-derived IDs.
- **Pros.** Production precedent (Tor). Algorithmically clean.
- **Cons.** Requires shared epoch secret distribution — composes with
  MLS-style group keying. Heavier crypto contract.
- **Sources.** Tor hidden-service descriptor rotation (cited in
  Willow open-problems).

### Willow position

Open. Existing epoch-rotation spec relied on relay running app code;
that no longer holds under runtime.

### Sources

- PR #636 lines 535-550, willow/docs/specs/2026-04-24-epoch-key-rotation.md.

---

## Open questions Myrhiza must answer

These cut across the domains above:

1. **Sharding model — commit or defer?** Holochain shipped without
   sharding; six years later partial arcs still aren't load-tested. If
   Myrhiza defers, name the scale ceiling explicitly.
2. **Worker model at v1 — A (trusted Rust) or B (untrusted WASM)?**
   PR #636 reads as B, but B is materially heavier. Trade-off: ship
   faster on A, migrate later (Willow's path) vs. start clean on B.
3. **Maintenance-as-fourth-profile or collapse-into-behavior?** §5
   Option A vs C. Gated by §6.
4. **Sybil-resistance choice.** Master-spec commit to social-graph
   (Option C, the unique advantage), or defer to child spec? Most of
   the brainstorm hangs on this.
5. **Free-rider threat model breadth.** Chat-shape (narrow) vs broader
   (financial, contractual). Determines whether storage proofs are
   ever in scope.
6. **Browser two-tier delivery.** Does the master spec name a
   wake-from-cold mechanism (push-notification capability), or stays
   silent and lets each app handle it?
7. **Topic-ID rotation protocol shape.** Member-announced (A) or
   epoch-derived (B). Composes with crypto/identity cluster.
8. **Relay metadata privacy.** iroh relays see "NodeID A talks to
   NodeID B at time T." Does Myrhiza's threat model accept this?
   Padding/cover traffic is app-layer; document it.

## Cross-domain interactions

- **§1 sharding ↔ §6 Sybil ↔ §3 sync.** Choosing arc-based sharding
  (Holochain-shaped) effectively chooses DHT-responsibility Sybil
  defense; choosing per-author DAG sync excludes range-based
  reconciliation as the canonical sync method.
- **§4 worker WASM ↔ §7 fuel ↔ determinism cluster.** Untrusted WASM
  workers require deterministic fuel-by-spec; this couples to the
  state-apply determinism story (sibling report).
- **§5 maintenance-as-component ↔ §6 participation.** If maintenance
  is a fourth profile with kernel-aware capacity declarations, the
  participation primitive can rely on those declarations; if
  maintenance collapses into behavior, participation has nothing
  kernel-known to refer to.
- **§2 relay ↔ §8 browser.** Browser peers always relay; relay-fleet
  ops decisions cascade into the browser story.
- **§9 topic-ID rotation ↔ identity cluster.** Rotation-secret
  distribution composes with multi-device identity (sibling cluster).
- **Worker fuel determinism ↔ identity cluster.** Behavior identity
  per-(peer, instance) means each behavior instance is independently
  rate-limited; ties into the unified multi-device + behavior key
  custody story.

## Brainstorming questions

Rough order of dependency:

1. What is the app-shape ambition? Chat-shape (free-rider tolerant)
   or broader (financial / data sovereignty)? Determines §6 threat
   model.
2. Do we leverage the permission/invite trust graph as social-graph
   Sybil input — yes or explicit no? (PR #636 names this as the
   unique advantage; rejection would be a deliberate choice.)
3. Does Myrhiza ship a participation primitive at master-spec level,
   or defer to a child spec? §5+§6 outcome.
4. Maintenance: fourth component profile, or deployment-role of
   behavior? §5.
5. Workers at v1: trusted-Rust (Willow's path) or untrusted-WASM
   (PR #636 commit)? §4.
6. Sharding: commit to a model, or "every peer holds everything for
   their topics" with explicit scale ceiling? §1.
7. Sync protocol: keep per-author `HeadsSummary` (Willow) or
   generalize to RBSR? §3.
8. Topic-ID rotation: member-announced vs epoch-derived? §9.
9. Browser two-tier delivery: master-spec or app-level? §8.
10. Relay model: dumb-bridge only, or also a privileged
    `SyncProvider` role? §2. Cross-cuts §4.
11. Fuel + memory defaults: name them, or defer? §7.
12. Maintenance-component capacity declarations: kernel-aware (gates
    §5+§6 hybrid), or app-internal? §5.
13. Does the master spec name a deny-list / app-allow-list operator
    surface for workers, or leave it to operator config? §4.
14. Worker capability advertisement: ship with v1 or defer to child
    spec? §4 option C.
15. Pure-relay-only mode for high-threat-model apps (Tor-on-iroh):
    master spec hook or out-of-scope? §2 cross-cuts custom-transport
    cluster.

## Sources

Primary:

- `/tmp/willow-pr-636.diff` — full PR #636, especially:
  - Lines 421-444 — "What changes about Willow" (worker trust model
    shift, kernel crate emergence).
  - Lines 535-550 — "Relays are gossip-driven, not state-driven."
  - Lines 619-625 — child specs list (worker-as-untrusted-WASM,
    relay-and-rotation, state-materialization-on-workers).
  - Lines 681-843 — `research-notes-distributed-maintenance.md`
    (157-line research notes, the load-bearing open-problem
    document).

Prior-art folders consulted:

- `prior-art/iroh/transports.md`, `blobs.md`, `gossip.md`,
  `lessons.md`, `nat-traversal.md`, `mobile-and-wasm.md`.
- `prior-art/holochain/networking.md`, `lessons.md`,
  `open-problems.md`.
- `prior-art/pears/hyperswarm.md`, `hypercore-stack.md`,
  `transport-comparison.md`.
- `prior-art/willow/networking.md`, `workers.md`, `open-problems.md`,
  `lessons.md`, `runtime-vision.md`.

Academic / literature:

- HyParView (Leitão et al. 2007) — partial-view membership.
- Plumtree (Leitão et al. 2007) — epidemic broadcast tree.
- BitTorrent choking algorithm (Cohen 2003) — pairwise reciprocity.
- EigenTrust (Kamvar, Schlosser, Garcia-Molina 2003) — global trust
  via gossip eigenvector.
- BarterCast (Tribler) — local-view reputation.
- BAR Gossip (Aiyer, Alvisi, Clement, Cowling, Dahlin, Riché 2005) —
  Byzantine-Altruistic-Rational framework.
- SybilGuard (Yu, Kaminsky, Gibbons, Flaxman 2006) — social-graph
  Sybil resistance.
- SybilLimit (Yu, Gibbons, Kaminsky, Xiao 2008) — improved bounds.
- Whanau (Lesniewski-Laas 2010) — Sybil-proof DHT.
- Filecoin proofs of replication / proofs of spacetime — verifiable
  storage.
- Adar & Huberman, "Free Riding on Gnutella" (2000) — measurement
  paper.
- Range-Based Set Reconciliation (Meyer 2022) — RBSR foundations.
- BLAKE3 specification — content-addressed hashing.
- Bao verified streaming — chunk-granular verification format.

Cross-references:

- `references/local-first.md` — anchor index.
- `CLAUDE.md` (Myrhiza) — locked decisions: iroh as transport,
  capabilities-only host surface, determinism for state-apply.
