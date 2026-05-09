**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — Honest critique of marketing vs reality

# Critiques

The Pears stack is the most production-deployed P2P-app substrate in this
prior-art set, and that earns it a careful reading rather than a hagiographic
one. This file is the section to consult when someone says "let's just do
what Pears does" — these are the places the marketing and the reality
diverge, where the documentation is thinnest, and where Myrhiza spec
authors will inherit unresolved problems by borrowing a Pears pattern.

## "No Servers" vs Bootstrap-Node Reality

Keet's App Store description says (verbatim, as of 2026-04-29):

> No servers. No data stored in the cloud.

The technical reality is more nuanced. The Hyperswarm DHT bootstrap process
needs *some* known address to find peers. The default
`hyperdht/lib/constants.js` ships with three hardcoded bootstrap nodes:

```js
exports.BOOTSTRAP_NODES = global.Pear?.config.dht?.bootstrap || [
  '88.99.3.86@node1.hyperdht.org:49737',
  '142.93.90.113@node2.hyperdht.org:49737',
  '138.68.147.8@node3.hyperdht.org:49737'
]
```

Those are Holepunch-operated infrastructure. Without them — or without
operator-supplied alternatives — peer discovery fails on a fresh install on
any network where mDNS doesn't bridge to the other peer.
[hyperswarm#194](https://github.com/holepunchto/hyperswarm/issues/194) ("Running
the README Example with No Internet Connection Does Not Work — Discovery
Fails Without DHT Bootstrap", open since 2025-07-09) documents exactly this:

> Without Internet Access: The client hangs on `Client is searching for
> peers...` and never connects to the server. This happens even when both
> devices are on the same LAN and can ping each other.
>
> Used `const swarm = new Hyperswarm({ bootstrap: [] })` to force LAN-only
> discovery → Also fails, even with internet access.

The Russian volunteer in [hyperswarm#212](https://github.com/holepunchto/hyperswarm/issues/212)
("new bootstrap node in Russia", 2026-03-29) offering to help further
underscores that bootstrap-operator availability is an operational concern,
not a settled-architecture detail.

**The fair framing.** "No servers in the message-routing path; bootstrap
servers in the discovery path." Once a peer connection is established, data
flows directly. But the *finding* part is not server-free in practice. The
"no servers" rhetoric is true for the steady-state and false for the
bootstrap state — and the marketing doesn't make the distinction.

For Myrhiza spec authors: any P2P design should *explicitly* document the
bootstrap surface. Bootstrap nodes are policy artifacts, not implementation
details — pretending they don't exist is how you end up with a centralized
dependency you didn't plan for.

## Closed-Source Flagship App

Keet ships, but its source is not visible. The infrastructure libraries
(Hypercore, Hyperswarm, Hyperdrive, Autobase, Bare, Pear) are open-source;
the Keet client itself is not. The `keet-appling` repo is the desktop
*shell* (Apache-2.0, 4 stars, 31 lines of substantive code); the
`keet-mobile-releases` repo is binary artifacts only.

Implications:

- **The actual production-mobile reference implementation is opaque to
  outside readers.** When you want to know "how does Keet actually handle iOS
  background mode?" you can't read the code. The answer is whatever
  Holepunch ships in the `.ipa`.
- **Edge-case behavior is unverifiable.** How does Keet handle:
  - 6-month-offline reconnection?
  - Hard NAT pairs on cellular networks?
  - Battery-drain mitigation on Android?
  - iOS push-notification fallback?

  None of these have public answers at the source level.
- **The "P2P apps work at scale" claim is gated on Holepunch's word.**
  Outside teams can't independently verify that the patterns scale because
  the only proof point is closed.

Compare to Delta Chat (open source, ships on iOS / Android / desktop, uses
Iroh as transport) — which is *less* polished as a product but *more*
verifiable as evidence of "P2P consumer mobile is feasible."

**For Myrhiza:** when citing Keet as evidence that "P2P consumer apps
work", be honest that the citation is "Holepunch claims they work, and the
App Store rates the result 4.59★ across 99 reviews." That is real but
narrow.

## Pear Runtime Is Young

The Hypercore data layer is 10+ years old. The Pear *application runtime
layer* is from 2024-02-03. Two years of production hardening is not nothing,
but it's not the same as the Hypercore-stack maturity. The CHANGELOG shows
continuing CLI churn through May 2026 — `pear install` was added in May 2026
itself, `pear build` and `pear multisig` in late-2024 / 2025.

If you're borrowing the Pear-runtime *patterns* (not the Hypercore
*protocols*), you're borrowing two-year-old infrastructure. Plan for it to
keep moving.

## iOS Background Mode + Push Notifications

This is the structural problem any P2P client on iOS faces, and Keet's
solution is non-public.

iOS does not let arbitrary apps maintain long-lived sockets in the
background. The OS suspends the app within seconds of leaving the
foreground and only allows specific entitled categories (VoIP, audio, GPS,
etc.) to keep network connections alive. For a P2P chat app, this means
that delivering a message to an offline-but-installed iOS device requires
*something* to wake the app — and the only "something" the platform offers
is **APNS** (Apple Push Notification Service), which is operated by Apple
and routed through Apple's servers.

How does Keet do this? Unknown at the source level. The `holepunchto/*`
repos contain no public push-notification implementation. Plausible
hypotheses:

- A small relay service operated by Holepunch that holds messages for
  offline iOS recipients and triggers an APNS push. Once the app wakes,
  it pulls over Hypercore directly.
- A "always-on companion" pattern where another peer (e.g., a desktop the
  user owns) routes notifications to APNS on behalf of mobile peers.
- Reduced functionality — iOS Keet just doesn't get push and is
  foreground-only.

The first option (Holepunch-operated relay + APNS) is the most plausible
given the deployment shape. If true, it means **Keet relies on
Holepunch-operated infrastructure for iOS push**, which contradicts the
"no servers" framing more sharply than the bootstrap-node case.

There is no public protocol-level answer to this on the Holepunch side. It
is the single largest architectural problem Pears does not solve in the
open. Myrhiza will face the same problem and will need a different answer.

## Performance / Battery on Mobile

[hyperswarm#47](https://github.com/holepunchto/hyperswarm/issues/47) ("Add
ability to toggle different layers of hyperswarm on/off", open since
2020-04-28):

> it would be great for many usecases to control if hyperswarm is on or off
> for e.g. battery reasons. this can be extended to toggling on/off
> different layers: all of hyperswarm, the DHT, wifi multicast

Open six years. The fact that this is still an outstanding feature request
suggests that Hyperswarm's default behavior (always-on connections, periodic
DHT participation) is not battery-tuned for mobile clients out of the box —
each app has to make its own choices about when to bring up / tear down the
swarm.

