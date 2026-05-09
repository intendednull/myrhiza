**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — Structurally unresolved problems Myrhiza inherits if borrowing

# Open Problems

These are the structural problems the Pears stack has *not solved* — not bugs,
not roadmap items, but architecturally open questions. If Myrhiza borrows a
Pears pattern, it inherits the corresponding unresolved problem. Each section
names the problem, what Pears does today, and what Myrhiza will need to
decide additionally.

## 1. Multi-Author Conflict Resolution at the Application Level

**The Pears state.** Autobase mechanically merges multi-writer cores into a
linearized order. The merge is deterministic given the same set of input
blocks. *What the merged log means* is the application's job — Autobase
calls the app's `apply(nodes, view, host)` function, and the app decides
whether each node is a state mutation, a writer-add, a no-op, or invalid.

**What's unresolved.** The app's `apply` function is just JavaScript. It
must:

- Be deterministic for cross-peer convergence (no `Math.random()`, no
  `Date.now()`, no map-iteration-order dependence).
- Reject invalid mutations consistently — every replica has to reject the
  same set or they diverge.
- Handle the writer-set-as-state problem: who is allowed to write *to* the
  Autobase is itself state, so the validation rules have to be derivable
  from the prior state of the log.

There is no substrate enforcement. There is no type signature. There is no
"valid `apply` function" certificate. If your app's merge function has a
bug, every replica that runs the buggy version will diverge from every
replica that doesn't, and the only way to detect it is to compare derived
views across peers.

**What Myrhiza inherits if borrowing.** All of the above, plus the question
"how do we declare the `state-apply` function in a way that's enforceable
at the substrate?" Myrhiza's WASM CM substrate at least gives a place to
declare the host-import surface; the *content* of the apply function is
still the app's responsibility, but the substrate can refuse to call it
with non-deterministic helpers. That's a step Pears doesn't take.

## 2. Push Notifications Without a Server

**The Pears state.** Hypercore is a pull protocol — peers fetch new blocks
when they're online and looking. There is no protocol-level concept of "wake
up the recipient when a message arrives." On always-online devices
(servers, desktops with the app running) this is fine. On mobile devices
that suspend the app, it isn't.

iOS specifically:
- Suspends apps within seconds of backgrounding.
- Does not allow arbitrary apps to maintain background sockets.
- The only reliable wake mechanism is APNS (Apple Push Notification
  Service) — which requires a server-side party to send the push and is
  routed through Apple's infrastructure.

Android is more permissive (foreground services, JobScheduler) but still
has Doze mode and increasingly aggressive battery management on
manufacturer skins. Long-lived sockets are not a portable assumption.

**What's unresolved at the Pears protocol level.** Nothing. There is no
public Hypercore push-notification spec. Keet presumably solves this with
some mix of (a) a Holepunch-operated relay that triggers APNS pushes for
mobile recipients, or (b) an "always-on companion" model where another
peer the user owns relays. Neither is documented in the open repos.

**What Myrhiza inherits if borrowing the data-pull pattern.** The same
problem. Myrhiza will need an *explicit* answer:

- Either accept that mobile is offline-by-default and design UX around it
  (chat works when both peers are online; otherwise messages are queued).
- Or design a notification-relay capability — declared at the host import
  layer — that explicitly mediates "ask Apple/Google to wake the
  recipient." This is a server-mediated step; pretending otherwise is
  what Pears does and what Myrhiza shouldn't.

## 3. Cross-Platform Consistency: Browser Parity

**The Pears state.** A native Hypercore client (via Bare) gets full P2P:
direct UDP holepunching, full DHT participation, mDNS local-network
discovery. A browser-based Hypercore client *cannot* — browsers don't expose
raw UDP, can't run a DHT node, and have limited NAT-traversal options
(WebRTC is the realistic fallback).

The `holepunchto/hyperswarm-dht-relay` repo (ISC licensed, 45 stars) exists
to bridge browser clients into the DHT via a relay node. This works but
means browser clients are *not* full peers — they're clients of a relay,
which is a server, which contradicts the architecture's framing.

