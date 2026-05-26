**Date:** 2026-05-22
**Status:** active
**Subject:** What BEAM/OTP doesn't structurally solve, and what Myrhiza inherits if it borrows OTP patterns

# Open problems

This is the "what's broken or unsolved in OTP, and what does that mean for us" file. Pairs with [`critiques.md`](critiques.md) (which catalogues weaknesses) — open-problems is the prospective view: what hasn't been solved at all.

## Capability discipline at the runtime layer

BEAM does not have capabilities. Process identifiers are forgeable across the wire (cookie-trusted). There is no per-process permission model. Any node in the cluster can RPC any module on any other node — including arbitrary `os:cmd`.

**What's been tried:** various community capability libraries (e.g. `safe_erlang` experiments, the SES-equivalent attempts). None landed in tree.

**What Myrhiza inherits if it copies BEAM's distribution model:** the same problem. Don't. **The capability discipline must come from elsewhere — Component Model + WASI capabilities, OCapN for cross-peer, Spritely/Agoric's discipline for object-graph integrity.** OTP can teach us supervision and message-passing; not authority.

## Distributed leader election

There is no in-tree consensus primitive. `global` is a poor-man's lock; `ra` is a third-party library; external consensus stores (etcd, ZooKeeper) are used in many production deployments.

**Myrhiza inherits this problem.** If behaviour-coordination requires a leader (e.g. exactly-one bot instance for `chat/general`), neither OTP nor any of the prior-art-folder targets gives us a turnkey answer. The options are the same as in the OTP world: Raft library, external coordinator, or "design around the need for a leader." Myrhiza spec authors will have to make the same choice; see [`prior-art/willow/open-problems.md:303`](../willow/open-problems.md).

## Live schema migration with rollback

`code_change/3` migrates state from old shape to new. Does **not** give you rollback: if the new shape is wrong and you want to downgrade, you have to author the *downgrade* `code_change/3` callback in advance (most teams don't). Once you've upgraded a node, downgrade is "restore from backup."

**Myrhiza decision point:** when designing hot-reload v2, decide whether downgrade is in scope. Most production OTP shops have effectively decided "rollback = restore from backup." If Myrhiza wants better, that is a research-grade problem — there is no off-the-shelf solution to lift.

## Cross-node hot code loading

Hot loading is per-node. To upgrade a cluster, you upgrade each node separately. The `release_handler` does this; the network sees a brief mixed-version cluster. **There is no automatic version-skew handling between in-flight messages from old-version nodes to new-version nodes.**

In practice this is fine if the message ABI is stable across the upgrade (the usual case). It breaks horribly if the message shape changes — you get malformed-message crashes during the cluster upgrade window.

**Myrhiza inherits this if it borrows cluster-wide hot reload.** Mitigation in the OTP world: never change message shape without an explicit version-tag field, and have new-version code parse old-version messages indefinitely.

## Backpressure as a first-class concept

BEAM mailboxes are unbounded by default. The community has built `GenStage` / `Flow` (Elixir) and `gen_buffer` (Erlang) to add backpressure on top, but it is not the runtime's contract. **The runtime contract is: "if you send to a slow consumer, the producer is fine and the consumer is fine and the system OOMs."**

**Myrhiza must design backpressure into the kernel from day one.** This is a hard "do not borrow." Event queues should be bounded, with explicit backpressure semantics, and the kernel should refuse to enqueue when the bound is hit (with a back-channel to the producer). See [`critiques.md`](critiques.md) "Mailbox-as-unbounded-queue."

## Determinism

BEAM is non-deterministic. Process scheduling order, mailbox arrival order, network arrival order, GC timing — all observable, all variable run-to-run. Replay-driven debugging is hard.

**Myrhiza requires determinism in the `state-apply` profile.** OTP gives us nothing here; SwingSet's transcript-driven vat replay ([`prior-art/agoric-endo/determinism.md`](../agoric-endo/determinism.md)) is the model. **Strict determinism is the property where BEAM is the worst influence in the corpus** — its idioms encourage exactly the non-determinism Myrhiza forbids in state-apply components.

## Garbage collection across distributed actors

BEAM does per-node, per-process GC. There is **no distributed garbage collection** — if process A on node X holds a reference to process B on node Y, and Y's process dies, A keeps the dead reference (which `Pid ! Msg` to a dead pid silently drops). Monitors (`erlang:monitor/2`) compensate but require explicit setup.

In practice this is fine because BEAM's distributed mode is trust-network-only and clusters are small. **Open internet peer-to-peer with cap-graph integrity is a different problem.** Spritely solves acyclic distributed GC; cycles are an open research problem (Pekko's CRGC is the closest production-grade attempt). See [`prior-art/spritely-ocapn/captp-and-ocapn.md`](../spritely-ocapn/captp-and-ocapn.md).