Tail-latency for hard NATs is also a known concern. The `randomPunchInterval`
default in `hyperdht` is 20 seconds, with at most one random punch in flight
at a time (`_randomPunchLimit = 1`). For symmetric-NAT pairs that need
multiple random punch attempts, this puts a floor on connection
establishment time that's not great for human-perceived latency.

No public benchmarks of Keet's battery / latency behavior on real mobile
networks. Holepunch presumably has internal numbers; outside observers do
not.

## Tether-Only Funding Risk

Holepunch is funded primarily by Tether the stablecoin issuer. This is a
single-investor concentration risk that doesn't exist for foundation-backed
projects:

- **One decision can cut funding.** If Tether deprioritizes the bet, the
  whole stack's roadmap goes with it. There's no syndicate of investors to
  absorb a partial pullback.
- **Strategic alignment to Tether's priorities.** Recent activity in the
  `tetherto/*` GitHub org (Pearpass — a password manager built on the same
  stack) suggests Tether is increasingly directing the stack toward
  financial-application use cases. Non-financial directions are at Tether's
  discretion.
- **Reputational coupling.** Tether the stablecoin operator carries its own
  regulatory and reputational exposure. Anything Holepunch ships inherits
  some of that.

This is not a critique of the technical work; it's a critique of the
operational continuity model. Foundation governance, in the long run, is
more resilient than single-corporate-parent governance — and the Dat-era
precedent already showed what happens when a single funder steps back.

## Hypercore Protocol Version Churn

Hypercore has had at least three major-version transitions in the past decade:

| Version | Date | Compatibility |
|---|---|---|
| v8.x | through 2022 | Older format |
| v10.0.0 | 2022-08-15 | **Not compatible with v9 and earlier** — re-encode required |
| v11.0.0 | 2025-01-13 | Migration from v10 required |
| v11.30.1 | 2026-05-06 | Current LTS-ish line |

The README's framing on v10 is honest: *"Version 10 is not compatible with
earlier versions (9 and earlier), but is considered LTS, meaning the storage
format and wire protocol is forward compatible with future versions."*
Translation: each major bump means existing data has to be migrated, but
*future* breakage is intended to be smoother.

