**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — identity, signatures, trust model, multi-device

How Willow roots identity in Ed25519, encodes identifiers for humans
in bech32m, ships invites as join links, and structures trust around
an owner-rooted permission DAG. Companion: [networking.md](networking.md),
[crypto.md](crypto.md), [ui.md](ui.md), [README.md](README.md).

## Ed25519 as identity root

Every Willow participant has a single Ed25519 keypair, wrapped by
`willow_identity::Identity` (`crates/identity/src/lib.rs:107-323`).
The wrapper re-exports iroh's native types
(`iroh_base::{EndpointId, PublicKey, SecretKey, Signature}`) so the
network address, signing key, and identity type are the same primitive
end-to-end — one of the simplifications driving the iroh migration
(`docs/specs/2026-03-29-iroh-migration-design.md:53-62`). Notable
properties of the production type today:

- `Identity` is `ZeroizeOnDrop` and `Send + Sync`
  (`identity/src/lib.rs:107-111`).
- `with_secret_bytes(closure)` is the preferred secret-bytes accessor:
  closure-scoped exposure with automatic zeroize on return
  (`identity/src/lib.rs:155-160`).
- `load_or_generate(path)` enforces `0o600` on Unix, atomic temp-file
  + rename on write, and refuses to load a key file with loose
  permissions (`identity/src/lib.rs:196-237`, issue #126 regression
  tests at lines 615-663).
- `verify()` is wired through `iroh-base`'s
  `ed25519_dalek::VerifyingKey::verify_strict` (RFC 8032 strict mode),
  closing Ed25519 signature-malleability vectors with a regression
  test pinning the call site (`identity/src/lib.rs:344-346`,
  test at lines 765-785).
- `pack(payload, identity) / unpack(bytes) -> (T, EndpointId)` is the
  signed-envelope wire form — Ed25519 sig + public key + payload
  bytes, all bincode-framed (`identity/src/lib.rs:434-467`).