## Cross-version protocol compatibility

If your cluster runs nodes on OTP 27 and OTP 28 simultaneously, **Distributed Erlang is supposed to interoperate** but the support window is narrow (typically two adjacent major versions). External-format (`term_to_binary`) versioning has historically been done by extending tags, not by version-negotiated handshake. The "we add new tags, old nodes ignore them" pattern has held up but is fragile.

**Myrhiza, with bigger version-skew expectations (peers update on their own schedules), needs explicit protocol versioning at the wire level.** Don't borrow BEAM's "ignore unknown tags" pattern; it's a footgun for adversarial inputs.

## Browser viability

BEAM does not run in the browser. Period. The Lisp Flavoured Erlang community has experimented with cross-compiling BEAM bytecode to JS; the Elixir community has `firefly` (Elixir-to-WASM compiler, abandoned 2023); none landed in usable form.

If your runtime story requires browser tabs to be first-class peers, BEAM is not your substrate. **WASM Component Model is — this is exactly the gap CM fills that BEAM doesn't.** See [`prior-art/wasm-component-model/`](../wasm-component-model/).

## Multi-tenancy / sandboxing within a node

BEAM has no concept of "this code runs in this tenant's heap with this resource budget." Reduction-counting limits per-process CPU implicitly; there is no built-in memory cap per process; there is no scheduling priority that's effective at preventing one tenant from starving others under load.

**The OTP world's "solution" is one-node-per-tenant.** For Myrhiza's "many apps per peer" model, this is the wrong shape. We need real per-component resource accounting — fuel-style for CPU, hard memory caps, deterministic scheduling. WASM's existing primitives (fuel, epoch deadlines, memory pages) are a better starting point.

## Type safety across the upgrade boundary

`code_change/3` callbacks are user-written and untyped. Nothing prevents you from writing a `code_change/3` that silently transforms `#user{name=N, email=E}` into `#user{name=N, email=N}` (typo). The bug surfaces on first invocation of any callback that needs the email.

The community's mitigation is `dialyzer` + careful code review. Neither is a runtime check.

**Myrhiza's WIT-typed interfaces give us better tooling here**, but the migration function itself is still user-written. The honest read: typed migration is a small win over BEAM, not a structural one. The deeper question — "how do we validate a migration is correct before activating it?" — is open everywhere.

## Hot-reload as marketing vs. reality

Restated for emphasis from [`hot-code-loading.md`](hot-code-loading.md): **the canonical reference for hot code loading has been retreated from by most of its production user base.** This is the load-bearing meta-lesson: don't sell a feature you can't operate.

Myrhiza's hot-reload v2 should be designed *expecting* that most apps will choose restart-based deploys, not in-place upgrade. Make restart cheap and reliable; make in-place upgrade an opt-in for the small subset of apps where its costs are justified.

## Tooling for runtime invariant checks

BEAM has `dialyzer` (static), `proper` / `eqc` (property-testing), and at-runtime nothing. There is no "this gen_server should never receive a message of type X in state Y" runtime invariant that the supervisor enforces.

**Myrhiza, with WIT typing + WASM Component Model resource handles, can do better here at the substrate level — but the high-level "behaviour invariant" question (does the state machine actually obey its contract?) is unsolved on every runtime.** This is research-grade. Expect to ship without it in v1.

## Sources

- "Why no distributed GC in Erlang?" discussion (erlang-questions list, periodic): <https://erlang.org/pipermail/erlang-questions/>
- Backpressure patterns in Erlang (Sasa Juric, 2017): <https://www.theerlangelist.com/article/spawn_or_not>
- Akka CRGC (cycle-collecting distributed GC) — Pekko fork: <https://github.com/apache/pekko>
- `ra` Raft library: <https://github.com/rabbitmq/ra>
- Firefly Elixir compiler (abandoned 2023): <https://github.com/GetFirefly/firefly>
- Cross-references: [`hot-code-loading.md`](hot-code-loading.md), [`critiques.md`](critiques.md), [`distribution.md`](distribution.md)