**What's unresolved.** There is no browser-side Hyperswarm full-peer story.
The web is structurally a client-server platform; making it not so requires
either WebTransport (still not universally deployed in browsers) or a
relay-node compromise.

**What Myrhiza inherits.** If Myrhiza wants browser-as-app-host, it has the
same constraint. Either accept relay-mediated browser participation, or
declare browser-as-supported-only-via-WASM-in-a-native-shell. The Pears
posture is implicitly the second one — Pear apps are not browser apps.
Myrhiza should be explicit about which.

## 4. Identity Portability: Key = Device

**The Pears state.** Hypercore identity is per-keypair. A keypair
typically lives on a single device's storage. If you reinstall, lose the
device, or want to use the same identity from a second device, the
operations look like:

- **Reinstall same device:** identity gone, you have to re-pair.
- **New device:** new keypair → other peers see you as a new peer →
  you have to be re-added to all rooms / followers / shared cores.
- **Multiple devices for one user:** Keet solves this with the
  `holepunchto/keet-identity-key` (hierarchical deterministic key pairs)
  + `blind-pairing-core` patterns — but this is per-application logic,
  not protocol-level identity.

The Hypercore protocol doesn't have a concept of "user identity" beyond
"keypair that signs blocks." User-level identity is an application
abstraction layered on top.

**What's unresolved.** A protocol-level account / user / persona primitive.
Every app currently invents its own. Cross-app identity portability is
not in the picture at all — your Keet identity is not your hypothetical
P2P-Twitter identity.

**What Myrhiza inherits.** If it borrows the per-device-keypair pattern,
it inherits the recovery problem. Possible directions: (a) protocol-level
HD-key-derivation patterns à la Keet (and BIP32); (b) a separate identity
primitive backed by hardware-secure-enclave keys with sync via the user's
own data; (c) explicitly delegate identity to an external system (e.g.,
DIDs, OCapN-style capabilities). Pears does (a) per-app; Myrhiza could do
better by declaring an identity capability at the host-import layer.

## 5. Sybil Resistance: Free-for-All DHT Topics

**The Pears state.** Hyperswarm topics are 32-byte values. Anyone can
announce on any topic. Anyone can join any topic. There is no
protocol-level rate limiting, no proof-of-work, no Sybil resistance.

Apps build their own:

- **Keet:** room keys are large-secret keys. Knowing the key is the
  capability to join. Sybil is mitigated because joining requires the
  invite-issued key. This is access-control-by-capability, not
  Sybil-resistance-at-the-DHT.
- **Open-topic apps (file sharing, etc.):** no protection. Anyone can
  flood the topic with junk peers, and consuming peers have to filter.

**What's unresolved.** Protocol-level Sybil resistance for open topics.
Pears doesn't try to solve this; Holochain's per-DNA validation rules at
least let an application reject malformed peers, but Hyperswarm's DHT
makes no claim.

**What Myrhiza inherits.** The same problem on any DHT-based discovery.
Solutions are application-level: either (a) restrict topic access by
shared secret (capability) — this is what Keet does — or (b) require some
proof of work / proof of stake / proof of identity. Pears doesn't pretend
to solve (b); Myrhiza shouldn't either, but it should be explicit that
the substrate doesn't, so apps know they have to.

## 6. Storage Growth: Append-Only Means Linear Growth

**The Pears state.** Hypercore is append-only. New events get appended.
**Truncate** exists as of v10 (the breaking change in 2022) and lets you
drop a prefix or suffix of the log, but the per-block chain of hashes
means you can't surgically remove blocks from the middle without breaking
verification.

Sparse replication helps with *bandwidth* — peers download only the blocks
they need — but doesn't help with *long-term storage* on the writers.
Writers keep the full log unless they truncate.

**What's unresolved.** "How do we run for years without unbounded growth?"
The current pattern in the wild:

- Truncate + new key for each "chapter" of an app's lifetime.
- Compaction via app-defined snapshots, written into a separate
  Hypercore, with the original log eventually truncated below the
  snapshot.
- Just accept linear growth and rely on storage-cheap-enough-for-now.

Keet's actual approach is unknown publicly.