The v10 → v11 transition (less than three years later, January 2025)
suggests "LTS" doesn't mean "indefinitely-supported in place"; it means
"we'll move you to the next thing." Apps on Hypercore should plan for a v12
migration before 2028 as a base rate.

For Myrhiza's `state-apply` semantics, this matters: if the underlying
event-log format changes, every replica needs to migrate in lockstep, or
they fall off each other's wire-format compatibility.

## Sparse Documentation Outside the Main Path

Multiple long-open issues across the repos make the same complaint:

- [hyperswarm#60](https://github.com/holepunchto/hyperswarm/issues/60) —
  "Protocol documentation?" — open since 2020-04-28, 10 comments, last
  activity 2026-04-26. Original ask: how do I implement Hyperswarm in a
  non-JS language?
- [hyperdht#37](https://github.com/holepunchto/hyperdht/issues/37) —
  "Documentation" — open since 2020-04-30:
  > the documentation (README) fails to succinctly explain what this
  > distributed hash table is and what it is not. For instance, it does not
  > explain how authentication and encryption is achieved.
- [autobase#186](https://github.com/holepunchto/autobase/issues/186) — "Open
  documentation issues" — open since 2024-12-11
- [corestore#88](https://github.com/holepunchto/corestore/issues/88) —
  "Plans for TypeScript/JSDocs and/or Improved API Docs?" — 2024-02-24
- [pear#202](https://github.com/holepunchto/pear/issues/202) — "TypeScript
  support?" — 2024-08-20

Pattern: documentation lives at `docs.pears.com` for the happy-path
developer-getting-started flow, but **protocol-level specs sufficient for
re-implementation in another language are not maintained**. If you want to
build a Rust Hypercore client, you read the JS source. There is no
RFC-style document.

This is consistent with the single-vendor governance model — Holepunch
doesn't need protocol specs because they own the only implementation. It
becomes a problem the moment anyone wants to build interop.

## JS-Only Stack

Bare is impressive engineering, but the stack remains JavaScript-only at the
application layer. Implications:

- **No Rust / Go / Swift / Kotlin native components.** A Keet-style app is
  pure JS top to bottom. Calling Rust requires writing a `bare-*-native`
  Node-API addon — workable, but every native module has to be packaged for
  every platform Bare supports.
- **Type system only via TypeScript** — and TS support in Pear is itself an
  open issue ([pear#202](https://github.com/holepunchto/pear/issues/202)).
  No substrate-level types.
- **No determinism enforcement.** JS is non-deterministic by default
  (`Math.random()`, `Date.now()`, async ordering). Apps that need
  cross-peer convergence have to be careful by hand. Compare to xsnap
  (Agoric's deterministic JS engine) or any WASM Component Model runtime
  with declared host-import surfaces.
- **Server-grade language affordances are absent.** No goroutines, no
  Tokio-shaped async, no compile-time-safe enums. JS' affordances are JS'
  affordances.

For Myrhiza — which has chosen WASM as the substrate explicitly to get
language neutrality and substrate-level capability typing — Pears' JS-only
posture is a *what-not-to-do*, not a model.

## Summary Table

| Critique | Severity for borrowing | Evidence link |
|---|---|---|
| "No servers" vs bootstrap reality | High — affects honest framing | [hyperswarm#194](https://github.com/holepunchto/hyperswarm/issues/194) |
| Closed-source flagship | High — limits verifiability | iOS App Store, no source repo |
| iOS push notifications unsolved publicly | High — Myrhiza will hit same wall | No public answer |
| Pear runtime young | Medium — APIs still moving | [pear/CHANGELOG.md](https://github.com/holepunchto/pear/blob/main/CHANGELOG.md) |
| Battery-on-mobile defaults | Medium — every app retunes | [hyperswarm#47](https://github.com/holepunchto/hyperswarm/issues/47) |
| Tether-only funding | Medium — operational continuity | Org metadata + tether-flatpak link |
| Hypercore version churn | Medium — migration tax | v10 (2022) → v11 (2025) |
| Sparse protocol docs | Medium — interop blocker | [hyperswarm#60](https://github.com/holepunchto/hyperswarm/issues/60) |
| JS-only substrate | High *for Myrhiza specifically* | Bare README, Pear architecture |
| Single-vendor governance | High — see governance.md | `pear/CONTRIBUTING.md` |

## Cross-references

- [governance.md](./governance.md) — single-vendor risk in detail
- [open-problems.md](./open-problems.md) — problems Myrhiza inherits if borrowing
- [lessons.md](./lessons.md) — what to validate / avoid / borrow
- [history.md](./history.md) — protocol-version churn timeline
