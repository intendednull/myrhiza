**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/SwingSet — comms vat, mailbox, off-machine routing, OCapN co-design

# CapTP and Network

For Agoric, "the network" is one specific vat. Off-machine messaging is implemented entirely inside the **comms vat**, with bytes going onto the wire through a non-vat **mailbox device**, and the host application's after-commit hook moving those bytes to whatever transport actually carries them. CapTP (the Capability Transport Protocol) is the *idea* — capability-passing, promise-pipelining, four-tables refcounting — that the comms vat embodies. The current concrete wire format is a SwingSet-specific text protocol; the cross-implementation OCapN format that Agoric is co-designing is a separate effort still pre-1.0.

For Spritely's view of CapTP/OCapN, the wire-level details, sturdyrefs, and netlayers, see [`../spritely-ocapn/captp-and-ocapn.md`](../spritely-ocapn/captp-and-ocapn.md). This file deliberately does not duplicate that material; it focuses on what Agoric does that Spritely does not.

## The comms vat

The comms vat is just another vat from the kernel's perspective — same `dispatch`/`syscall` API, same crank model. What's different ([`comms.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/comms.md)):

- Its source code does **not** use liveslots; it has a `setup()` default export (the legacy path) and does its own marshalling. Going through the standard liveslots serialize/deserialize path would be wasteful for what is effectively a packet-shuffling vat.
- It runs with `enablePipelining: true`. Liveslots vats default to `false`. The comms vat is currently the only production vat where pipelining is on, because pipelining matters most when the round-trip is *off the machine*. ([`delivery.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/delivery.md))
- It maintains *two* c-list-shaped tables: one facing the kernel (mapping local `o/p` IDs to its internal `lo/lp` namespace) and one for each remote machine (mapping `lo/lp` to per-peer `ro/rp`). The local namespace deduplicates references that arrive from multiple peers about the same object.
- It's the only vat that needs both `local`/`xs-worker` semantics and full read access to inbound bytes — so the host gives it the mailbox device.

There is **one comms vat per machine**. All off-machine traffic funnels through it. This is the centralization point that lets capability-discipline survive across machines: every cross-machine reference passes through the comms vat's c-list, so a peer cannot fabricate a reference to anything not previously granted, just as a vat cannot fabricate a kernel reference.

## Naming the same object across the boundary

The comms vat's docs walk through how a single object on Vat A on Machine A is renamed at every boundary:

- in vat `a`: `o+1` (vat export 1)
- in the comms vat on Machine A: `o-2` (import 2 from kernel)
- on the wire to B: `ro-3` (remote-object ingress 3, "I, machine A, sent it")
- in the comms vat on Machine B: `o+4` (vat export 4)
- in vat `b`: `o-5` (vat import 5)