**What Myrhiza inherits.** Same problem. The append-only log is
attractive for determinism (immutable history → reproducible state), but
unbounded growth is a real cost. Specs should plan for compaction /
snapshot semantics from day one — Pears retrofitting truncate as a v10
breaking change is a cautionary tale.

## 7. Encryption-Key Rotation

**The Pears state.** Hypercore supports per-block encryption via the
`encryption: { key: ... }` option. Once a core is created with an
encryption key, **rotating that key effectively starts a new core** — there
is no in-place re-encryption story. To rotate, you create a new
encryption-keyed core, copy the meaningful state into it, and announce
the new core's key to all readers.

**What's unresolved.** In-place encryption-key rotation, forward secrecy
(future compromise of a key shouldn't expose old messages), and
post-compromise security (after a key compromise, you can re-establish
secrecy without recreating the whole core). None of these are addressed.

**What Myrhiza inherits.** If Myrhiza uses Hypercore-shaped encrypted
logs, it inherits the rotation tax. The Signal-protocol-style ratchet
that gives both forward and post-compromise secrecy is *not* what
Hypercore provides. Apps that need real ratchets layer them above the
log (e.g., MLS) and treat the log as transport. Specify this explicitly
or you'll have a security review nightmare.

## 8. Determinism: JS at Runtime, Not at Substrate

**The Pears state.** Autobase merges are deterministic *given the same
input blocks*. The execution that produces those blocks is JavaScript
in Bare or Node — which is *not* deterministic by default. Apps that
care about cross-peer convergence have to:

- Avoid `Math.random()` (use a derived seed from log state).
- Avoid `Date.now()` (use logical clocks or block-included timestamps
  treated as data, not as authority).
- Avoid object-property iteration order assumptions (mostly okay in modern
  V8 but not universal).
- Avoid floating-point arithmetic where order matters.
- Avoid async-ordering dependencies (multiple `await` arms can resolve
  in unpredictable order).

The substrate does not enforce any of this. The only feedback mechanism
is "your app diverged across peers and someone noticed."

**What's unresolved.** Substrate-enforced determinism. Pears chose JS for
ergonomics; the cost is that determinism is on the honor system.
Compare:

- **xsnap (Agoric):** deterministic JS engine — substrate-level
  enforcement.
- **WASM CM (Myrhiza target):** host-import surface declaration —
  non-deterministic helpers can be refused at the import layer.
- **Bare (Pears):** none — all V8 globals are present, app must avoid
  them.

**What Myrhiza inherits — or rather, gets to fix.** This is the place
Myrhiza's substrate choice (WASM CM) is meaningfully better than Pears'.
The `state-apply` profile in CLAUDE.md treats determinism as a
load-bearing property; the substrate must refuse to call `state-apply`
with non-deterministic helpers. Pears does not have this. Myrhiza
should.

## Summary

| Open problem | Pears's posture | Myrhiza's required answer |
|---|---|---|
| Multi-author conflict resolution | App-defined `apply`, no substrate help | Declare `state-apply` semantics, type the helper set |
| Mobile push notifications | Privately solved in Keet, not public | Explicit notification-relay capability |
| Browser parity | Relay-mediated, not full peer | Explicit "browser-via-WASM-shell" or accept relay |
| Identity portability | Per-app HD keys (Keet) | Declared identity capability at host-import |
| Sybil resistance | App-defined (room keys) | Application-layer; substrate can't solve |
| Storage growth | Truncate + rotate-to-new-core | Compaction primitive in spec from day one |
| Encryption-key rotation | New core | Layer ratchet above log (MLS or equivalent) |
| Determinism | Honor system | Substrate-enforced via host-import declarations |

Myrhiza's WASM CM substrate is better-positioned to address the
determinism, identity, and notification-capability problems *at the
substrate layer*. Pears solves them per-application or doesn't solve them.
The other problems (storage growth, Sybil, encryption rotation) are
architectural constraints of the append-only-log + DHT-discovery model that
both stacks share — Myrhiza inherits them by borrowing the pattern.

## Cross-references

- [critiques.md](./critiques.md) — observable evidence for the unresolved problems
- [lessons.md](./lessons.md) — concrete validates / avoid / borrow tables
- [governance.md](./governance.md) — why these problems persist (single-vendor)
