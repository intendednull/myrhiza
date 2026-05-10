**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Maintenance and participation

# Maintenance and participation

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


