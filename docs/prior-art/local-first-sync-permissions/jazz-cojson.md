**Date:** 2026-05-29
**Status:** active
**Subject:** Jazz / cojson — per-transaction peer-verified Ed25519 group/role authority over an untrusted relay (the Myrhiza-shaped model)

# Jazz / cojson

cojson ("collaborative JSON") is the protocol layer beneath Jazz, a local-first
framework from **Garden Computing, Inc.** (founder Anselm Eickhoff). It is the
one shipped local-first product whose authority model matches the shape Myrhiza
adopts: **authority is verified by each peer from signatures, not enforced by a
trusted server.** Eickhoff's framing in his Jazz talk: *"this access control is
not implemented by some central authority or a backend, but globally by
encryption signatures."*

This file describes the **Classic Jazz / cojson** model (npm `cojson` 0.20.x).
The in-progress **Jazz 2.0 alpha** rewrite changes this materially — see
[maturity-and-the-2.0-pivot.md](maturity-and-the-2.0-pivot.md), which is
load-bearing for any honest read of Jazz's trajectory.

## Data model: CoValues, sessions, transactions

- A **CoValue** is a collaborative value (CoMap, CoList, CoStream, etc.) with a
  content-addressed header. Its `id` derives from the header hash.
- State is a log of **transactions**. Each transaction is appended to a
  **session** (a `(signerID, sessionID)` stream); sessions give per-device
  monotonic ordering, analogous to Myrhiza's per-author chain.
- Every transaction is **Ed25519-signed** by the author's signer secret and
  **content-hashed with BLAKE3**. The wire types are explicit:
  `signer_z…` (Ed25519 public), `signerSecret_z…`, `signature_z…`,
  `sealer_z…` (X25519 public, for encryption), `sealerSecret_z…`. An **AgentID**
  is the pair `sealer_z…/signer_z…` — encryption identity and signing identity
  bundled. (Source: cojson `src/crypto/crypto.ts`, `src/ids.ts`.)

This is structurally the same primitive as a Myrhiza event: signed, hash-linked,
per-author-ordered, opaque-payload. See [paradigm-contrast.md](paradigm-contrast.md)
for the full mapping.

## The group/role model

Permissions live in a **Group** CoValue. A Group is itself a CoMap whose entries
map member IDs (accounts or agents) to roles, plus encryption key material. The
roles (from cojson `src/permissions.ts`, verbatim):

| Role | Capability |
|---|---|
| `reader` | Can read the group's CoValues |
| `writer` | Can read and write the group's CoValues |
| `admin` | Read, write, **and change member roles** |
| `manager` | Can change roles **except** admin; can read/write |
| `writeOnly` | Can only write, and read **their own** changes (not others') |

Invite roles (`adminInvite`, `writerInvite`, `readerInvite`, `writeOnlyInvite`,
`managerInvite`) and `revoked` round out the `Role` union. A CoValue is owned by
a Group; the owner's role table is the access-control list. Groups can be
**extended** (a group added as a member of another group) so permissions
cascade — Myrhiza has no direct analogue at v1 but it informs the deferred
`myrhiza-permission-rbac`.

## How authority is verified — `determineValidTransactions`

This is the load-bearing function ([cojson `src/permissions.ts`]). For each
transaction awaiting validation, cojson:

1. Reconstructs the **group state as of the transaction's `madeAt` time**
   (`groupContent.atTime(tx.currentMadeAt)`) — authority is evaluated against the
   role the author held **when the transaction was made**, not the current role.
2. Looks up the author's effective role at that time (`roleOfInternal`).
3. Marks the transaction **valid** only if that role permits the write
   (`admin` / `manager` / `writer` / `writeOnly`), otherwise
   `tx.markInvalid("Transactor has no write permissions")`.
4. Group-administration transactions (setting roles, setting `readKey`) carry
   their own checks: *"Only admins can set readKeys"*, role-assignment must not
   exceed the assigner's own role (`isHigherRole` ladder).

Crucially this runs **on every peer that ingests the transaction** — the relay
does not adjudicate. A transaction signed by a non-member, or by a member whose
role at that time lacked write permission, is rejected identically by every
honest peer. **Convergence + authority in one deterministic pass.** This is the
exact shape of Myrhiza's `state-apply` authority verdict
([../../specs/2026-05-09-myrhiza-master-design/convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4).

## Read access is cryptographic, not advisory

`reader` is not enforced by withholding bytes — the relay sees ciphertext.
CoValue content is symmetrically encrypted under a **read key**; the read key is
**sealed** (X25519 anonymous-box, `encryptKeySecret`) to each member's sealer
public key. Only members holding a role with read rights get the key revealed to
them. A peer that is not a reader literally cannot decrypt, even if it holds the
ciphertext. (Source: cojson `src/permissions.ts` `writeOnlyKeys`/`readKey`
handling, `src/coValues/group.ts`.)

## Revocation rotates keys

Removing a member sets their role to `revoked` and triggers
`rotateReadKey()` (cojson `src/coValues/group.ts`): a **new read key is generated
and re-sealed to the remaining members**. The revoked member keeps any content
they already decrypted but cannot read transactions written after revocation.
This is forward-secrecy-by-rotation, the same problem MLS solves with TreeKEM
(see [../mls/](../mls/)). cojson's source carries a comment noting a past bug
where *"the new read key was not revealed to everyone"* on rotation — a candid
signal that this path is subtle and has been wrong in production.

## Jazz Cloud and the relay

**Jazz Cloud** is Garden Computing's hosted sync+storage. In the cojson model it
is a **dumb relay/blob store**: it forwards signed/encrypted transactions and
persists ciphertext. It cannot read content, cannot forge transactions, and its
verdict on validity is irrelevant — peers re-derive validity. This is the
property Myrhiza wants from iroh relays
([../../specs/2026-05-09-myrhiza-master-design/networking.md](../../specs/2026-05-09-myrhiza-master-design/networking.md) §11.4: "relays are
dumb topic bridges").

## Honest maturity note

We could **not** verify any named at-scale production deployment of Jazz/cojson.
Jazz has **no disclosed external funding** (Tracxn lists it as unfunded as of
April 2025); it is an open-source project (MIT) plus a hosted cloud. Treat its
peer-verified model as **architecturally validating, not deployment-proven** —
the same posture the corpus takes toward Loro ([../crdts/ecosystem.md](../crdts/ecosystem.md)).
The 2.0 rewrite (next file) is reintroducing a trusted-server tier, which is
itself evidence the pure model has DX/scaling friction.

## Sources

- cojson source (npm tarball `cojson@0.20.18`): `src/permissions.ts`,
  `src/crypto/crypto.ts`, `src/ids.ts`, `src/coValues/group.ts`
- npm registry: <https://registry.npmjs.org/cojson> (latest 0.20.18, MIT)
- Jazz repo + LICENSE (MIT, "Copyright (c) 2026 Garden Computing, Inc."): <https://github.com/garden-co/jazz>
- Eickhoff Jazz talk: <https://gitnation.com/contents/jazz-build-real-time-local-first-react-apps-with-sync-and-secure-collaborative-data>
- Jazz Cloud: <https://jazz.tools/cloud>
- Tracxn (unfunded): <https://tracxn.com/d/companies/jazz/__zNBGBa_i64EhT_bn7WnAcrSg2wmDYY47ReTJInp-zuY>
