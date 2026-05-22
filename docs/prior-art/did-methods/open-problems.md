**Date:** 2026-05-22
**Status:** active
**Subject:** What the DID ecosystem does not solve — gaps Myrhiza inherits when building on DID-shaped identity primitives.

# Open problems — DID methods

Each entry: problem + why it matters for Myrhiza + canonical sources.

## 1. Multi-device key custody under a long-term identity

DID methods give you "one DID, multiple verification methods." They don't tell you how the *new* device gets the second key paired to the existing DID without compromising security. Bluesky's answer (PDS-mediated session creation) leaks key material to the PDS. Signal's answer (linked-devices via QR code) is in-app. MLS's answer (Welcome message) requires an existing group.

**What's needed:** a Myrhiza-native pairing flow. Probably: existing-device-signs-new-device-key over an out-of-band channel (QR / link), with the existing device acting as the rotation-key authority.

**Canonical sources:** [`rotation.md`](rotation.md), [`prior-art/mls/`](../mls/), [`prior-art/signal/identity.md`](../signal/identity.md), [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md).

## 2. Resolver decentralization

`did:plc` works at scale because one operator runs the registry. `did:web` decentralizes resolution at the cost of HTTPS / DNS dependencies. `did:webvh` adds verifiable history but inherits both `did:web`'s problems and the operator-resolver problem (who hosts the `did.json`?).

**What's needed:** decide whether Myrhiza identifiers need network-resolvable form at all. If yes, pick: bundled-with-app discovery, gossip-over-iroh discovery, or HTTPS-based. None are fully decentralized.

**Canonical sources:** [`methods.md`](methods.md), [`adoption.md`](adoption.md), [`prior-art/iroh/`](../iroh/) (discovery primitives).

## 3. Revocation propagation

Rotation keys can rotate an active signing key. But how does an app peer find out that key K, which used to be valid, is now revoked? `did:plc` answers this via resolver consistency (peers re-resolve the DID document). MLS answers via group epoch updates. The general distributed-revocation problem is unsolved at peer-to-peer scale.

**What's needed:** Myrhiza's revocation propagation model. Likely tied to the cap-token / cap-grant layer (revocation is an event in app state); the DID layer remains advisory.

**Canonical sources:** [`rotation.md`](rotation.md), [`prior-art/capability-tokens/`](../capability-tokens/), [`prior-art/mls/`](../mls/).

## 4. Cross-method interop

A Myrhiza app peer might encounter `did:plc`, `did:webvh`, `did:key`, and Myrhiza-native bech32m peer keys all in the same session (the user has a Bluesky account, a Spruce-managed enterprise identity, an ephemeral cap-token, and a Myrhiza peer). Which "identity" is the user? How do they map?

**What's needed:** an identity-aggregation layer (or an explicit decision to not aggregate — each binding is its own identity for its own context).

**Canonical sources:** [`methods.md`](methods.md), [`implementations.md`](implementations.md), [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md).

## 5. Post-quantum DID method readiness

PQXDH ([`prior-art/signal/post-quantum.md`](../signal/post-quantum.md)) and the broader PQ migration affect DIDs eventually — the signing keys bound to DIDs are mostly Ed25519 / secp256k1, both broken under a sufficiently large quantum computer. No DID method has a PQ-key story yet.

**What's needed:** track the W3C DID Core 1.1 work for PQ key types. For now, document that Myrhiza's identity primitives are pre-PQ.

**Canonical sources:** [`crypto.md`](crypto.md), [`prior-art/signal/post-quantum.md`](../signal/post-quantum.md).

## 6. Privacy of the DID document itself

A DID document can reveal which keys are active, which services the user uses, what verification methods are bound — useful metadata for an adversary. `did:peer` (peer-to-peer DIDs, not gossiped to the world) is the privacy-preserving variant, but the spec is small and under-deployed.

**What's needed:** either use private-by-default DID methods (`did:peer`) or accept that Myrhiza identifiers are not private (the DID itself + DID document leak). Either way, document it.

**Canonical sources:** [`methods.md`](methods.md), [`prior-art/anonymity-transports/`](../anonymity-transports/), [`prior-art/signal/identity.md`](../signal/identity.md) (sealed sender for comparison).

## 7. Recovery from total key loss

Rotation keys recover from rotation-key compromise. They don't recover from rotation-key *loss*. If the user loses all rotation keys for a DID, the DID is dead (subject to the `did:plc` 72-hour clobber window or equivalent).

**What's needed:** social recovery (key shards held by trusted parties), service-mediated recovery (Bluesky-style email-recovery), or accept that lost keys = lost identity.

**Canonical sources:** [`rotation.md`](rotation.md), [`prior-art/willow/open-problems.md`](../willow/open-problems.md) §"Multi-device identity".

## 8. Performance at scale

`did:plc` resolution scales because Bluesky operates `plc.directory` with cache + CDN. A Myrhiza network with millions of peers each resolving DIDs through HTTPS or gossip doesn't have the same operator. Resolution latency dominates UX.

**What's needed:** caching strategy + offline-tolerance. Myrhiza apps must work when the user's DID is not resolvable right now (cached pubkey, signature still valid).

**Canonical sources:** [`adoption.md`](adoption.md), [`implementations.md`](implementations.md).

## 9. Governance of DID-method registries

The W3C DID Method Registry is community-maintained (anyone submits a method). DID Methods don't have to pass review for acceptance. A DID method that says "method X is broken" cannot remove method X from the registry without contention.

**What's needed:** Myrhiza picks a fixed set of methods it natively supports. Don't depend on the W3C registry as authority for which methods to trust.

**Canonical sources:** [`history.md`](history.md), [`methods.md`](methods.md).

## 10. Bluesky's `did:plc` operator risk

Bluesky PBC is the operator of `plc.directory`. If Bluesky as a company fails, `did:plc` resolution fails. Bluesky has stated commitments to non-misuse but operator-failure is a non-zero risk for any Myrhiza app depending on `did:plc` resolution.

**What's needed:** if Myrhiza uses `did:plc` at all, document the operator-dependency explicitly. Consider mirroring or proxying `plc.directory` data.

**Canonical sources:** [`adoption.md`](adoption.md), [`prior-art/at-protocol/governance.md`](../at-protocol/governance.md).

## Cross-references

- [`README.md`](README.md), [`lessons.md`](lessons.md)
- Per-method evidence files
- [`prior-art/at-protocol/`](../at-protocol/), [`prior-art/mls/`](../mls/), [`prior-art/signal/`](../signal/), [`prior-art/capability-tokens/`](../capability-tokens/), [`prior-art/willow/open-problems.md`](../willow/open-problems.md)

## Sources

All sources in evidence files.
