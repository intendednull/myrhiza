**Date:** 2026-05-22
**Status:** active
**Subject:** AT Protocol identity — `did:plc` rotation keys vs `#atproto` signing key, the load-bearing prior art for Myrhiza multi-device identity

# Identity in AT Protocol

This is the **load-bearing file for Myrhiza**. AT Protocol is the only at-scale production system that has cleanly separated *long-term user identity* from *active signing key*, in a way that lets users rotate signing keys, recover from device compromise, and migrate hosting providers without changing identity. The mechanism is `did:plc` — a custom DID method Bluesky built because the existing options didn't fit.

The model has three layers worth lifting independently:

1. **DID as durable identifier.** A `did:plc:ewvi7nxzyoun6zhxrhs64oiz` (or `did:web:alice.example.com`) is the user's permanent identifier. It doesn't change when they migrate PDSes, rotate keys, or change handles.
2. **Rotation keys as identity control.** A small set (1-5) of high-authority keys that can rewrite the DID document — including replacing the signing key, changing the PDS, or replacing the rotation keys themselves.
3. **Signing keys as operational authority.** A single low-authority `#atproto` verification method that signs repository commits but cannot reconfigure identity.

The asymmetry is the point: **rotation keys are kept rarely-used and offline-capable; signing keys live on the PDS or device and rotate on a schedule.** This is the exact shape Myrhiza's Plan B-2 needs for multi-device identity (see [`docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md`](../../specs/2026-05-19-plan-b-2-persistent-identity-design.md)) and for the unified user-identity / behaviour-identity problem named in [`prior-art/willow/open-problems.md`](../willow/open-problems.md) §"Multi-device identity".

## DID methods supported

AT Protocol blesses **two DID methods** and explicitly keeps the set small:

| Method | Operator | Update mechanism | Recovery | Used by |
|---|---|---|---|---|
| **`did:plc`** | `plc.directory` (Bluesky PBC) | Signed PLC operations submitted to central directory | 72-hour rewrite window via higher-priority rotation key | Default for new Bluesky accounts; ~12M+ DIDs as of October 2024 |
| **`did:web`** | Self-hosted via HTTPS + DNS | Edit your own `.well-known/did.json` | DNS / hosting control | Power users, organizations, self-sovereign-identity purists |

Other methods (`did:key`, `did:ion`, `did:ethr`, etc.) are not supported. The atproto spec explicitly says: *"other methods will be supported in the future, but the intention is to keep the 'blessed' set as small as possible."* That decision matters: every consumer (PDS, Relay, AppView, client) has to be able to resolve every supported method, so adding a method is expensive.

**Practical reality:** the vast majority of users are on `did:plc`. `did:web` exists but requires DNS-and-HTTPS-savvy operations and forfeits the recovery story (you have whatever recovery your DNS registrar offers, which is "phone support and pray").

## Rotation keys vs signing keys

This is the architectural insight worth lifting.

### Rotation keys

From the `did:plc` spec:

> *"Control over a `did:plc` identity rests in a set of reconfigurable rotation key pairs."*

Properties:

- **Count: 1 to 5**, no duplicates, **stored in priority order**. Index 0 = highest authority.
- **Curves: secp256k1 (`k256`) or NIST P-256 (`p256`) only.** No Ed25519, no other curves. (See [crypto.md](crypto.md) for why.)
- **Format: `did:key`** encoding (multibase + multicodec key prefix).
- **Not in the rendered DID document.** Rotation keys live in the PLC *operation log*, not in the resolved DID doc. A consumer fetching the DID doc to verify a repo signature does **not** see the rotation keys — only the signing key. (This is important: it means rotation keys can stay private to the PLC directory + operator.)
- **Operations are signed by a rotation key.** Each PLC operation references the prior operation by CID and is signed by one of the current rotation keys. The PLC directory validates the signature.
- **Higher-priority rotation keys can rewrite history.** This is the recovery mechanism — see §72-hour recovery below.

### Signing keys (verification methods)

From the atproto DID spec:

- **One `#atproto` verification method per DID document**, of type `Multikey` with a `publicKeyMultibase` field.
- **Any `did:key`-compatible curve.** In practice Bluesky uses the same secp256k1/p256 set, but the spec is permissive.
- **Purpose: signs repository commits.** When the PDS writes to a user's repo, it signs the commit object with the `#atproto` signing key.
- **No identity control.** From the `did:plc` spec: *"The signing key does not have any control over the DID identity unless also included in the `rotationKeys` list."*

The separation means: **compromise of the signing key gives an attacker forgery-of-future-commits power but not identity-takeover power.** The rotation keys retain full control to invalidate the compromised signing key, install a new one, and (within 72 hours) rewrite any malicious history.

## The 72-hour recovery window

This is the most distinctive piece of the design and the part Myrhiza should study most carefully.

The mechanism, per `did:plc` spec:

> *"The PLC server provides a 72hr window during which a higher authority rotation key can 'rewrite' history, clobbering any operations...signed by a lower-authority rotation key."*

How it works in practice:

1. Alice has rotation keys `[K0, K1, K2]` in priority order. `K0` is offline / in cold storage; `K1` is on her laptop; `K2` is on her phone PDS.
2. Attacker compromises `K2` and submits a malicious operation `Op_bad` that replaces all rotation keys and the signing key with attacker-controlled keys.
3. PLC accepts `Op_bad` because it's signed by a valid rotation key.
4. **Within 72 hours**, Alice (or her recovery service) uses `K0` or `K1` to submit `Op_recover`, which references the operation *before* `Op_bad` as `prev`. PLC sees this is signed by a higher-priority key (lower index in the original `rotationKeys` array) and **clobbers `Op_bad`** from the operation log.
5. If 72 hours pass without recovery, `Op_bad` becomes final and Alice has lost her identity.

The model assumes a recovery service or watchful user / agent monitors PLC operations and can detect malicious changes within 72 hours.

**This is not consensus-free.** It depends on `plc.directory` honoring the recovery rule — the directory operator can in principle decline to apply a recovery operation, or apply a non-recovery operation against the user's wishes. Bluesky operates plc.directory in good faith and publishes the audit log, but the trust model is "transparent server with audit log," not "trustless."

### What Myrhiza inherits / rejects from this

**Worth borrowing**:
- **The rotation-key-priority + clobber-window mechanism itself.** It maps well to a P2P setting if the role of `plc.directory` is replaced by either a Willow-style replicated state (every peer sees the operation log) or an MLS-style group state.
- **Rotation keys not in the DID document.** Reduces the attack surface for casual signature verification.
- **Multi-rotation-key with priority ordering.** Lets a user have hot-warm-cold key tiers without committing to a specific HSM / paper-backup workflow.

**Worth rejecting / replacing**:
- **The central registry.** Myrhiza has no `plc.directory` analog and shouldn't build one. Replace with replicated state-apply.
- **The 72-hour fixed window.** In a P2P setting, "72 hours from PLC clock" isn't well-defined. A logical-clock variant ("recovery valid if it lands within N events after the malicious op") works better but creates a different attack surface (attacker stalls event delivery).
- **secp256k1/p256 curve restriction.** Myrhiza prefers Ed25519 throughout per Willow precedent; the secp256k1 choice is largely a Bitcoin-ecosystem legacy and the p256 fallback exists for hardware-token compatibility.

## did:web fallback

`did:web:alice.example.com` resolves by fetching `https://alice.example.com/.well-known/did.json` and using whatever DID document is served there.

Properties:

- **Self-sovereign in the limited sense that the user controls their domain.** If the user controls DNS and HTTPS, they control identity.
- **No 72-hour recovery.** If the user loses control of the domain (DNS hijack, expired registration, hosting provider banning them), they lose identity.
- **No operation log.** A `did:web` resolver fetches the *current* document each time. There's no historical signature chain.
- **Used by power users and some organizations.** `bsky.app` itself (the official Bluesky AppView) has a `did:web` identity.

Most users will never see `did:web`. It exists primarily so that organizations and people with strong opinions about self-sovereignty have an exit ramp from `plc.directory`.

## Handles vs DIDs

Worth noting separately:

- **Handle**: `alice.bsky.social` or `alice.example.com`. Human-readable, resolved via DNS TXT record (`_atproto.alice.example.com TXT "did=..."`) or HTTPS well-known (`https://alice.example.com/.well-known/atproto-did`).
- **DID**: `did:plc:ewvi7nxzyoun6zhxrhs64oiz`. Permanent.

A user can change handles freely (subject to DNS control); the handle is just a forward pointer to the DID. The DID is the durable identifier the protocol uses internally.

**Bidirectional verification**: the DID document includes `alsoKnownAs: ["at://alice.example.com"]` listing claimed handles; the DNS / HTTPS resource at the handle includes the DID. Both directions must match for the handle to verify. This prevents handle squatting in either direction.

## Account migration

Because identity is the DID (not the PDS hostname), users can migrate PDSes. The migration flow:

1. Create account on new PDS, declare intent to migrate.
2. New PDS imports the repo from old PDS (CAR file export).
3. User submits a PLC operation (signed by a rotation key) that updates the DID document to point to the new PDS service endpoint.
4. Once propagation completes, the old PDS can release the data (or keep a mirror, depending on policy).

The September 2025 Bluesky blog post "Enabling Account Migration Back to Bluesky's PDS" notes that this works in both directions — users who left for a third-party PDS can return without losing identity. Migration in either direction requires the rotation key.

**This is the strongest credible-exit story in production federated social.** Mastodon has account migration but loses your follower graph (followers must re-follow you); atproto's migration preserves the graph because everyone's following relationships are stored against your DID, not your PDS.

## What this prior art tells Myrhiza Plan B-2

Plan B-2 (`docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md`) currently splits Willow's identity into `PeerKeypair` (device-scoped) and `AuthorKeypair` (long-term, per-user). That's structurally similar to atproto's `#atproto` signing key vs `rotation key` split but with two important differences:

1. **Plan B-2's `AuthorKeypair` is one key, not a set.** AT Protocol uses 1-5 rotation keys in priority order, enabling hot-warm-cold tiering. Myrhiza should consider whether the Author tier wants this — the runner-up "one author key" is simpler but loses the recoverability story.
2. **Plan B-2 has no recovery window.** A compromised `AuthorKeypair` is currently terminal. AT Protocol's 72-hour clobber-window is the deployed answer to "what do we do when a signing key leaks." Myrhiza needs an equivalent — either a clobber window backed by replicated state, or an explicit "rotation event" type that revokes the prior key.

The unified-multi-device-and-behaviour-identity problem named in `prior-art/willow/open-problems.md` §"Multi-device identity" maps cleanly to atproto's model: the user is the DID; each device gets its own signing key declared as a verification method (atproto today only allows one `#atproto` method but the structural primitive supports multiple); a behaviour-instance is structurally the same as "a device that happens to be a bot." Myrhiza can lift this directly.

What Myrhiza **cannot** lift directly: the central registry. AT Protocol's "the directory honors the recovery rule" trust model collapses immediately in a peer-symmetric setting. The equivalent has to be either (a) replicated state-apply where every peer sees the operation log and enforces the recovery rule, or (b) MLS-style group state where the user's "identity group" is a multi-device MLS group and key rotation is an MLS commit. Both have non-trivial design surfaces; see [open-problems.md](open-problems.md) §"Identity recovery in P2P".

## Sources

- atproto DID spec: <https://atproto.com/specs/did>
- did:plc spec (canonical): <https://github.com/did-method-plc/did-method-plc>
- did:plc v0.1 spec: <https://web.plc.directory/spec/v0.1/did-plc>
- did:web resolution in atproto: <https://atproto.com/specs/did#did-web>
- Handle resolution spec: <https://atproto.com/specs/handle>
- Account migration blog post (2025-09): <https://bsky.social/about/blog/enabling-account-migration>
- plc.directory operator info: <https://web.plc.directory/>
- DID PLC stats (~12M registered, Oct 2024): <https://web.plc.directory/>
- Plan B-2 design (Myrhiza): [`docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md`](../../specs/2026-05-19-plan-b-2-persistent-identity-design.md)
- Willow multi-device identity open problem: [`prior-art/willow/open-problems.md`](../willow/open-problems.md)