- `pack_profile / unpack_profile` add a `peer_id` cross-check to
  defeat profile spoofing — a profile claiming to be Alice signed
  by Mallory's key returns `IdentityError::PeerMismatch`
  (`identity/src/lib.rs:482-507`, issue #145 tests at lines 851-873).

## Bech32m-with-HRP user-facing identifiers

Per `docs/specs/2026-04-24-bech32-identifiers.md`, every identifier
that can appear in a UI, URL, paste buffer, or log line is encoded
as bech32m with a type-tagged human-readable prefix. Wire format is
unchanged — bech32 is strictly a display-and-input boundary
(deliberately mirroring NIP-19's "MUST NOT be used in NIP-01 events"
rule). HRPs declared today: `wpeer`, `wserver`, `wevent`, `wchan`,
`winv`, `wrelay`, `wblob` (spec §HRP table). Length 4-7 ASCII; the
spec rejected 2-char HRPs because the body already dominates length
and the longer form scans cleanly in logs.

The spec makes one explicit security commitment: **`wsecret` will
never exist**. Private keys do not get a bech32 form. The `nsec`
↔ `npub` visual-similarity disaster in the Nostr ecosystem is
treated as a settled negative; secrets stay in the keystore (native)
or non-extractable WebCrypto keys (browser) and never enter paste
buffers (`bech32-identifiers spec §HRP table` and §"No `wsecret`
HRP, ever").

bech32m (BIP-350) over plain bech32 (BIP-173) — the substitution-of-`q`-near-trailing-`p` flaw in plain bech32 is fatal for variable-length TLV
identifiers (`winv`, `wevent`, `wchan`).

## Shareable join links

`docs/specs/2026-03-27-shareable-join-links-design.md` ships invites
as a single URL: `https://willow.intendednull.com/#join=<base64-token>`
where `JoinToken { inviter_peer_id, server_id, link_id, server_name,
inviter_name }` is metadata only — **the link contains no secrets**
(spec §Security Model). All sensitive data (channel keys) flows
over the existing signed gossipsub channel during a live exchange
between joiner and inviter. Three new `WireMessage` variants drive
the handshake on the `_willow_server_ops` topic: `JoinRequest`,
`JoinResponse`, `JoinDenied`. Inviter-must-be-online is a known
constraint accepted in v1.

The spec is honest about the tradeoff against Discord-style invites:
"Anyone with the link URL can send `JoinRequest` messages to burn
uses. This is the same tradeoff as Discord invite links" — owners
mitigate with conservative `max_uses`, deletion of compromised
links, or feature disable.

## Trust model (per CLAUDE.md, `willow-state`)

- **Owner = implicit all-permissions root.** No event grants the
  owner anything; every other principal's permissions ladder up
  from owner-issued `GrantPermission` events.
- **Fine-grained permissions** — `SyncProvider`, `ManageChannels`,
  `ManageRoles`, `SendMessages`, `CreateInvite`. Granted via
  `GrantPermission` from owner or admin.
- **Admin status is structurally separate** — managed exclusively
  through `ProposedAction` + vote, never via `GrantPermission`.
  Kicks are admin-only `ProposedAction`; there is no granular
  "can kick" permission.
- **Invite trust lists are suggestions, not authority.** Joining
  peers verify state from multiple sources and use the
  majority-trusted-source state hash (CLAUDE.md §Trust Model).
- **Relay = regular client.** Trusted only if explicitly granted
  `SyncProvider` permission by the owner. The relay binary itself
  does not receive any privilege from being the relay.

The authority model is enforced at one point: `apply_event` +
`required_permission()` in `crates/state/src/materialize.rs`,
checked before an event is created so rejected events never enter
the DAG. See `docs/specs/2026-04-12-state-authority-and-mutations.md`.

## Multi-device identity — unsolved, deferred

`docs/specs/2026-04-24-seal-gift-wrap-dms.md` (the DM deferral spec)
calls out multi-device identity as **non-negotiable** for the future
MLS-over-Willow spec: a long-term identity plus a short-lived
per-device signing key. This is not implemented today; current
Willow has one identity per device file. The deferral spec states:
"NIP-17 explicitly lacks FS / PCS — and the Nostr ecosystem itself
has moved on to NIP-EE / Marmot, both MLS-based" — and uses MLS as
the umbrella for the multi-device problem. No code lands until that
spec is written.

## Behavior identity (PR #636)

PR #636 names behavior identity as **structurally the same problem**
as multi-device user identity, and recommends a shared kernel
mechanism rather than reinventing twice (PR #636 lines 515-529):

> When a peer enables a behavior, the kernel generates and
> custodies a fresh Ed25519 keypair scoped to that peer and that
> instance. Events authored through `host.broadcast` are signed
> under that identity, not the user's. The runtime does *not*
> migrate behavior keypairs between peers; cross-peer behavior
> continuity is an app-level concern.

The "cross-peer bot identity" pattern is then app-level: an in-band
registration event maps a peer-side behavior keypair to an
app-level role, enforced by the app's own pre-check. Behavior
components never see private keys; key custody is identical to the
user-identity custody story (PR #636 line 524). The seal-gift-wrap
deferral spec's flag and PR #636's "structurally the same problem"
together commit Myrhiza to one kernel mechanism for both cases.

## Lift-into-Myrhiza notes

- **Direct lift:** Ed25519 + bech32m identifiers + the
  `pack/unpack` + `pack_profile/unpack_profile` envelope shape +
  the strict-verify pin + `ZeroizeOnDrop` discipline on private
  bytes. All of this is small, well-tested, and load-bearing.
- **Direct lift with kernel boundary:** identity custody.
  Components describe events; the kernel signs. PR #636 commits to
  this hard ("Private signing keys live only in the kernel. No
  component sees them" — PR #636 line 50). Apps reach signing
  through `host.identity` (capability-gated for behavior profile)
  and through the implicit signing the kernel performs around
  state-`propose` output (PR #636 lines 268-310).
- **Multi-device + behavior identity:** a kernel-level mechanism
  serving both. Willow has not built this; Myrhiza must, and
  should treat it as one capability surface, not two.
- **Trust model surface:** owner-rooted with explicit grants. The
  generalization that PR #636 envisions ("each app defines its own
  permission set, but also supplies the *pre-check* code that
  gates event creation," PR #636 lines 366-385) is the migration
  story — the table of permissions and the `required_permission()`
  function become app-internal, called by the kernel through the
  same exported authority predicate that drives `apply`. Same
  function, two callsites — divergence is "impossible by
  construction."

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- `crates/identity/src/lib.rs` — `Identity`, `pack/unpack`, `verify`, `UserProfile`, `pack_profile/unpack_profile` (with line refs above)
- `docs/specs/2026-04-24-bech32-identifiers.md` — full bech32m HRP table, NIP-19 alignment rationale, `wsecret`-never commitment
- `docs/specs/2026-03-27-shareable-join-links-design.md` — JoinToken/JoinLink + WireMessage variants + handshake
- `docs/specs/2026-04-24-seal-gift-wrap-dms.md` — multi-device identity flagged as non-negotiable for MLS-over-Willow
- `docs/specs/2026-04-12-state-authority-and-mutations.md` — pre-check before signing
- PR #636 §"Crypto and key custody" (lines 266-310), §"Behavior identity is per-(peer, behavior-instance)" (lines 515-529), §"What stays the same about Willow" (lines 367-385)
- `willow CLAUDE.md` § Trust Model, § Architecture Notes (Authority Model)
