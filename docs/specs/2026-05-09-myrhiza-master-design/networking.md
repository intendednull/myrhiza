**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Networking

# Networking, sync, and relays

## 11. Networking, sync, and relays

### 11.1 Transport

iroh — gossip, content-addressed blob fetch, dial-by-pubkey QUIC,
DERP-style relay-bridged NAT traversal. The locked load-bearing
transport dependency.

**Version pin**: v1 commits to a specific iroh version pinned at
implementation start (likely `iroh 1.0.0` once stable, or the latest
RC at v1 ship). iroh's pre-1.0 API churn is real
(`prior-art/iroh/lessons.md`); v1 absorbs the pin and budgets for
upgrade pain.

**Network trait abstraction is preserved as a design seam.** Even
though iroh is committed for v1, the kernel-internal `Network` trait
(see §15.4 `crates/network/`) is shaped so a future kernel could
swap transports if iroh strategy shifts (Number 0 has redirected
before; iroh-ffi was mothballed). Trait shape: gossip publish/subscribe,
blob publish/fetch by content hash, dial-by-pubkey, NAT-traversal
hint. v1 ships only the iroh implementor; the seam exists for
optionality.

The kernel exposes a narrow networking surface to apps via
capability-gated host imports (broadcast, subscribe, blob fetch).
Apps do not see iroh directly. Transport-implementation changes are
not ABI changes for apps.

### 11.2 Topic membership

Apps subscribe to topics. A topic is a content-addressed identifier;
exact formula at §4.6. Membership in a topic = the peer is gossiping
events on that topic.

**Membership tracking** (v1): membership is implicit via subscription.
The kernel does not maintain a global membership roster — peers who
subscribe receive gossip; peers who unsubscribe stop receiving. Apps
that need explicit membership tracking (presence, online indicators)
implement it via state-apply events (e.g. `Join` / `Leave` events
materialized into `members` derived state).

**No anonymous-stranger gossip**: a peer cannot publish events to a
topic without first being granted topic-write permission via the
app's authority model (typically a permission module like
`myrhiza-permission-rbac`). Bandwidth cost of accepting gossip from
non-members is mitigated by the participation primitive (§12.5).

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
sync-provider-shape grant from a `myrhiza-permission-*` module).
Without permission, the peer functions as a regular peer, not a relay.

**Metadata correlation risk** (accepted): relays see traffic
patterns — which topic IDs subscribers join, message frequency,
participant count. The spec does NOT claim relays are trustless;
it claims relays do not see *payload contents* (encrypted with
group keys) and do not see *event semantics* (treated as opaque
gossip). For threat models requiring metadata privacy, relays must
be trusted operators or apps must implement traffic shaping (cover
traffic, padding) at the application layer. v1 does not budget
cover-traffic infrastructure.

**Censorship**: a malicious relay can selectively drop messages,
delaying convergence. Mitigation: peers route through multiple
relays when available; persistent message drops surface as
HeadsSummary divergence. Future-direction: relay-rotation policies.

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


