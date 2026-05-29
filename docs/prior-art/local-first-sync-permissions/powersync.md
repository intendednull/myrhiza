**Date:** 2026-05-29
**Status:** active
**Subject:** PowerSync (JourneyApps) — Sync Rules + JWT-claim bucketing; read-side partial replication, write-side server-validated

# PowerSync (JourneyApps)

PowerSync is a backend-DB ↔ SQLite sync engine from **JourneyApps / Journey
Mobile, Inc.** (founded 2009; the engine ran inside JourneyApps' industrial
platform for Fortune-500 customers — GE, Halliburton, ExxonMobil per their own
press — before being spun out as a product). It syncs Postgres / MongoDB / MySQL
/ SQL Server down to client SQLite. The service is **source-available** under
**FSL-1.1-ALv2** (Functional Source License, converts to Apache-2.0 after the
license-change date). It is the most production-proven of the trusted-middlebox
group, and it scopes authority by **JWT claims**.

## Sync Rules and buckets

PowerSync's authority/partition primitive is **Sync Rules** (recently also "Sync
Streams," in beta): YAML/SQL that defines **buckets** — named groups of rows a
client is entitled to sync. A bucket has one or more **parameter queries** that
select which rows belong to which client.

Parameter queries draw on two parameter sources:

- **Authentication parameters** — from the **JWT**. `request.jwt() ->> 'sub'`,
  `request.user_id()`, custom claims like `request.jwt() ->> 'app_metadata.org'`.
- **Client parameters** — passed directly by the client.

PowerSync's docs state the security rule plainly: *"Token parameters are embedded
in the JWT authentication token and therefore can be considered trusted and can
be used for access control purposes. In contrast, client parameters … should not
be used for access control purposes."* The whole authority model rests on
**trusting a JWT minted by a trusted issuer**.

## Read vs write authority

- **Reads**: Sync Rules are pure **partial-replication filters** — they decide
  what the client is allowed to *download*. This is the cleanest expression of
  read-side authorization-by-subset in the group.
- **Writes**: PowerSync sends client writes to a developer-defined **backend
  endpoint** (the "upload" path); that trusted backend validates and applies them
  to the source DB. So write authority is, again, **server-side**.

## Contrast with cojson / Myrhiza

PowerSync is the inverse of the Myrhiza model on two axes at once:

1. **Trusted issuer**: identity and entitlement come from a JWT signed by a
   trusted auth provider (Supabase, Auth0, your backend). Myrhiza identity is a
   self-sovereign Ed25519 key ([identity.md](../../specs/2026-05-09-myrhiza-master-design/identity.md));
   there is no issuer to trust.
2. **Trusted enforcer**: the PowerSync service filters reads and the backend
   validates writes. Myrhiza's verdict runs on every peer
   ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4).

See [paradigm-contrast.md](paradigm-contrast.md).

## Worth borrowing

- **Bucket = entitlement-scoped row set keyed by signed claim** is a precise
  mental model for the v2 partial-replication direction Myrhiza defers
  ([open-problems.md](open-problems.md); [convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.5
  commits v1 to "every peer holds everything"). When Myrhiza needs subsetting,
  the bucket-parameter-query shape is a studied design — but the "claim" must be
  on-DAG signed authority, not a JWT.
- **Auth-params-trusted / client-params-untrusted** is a clean, hard-won
  distinction worth preserving in any Myrhiza grant API.

## State (verified)

FSL-1.1-ALv2 (→ Apache-2.0), Journey Mobile, Inc.; deployed at industrial scale
via the parent platform; MongoDB partnership announced Oct 2024. The most
maturity of any product in this folder — but maturity in the *trusted-server*
paradigm, which is the one Myrhiza rejects.

## Sources

- Parameter queries: <https://docs.powersync.com/sync/rules/parameter-queries>
- Sync Rules from first principles: <https://www.powersync.com/blog/sync-rules-from-first-principles-partial-replication-to-sqlite>
- Client parameters (security note): <https://docs.powersync.com/usage/sync-rules/advanced-topics/client-parameters-beta>
- Service repo + LICENSE (FSL-1.1-ALv2, "Copyright 2023-2026 Journey Mobile, Inc."): <https://github.com/powersync-ja/powersync-service>
- MongoDB partnership: <https://www.businesswire.com/news/home/20241017413684/en/>
