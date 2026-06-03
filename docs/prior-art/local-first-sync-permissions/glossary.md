**Date:** 2026-05-29
**Status:** active
**Subject:** Glossary — terms used across the local-first-sync-permissions folder

# Glossary

System-specific terms. CRDT-theory terms (RGA, YATA, state vector, tombstone)
live in [../crdts/glossary.md](../crdts/glossary.md).

## Cross-cutting

- **Peer-verified authority** — authorization re-computed independently by every
  replica from signed data; no server is trusted to adjudicate. (Paradigm A,
  [paradigm-contrast.md](paradigm-contrast.md).)
- **Trusted-middlebox authority** — authorization evaluated by a trusted server or
  proxy whose verdict replicas accept. (Paradigm B.)
- **Partial replication** — a client syncs only a subset of the data. In Paradigm
  B this *is* the read-authorization mechanism (withhold what you can't see). In
  Myrhiza it is deferred and kept separate from authorization.

## Jazz / cojson

- **CoValue** — a collaborative value (CoMap, CoList, CoStream, etc.); the unit of
  state. Content-addressed by header hash.
- **cojson** — "collaborative JSON," the protocol implementation under Jazz.
- **Session** — a per-device append stream of transactions, `(signerID,
  sessionID)`; provides monotonic per-device ordering.
- **Transaction** — one Ed25519-signed, BLAKE3-hashed change appended to a session.
- **Group** — a CoValue holding the member→role map and key material; the
  access-control list that owns other CoValues.
- **Role** — `reader` / `writer` / `manager` / `admin` / `writeOnly` (+ invite
  variants + `revoked`). See [jazz-cojson.md](jazz-cojson.md).
- **`writeOnly`** — may write, and read only its own changes; used for
  drop-box / invite-request flows.
- **AgentID** — `sealer_z…/signer_z…`: an X25519 encryption identity paired with
  an Ed25519 signing identity.
- **Sealer / Signer** — X25519 (encryption, "seal") and Ed25519 (signature)
  keypairs. `sealed_…` = anonymous-box ciphertext.
- **Read key** — symmetric key encrypting a CoValue's content; sealed to each
  authorized member's sealer key.
- **`determineValidTransactions`** — cojson's per-peer function deciding which
  transactions are valid given role-at-the-time; the analogue of Myrhiza's
  `state-apply` verdict.
- **Jazz Cloud** — Garden Computing's hosted relay+storage; in the cojson model a
  dumb (content-blind) bridge.
- **Jazz 2.0** — the alpha relational rewrite that reintroduces a trusted-server
  tier ([maturity-and-the-2.0-pivot.md](maturity-and-the-2.0-pivot.md)).

## Zero (Rocicorp)

- **zero-cache** — the server process replicating upstream Postgres and serving
  filtered query results to clients.
- **ZQL** — Zero Query Language; permission read-rules are ZQL filters.
- **Custom mutator** — an arbitrary server-side TypeScript write function; the
  current (post-deprecation) way to enforce write authority.
- **Server-authoritative** — the server mutator's result takes precedence over the
  client's optimistic mutation.

## Triplit (Aspen Cloud)

- **Permissions API / rules API** — schema-embedded access rules (newer
  permissions API supersedes the older rules API), keyed by roles and JWT claims,
  enforced server-side.
- **postUpdate** — a permission check against the *resulting* row after an update.

## PowerSync (JourneyApps)

- **Sync Rules / Sync Streams** — YAML/SQL defining which rows sync to which
  client. Sync Streams is the newer (beta) form.
- **Bucket** — a named, entitlement-scoped group of rows.
- **Parameter query** — selects bucket membership from **authentication
  parameters** (trusted, from JWT) or **client parameters** (untrusted).

## ElectricSQL

- **Shape** — a partial replica of a Postgres table defined by a `WHERE` clause.
- **Proxy auth / Gatekeeper auth** — external authorization patterns; Electric
  itself has no permission model. Gatekeeper = token claim equals the exact
  requested shape definition.

## Sources

- Term definitions are drawn from the per-product files in this folder; primary
  sources cited there.