When B sends *the same object* back, it appears as `ro+3` over the wire — sign flipped because the receiver allocated it. The comms wire protocol is "exceedingly polite": references are always emitted in the format the receiver will recognize. ([`comms.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/comms.md))

`ro` = remote object, `rp` = remote promise. Sign: `+` = receiver allocated this number, `-` = sender allocated. The asymmetry the kernel/vat boundary has (vat sees `o+`/`o-` based on who allocated, regardless of direction) is replaced by direction-dependent sign-flipping at the comms vat boundary, because here the two parties are peers, not asymmetric.

## The current wire format

Per [`comms.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/comms.md), the protocol is single-line strings, newline-delimited:

```
deliver:${remoteTargetSlot}:${remoteResultSlot}:${slots..};${methargs.body}
resolve:object:${target}:${resolutionRef};
resolve:data:${target}:${slots..};${resolution.data.body}
resolve:reject:${target}:${slots..};${resolution.data.body}
```

A worked example. Machine A invokes `E(target).foo(1, 2, bar)` against an object `target` on B (known locally as `ro-1`), retaining the result promise as `rp+3`, and `bar` is an object on A:

```
deliver:ro+1:foo:rp-3:ro-2;[1,2,{"@qclass":"slot","index":0}]
```

`@qclass` is the marshalling library's tag for "this isn't plain JSON; this slot at index N is a capability reference, look it up in the slots array."

This is a SwingSet-internal protocol — not portable, not a standard, and not the same thing as OCapN's CapTP wire format. The doc closes with: *"In the E version of CapTP, there are four tables: questions, answers, imports, and exports. We only have ingresses and egresses."* The Agoric comms protocol explicitly does not implement the full E-style four-tables CapTP; it implements a simplified two-direction (ingress/egress) variant sufficient for SwingSet's needs.

## The mailbox device — the actual byte-on-wire boundary

The comms vat does not call sockets. It calls into the **mailbox device** ([`devices.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/devices.md)), which is a slot of the kernel's durable state vector:

- `add(recipient, msgnum, body)` — comms vat puts an outbound message in the slot
- `remove(recipient, msgnum)` — comms vat acknowledges receipt of an *inbound* message (so the partner can stop retransmitting)
- `ackInbound(recipient, msgnum)` — comms vat sends an ack to the partner
- `deliverInbound(...)` — host pushes bytes from the wire into the kernel as a device input

The host application loop, after `controller.run()` and `hostStorage.commit()`, scrapes the outbound mailbox slot for new messages and does whatever it does — a local solo agoric writes to TCP/TLS or libp2p; an Agoric chain validator stores it in chain state and lets it ride on IBC. Inbound, the host parses transport-level messages and feeds them to `deliverInbound`.

This deliberately defers send until after commit. The architectural reason is **hangover inconsistency**, the failure mode where a host crashes after sending an outbound message but before persisting the state change that justifies the message. Per Agoric, "[w]e follow the lead of the Waterken and E systems… outputs must be embargoed until all consequences of an inbound delivery have been durably committed" ([`host-app.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/host-app.md)).

The mailbox is dumb; the host is smart. SwingSet does *not* know about TCP, TLS, libp2p, IBC, or any other transport. It knows about a slot in its state vector.

## Routing in different host topologies

### Solo agoric

A `solo` (single-process) agoric runs the kernel and one comms vat. The host loop:

1. Accepts incoming network connections (the host code, not the kernel).
2. Frames inbound bytes, calls `deliverInbound`.
3. `controller.run()` until quiescent.
4. `hostStorage.commit()`.
5. Reads the outbound mailbox; sends frames.

Originally TCP/TLS; the historical README notes "TCP/TLS (perhaps with `libp2p`) or other means."

### Chain validator

On a Cosmos-SDK chain validator, the host loop is `BeginBlock`/`EndBlock` lifecycle hooks. Inbound IBC packets arrive as Cosmos-SDK messages, get unwrapped, and become `deliverInbound` calls. Outbound mailbox contents become IBC packets routed to whichever destination chain runs the receiving SwingSet. The `agoric-upgrade-23-rc1` release notes ([2026-05-06](https://github.com/Agoric/agoric-sdk/releases/tag/agoric-upgrade-23-rc1)) describe upgrading cosmos-sdk to v0.53. Each Cosmos block is a SwingSet "block" in the run-policy sense, modulo computron-budget exhaustion.

The chain → chain routing is IBC, not CapTP-over-TLS. From the kernel's perspective this is invisible: it's still bytes in mailbox slots, same wire format. From the validator's perspective the comms vat *is* its IBC userspace.

This is also why the comms vat must be **deterministic** — it runs on every validator, and they all need to compute the same outbound mailbox bytes for consensus.

## Hostnames vs vatids vs cluster ids

SwingSet doesn't use a network-level "machine identity" string in the kernel. The comms vat learns about peers via:

- A **machine ID** (sometimes called "remote ID" in the codebase) assigned when the comms vat is told about a new remote machine.
- A bootstrap object reference exchanged at connection setup — the equivalent of a sturdyref, granted out-of-band.

Vat IDs (`v1`, `v2`, …) are local to one kernel; they have no meaning across machines. Cross-machine references go via the comms vat's per-peer `ro/rp` tables — two peers might both have a vat called `v3`, and they don't collide because neither one is named on the wire; only object references are.

The host transport (TCP socket, IBC channel, etc.) is the layer at which "machine identity" is enforced. Inside SwingSet, machine identity is a string the comms vat tags messages with.

## OCapN — the cross-implementation effort

[OCapN](https://github.com/ocapn/ocapn) is the Object Capabilities Network, a Spritely-led standardization effort to give CapTP a portable, cross-implementation wire format. Agoric is a co-author. As of 2026, Agoric staff are major OCapN contributors — `kriskowal`, `kumavis`, `dckc`, `erights`, and `gibson042` are all in the OCapN repo's top-15 contributor list ([github.com/ocapn/ocapn contributors](https://github.com/ocapn/ocapn/graphs/contributors)).

The Spritely write-up at [`../spritely-ocapn/captp-and-ocapn.md`](../spritely-ocapn/captp-and-ocapn.md) covers the wire-level OCapN details — `op:start-session`, `op:deliver`, the four tables, sturdyrefs, swiss numbers, netlayers, the Syrup encoding. We will not repeat them.

The Agoric-specific story:

- **Agoric does not use OCapN in production today.** SwingSet's comms vat speaks its own line-delimited protocol (described above). OCapN is co-designed but not deployed on-chain.
- **Why not?** OCapN is still pre-1.0 in 2026 ([draft specs](https://github.com/ocapn/ocapn) under `draft-specifications/`); the wire format and locator URL grammar have shifted multiple times. Agoric isn't going to ship a non-final standard into a chain that has billions of dollars on it.
- **What Agoric contributes.** The four-tables model in OCapN comes from E (Mark S. Miller's earlier work), which Agoric staff have direct lineage to — `erights` is Mark Miller. Agoric's marshal library (`@endo/marshal`) and the `Far`/`Remotable` discipline informed how OCapN handles capability-marked data. Agoric's experience deploying capability-passing systems at scale is the design pressure that keeps OCapN realistic.
- **What Agoric will use it for.** Plausibly: cross-chain capability passing (Agoric → another OCapN-speaking chain or app), and Agoric-as-server / Spritely-as-client desktop integrations. Both speculative. No public commitment to a switchover date.
- **Why this matters for prior-art.** OCapN is the standard the *capability-passing* community is converging on. If we expect components-on-different-Myrhiza-instances to talk to capability-passing systems outside Myrhiza, OCapN is the protocol to consider. If we don't expect that, OCapN is informative as design pressure, not as a runtime dependency.

The open question Agoric has not publicly answered: when (if ever) the comms vat will speak OCapN over the wire instead of (or alongside) its current line-delimited protocol. Probably waits on OCapN reaching 1.0, which has been sliding for several years.

## Honest unflattering bits

- **The comms vat protocol is not CapTP.** It's a simplified two-table variant. Agoric has been doing capability-passing across machines for years without implementing the full four-tables protocol. This is fine, but it's worth noting that "Agoric uses CapTP" in its current production form is a stretch; "Agoric uses a CapTP-inspired protocol" is honest.
- **OCapN has been pre-1.0 since 2022.** [NLnet grant ran Aug 2022 – Oct 2023](https://nlnet.nl/project/SpritelyOCapN/). It is now 2026 and there's still no 1.0. Agoric uses it for talks and design discussions; production is the SwingSet protocol described above.
- **Mailbox is brittle as a network abstraction.** The host has to handle retries, ack-tracking, reordering, multi-peer scaling, all without help from the kernel. This is fine for a chain (IBC handles all that) and tolerable for solo (TCP handles most), but bare-metal P2P would need a netlayer-style abstraction layer. The mailbox device exists, the abstraction does not.
- **No native NAT traversal, no native QUIC, no native libp2p.** Whatever the host can do, the host must do entirely outside SwingSet. This is by-design (kernel is small) but it means SwingSet on its own is not a P2P networking primitive.
- **Pipelining is theoretically there but mostly not used.** Every doc emphasizes pipelining; only the comms vat enables it. Application vats don't get the latency benefit, and Liveslots vats run with `enablePipelining: false`.
- **Retransmit semantics are duplicate-of-state-machine.** The mailbox device adds retry-and-ack, but that means the comms vat *and* the host *and* the underlying transport (TCP / IBC) all have retry and ack logic. Three layers. Fine in practice; ugly in design.

## Implications for Myrhiza

- **Adopt the after-commit-then-emit invariant. Non-negotiable.** Any outbound network capability call must be embargoed until the kernel has durably committed the event that produced it. This is the hangover-inconsistency defense, and it is the difference between "P2P that works" and "P2P that occasionally double-spends." Bake it into the kernel's capability runtime, not into individual capability implementations.
- **Use OCapN's netlayer abstraction, not SwingSet's mailbox device, as the network model.** The mailbox device is a state-vector slot, which made sense when the host transport was already a chain. For a P2P runtime, we want a clean transport-pluggable interface (Tor, libp2p, QUIC, etc.) and netlayers are the better-designed primitive. See [`../spritely-ocapn/captp-and-ocapn.md`](../spritely-ocapn/captp-and-ocapn.md) for the Spritely netlayer interface.
- **Take the Agoric per-peer c-list pattern.** The comms vat keeps one c-list per remote peer, mapping a local namespace to that peer's view. This naturally supports peer-specific capability scoping ("this peer can reach object X but not Y") and makes per-peer GC tractable. Plan our cross-instance capability table the same way.
- **Steal the polite-direction-flip naming.** Always emit references in the *receiver's* numbering convention, with sign indicating who allocated. This drops a whole class of confusion bugs at the protocol layer.
- **Decide early whether we speak OCapN over the wire.** If yes, we're committing to track an unstable spec and likely be among the first non-Spritely-non-Agoric implementations. If no, we'll have our own protocol and need a story for interop later. The honest answer for Myrhiza right now is "not yet — design with OCapN-compatibility in mind, but don't block on it." Re-evaluate when OCapN hits 1.0.
- **Pipelining: implement it, design components to use it.** Agoric's regret implicit in the docs is that they shipped an asymmetric pipelining model where comms uses it and apps don't. We have a clean slate. Either pipelining is a first-class part of the cross-component invocation model or we leave it out — don't half-ship it.
- **The "everything is a vat" homogeneity is a feature.** The comms vat is just another vat. The timer vat is just another vat. The kernel is small, and *not* reimplementing networking in C++ is part of why. Myrhiza should preserve this: the network capability is implemented by a component, not baked into the kernel.
- **Determinism of the network component.** Whatever we use for the cross-instance capability bridge, it must be deterministic with respect to the events its state-apply path observes. Inbound bytes are an event; outbound bytes are a function of state and event. The comms vat is deterministic by virtue of running the same code on every chain validator. Our equivalent component must be too.
- **Don't conflate "wire format" and "transport."** OCapN keeps these separate (CapTP is the wire format; netlayers are transports). SwingSet's comms vat blurs them. Keep them separate in Myrhiza — capability-marshalling is one concern, byte transport is another, and pluggability of each independently is desirable.

## Sources

- [`packages/SwingSet/docs/comms.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/comms.md)
- [`packages/SwingSet/docs/devices.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/devices.md)
- [`packages/SwingSet/docs/delivery.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/delivery.md)
- [`packages/SwingSet/docs/host-app.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/host-app.md)
- [agoric-upgrade-23-rc1 release notes](https://github.com/Agoric/agoric-sdk/releases/tag/agoric-upgrade-23-rc1) — published 2026-05-06; cosmos-sdk v0.53 upgrade
- [OCapN repository](https://github.com/ocapn/ocapn)
- [OCapN contributors graph](https://github.com/ocapn/ocapn/graphs/contributors) — confirms `kriskowal`, `kumavis`, `dckc`, `erights`, `gibson042` as Agoric-staff contributors alongside Spritely's `tsyesika` and `cwebber`
- [erights.org — CapTP Four Tables](http://erights.org/elib/distrib/captp/4tables.html)
- [NLnet — Spritely OCapN grant page](https://nlnet.nl/project/SpritelyOCapN/) — Aug 2022 – Oct 2023 funding window
