**Date:** 2026-05-22
**Status:** active
**Subject:** AT Protocol glossary — system-specific terms

# Glossary

System-specific terms used throughout this corpus. Generic protocol terms (DID, ECDSA, JSON, CBOR, WebSocket, etc.) are omitted.

## AppView
The user-facing application tier in atproto's PDS / Relay / AppView trio. Consumes the Relay's firehose, indexes records relevant to its product, and serves clients via XRPC. `bsky.app` is the dominant AppView (Twitter-shaped microblogging); Whitewind, Smoke Signal, Frontpage, and Statusphere are smaller alternative AppViews. See [architecture.md](architecture.md) §AppView.

## Atmosphere
Bluesky's marketing/community term for the ecosystem of apps that share atproto identity and data. Atmosphere apps all read from and write to user repositories under the same DID. See [apps.md](apps.md).

## ATP / atproto
Two names for the same thing: the **AT Protocol** (Authenticated Transfer Protocol, though the acronym is rarely expanded). "ATP" appears in IETF working group names (`ietf-wg-atp`); "atproto" is the common shorthand and the GitHub org `bluesky-social`'s primary repo name.

## bsky.app
Bluesky's flagship Twitter-shaped microblogging app, operated by Bluesky PBC. The dominant AppView with ~27.5M MAU as of February 2026. Uses the `app.bsky.*` Lexicon namespace.

## BGS (Big Graph Service)
**Legacy name** for what is now called the Relay. Renamed November 10, 2023 ([discussion #1847](https://github.com/bluesky-social/atproto/discussions/1847)). The Go implementation still has a `bigsky` command name in `bluesky-social/indigo/cmd/bigsky` for backwards compatibility. If you see "BGS" in atproto documentation, it means Relay.

## Bluesky PBC
**Bluesky Social, Public Benefit Corporation** — the Delaware PBC that steward atproto and operates the dominant infrastructure (`plc.directory`, primary Relays, `bsky.app`). Incorporated October 2021, became PBC February 2022. See [governance.md](governance.md).

## CAR file
**Content Addressable aRchive** — IPFS's standard format for a self-contained CBOR DAG. Atproto exports repositories as CAR v1 files; account migration uses CAR for the data transfer. See [architecture.md](architecture.md) §CAR.

## CID
**Content Identifier** — IPFS's multihash + multicodec content-address format. Used throughout atproto for MST nodes, commits, and inter-record references.

## DAG-CBOR
**Deterministic CBOR encoding** — IPFS's canonical CBOR. Used for signing commits and PLC operations because two implementations encoding the same logical CBOR produce byte-identical bytes. See [crypto.md](crypto.md) §DAG-CBOR.

## DID
**Decentralized Identifier** — W3C spec for a permanent, resolvable identifier independent of any host. Atproto blesses two methods: `did:plc` and `did:web`. See [identity.md](identity.md).

## did:plc
The DID method developed by Bluesky for atproto identities. *"Self-authenticating DID which is strongly-consistent, recoverable, and allows for key rotation."* Operations submitted to `plc.directory`; rotation-key priority enables 72-hour recovery from key compromise. ~12M+ DIDs registered as of October 2024. See [identity.md](identity.md).

## did:web
The W3C-community-draft DID method based on HTTPS + DNS. Self-hosted via `/.well-known/did.json`. Supported by atproto as the independent-of-plc.directory alternative. Most users are on `did:plc`. See [identity.md](identity.md) §"did:web fallback".

## Firehose
The colloquial name for the `com.atproto.sync.subscribeRepos` WebSocket endpoint. A long-lived stream of CBOR-encoded commit events. Each Relay produces its own merged firehose; AppViews subscribe. See [networking.md](networking.md) §Firehose.

## Germ DM
Third-party E2E messenger that launched in `bsky.app` in February 2026. Uses atproto identity + MLS encryption + Germ's own storage. The first private messenger to launch natively in the Bluesky app. See [apps.md](apps.md).

## Handle
Human-readable identifier resolving to a DID. Either `alice.bsky.social` (sub-handle under Bluesky's domain) or `alice.example.com` (user-controlled domain). Resolves via DNS TXT record (`_atproto.<handle>`) or HTTPS well-known endpoint. Bidirectional verification required (DID document lists handle in `alsoKnownAs`; handle's DNS lists DID). See [identity.md](identity.md) §"Handles vs DIDs".

## Indigo
Bluesky's Go implementation of atproto infrastructure, particularly the Relay. Repo: `bluesky-social/indigo`. The Relay binary is `cmd/bigsky` (legacy name). MIT/Apache-2.0 licensed.

## Lexicon
Atproto's schema language. JSON-based, similar to JSON Schema with extensions for RPC, content-addressed types (`cid-link`, `blob`), and NSID namespacing. Strict additive evolution: `lexicon: 1` is fixed; individual schemas evolve without breaking changes. See [lexicon.md](lexicon.md).

## MST
**Merkle Search Tree** — atproto's content-addressed storage primitive for repositories. Deterministic from content (SHA-256-derived depth + key prefix compression). Two PDSes given the same records produce byte-identical MSTs. See [architecture.md](architecture.md) §MST.

## NSID
**Namespaced ID** — atproto's reverse-DNS identifier format for Lexicon schemas. Example: `app.bsky.feed.post` (under `bsky.app` domain), `com.whtwnd.blog.entry` (under `whtwnd.com` domain). Authority is rooted in DNS control. See [lexicon.md](lexicon.md) §NSID.

## PBC
**Public Benefit Corporation** — Delaware corporate form that legally permits the board to weigh public benefit alongside shareholder return. Bluesky Social is a PBC. Does not provide structural protection against acquisition. See [governance.md](governance.md).

## PDS
**Personal Data Server** — the per-user authoritative host in atproto's PDS / Relay / AppView trio. Hosts the user's repository, holds the `#atproto` signing key, signs commits. Self-hostable on a 1 vCPU / 1 GB / 20 GB VPS. ~99% of users are on Bluesky-operated PDSes. See [architecture.md](architecture.md) §PDS.

## PLC
The custom DID method name (`did:plc`). Originally stood for "Public Ledger of Credentials" but officially is just "PLC" with no standard expansion. The directory is `plc.directory`.

## plc.directory
The central registry for `did:plc` operations, operated by Bluesky PBC. Validates and orders operations; serves DID document resolution. ~12M+ DIDs as of October 2024. The trust model is "transparent server with audit log." See [identity.md](identity.md) §"DID methods supported".

## Relay
The fanout/aggregation tier in atproto's PDS / Relay / AppView trio. Subscribes to PDS firehoses, validates commits, produces a merged downstream firehose. Resource-heavy (terabytes of storage, gigabit throughput). Bluesky operates the primary Relays at `relay1.us-{west,east}.bsky.network`. Formerly called BGS (Big Graph Service); renamed November 2023. See [architecture.md](architecture.md) §Relay.

## Repository (Repo)
A user's data — collection of typed records and binary blobs, stored as a Merkle Search Tree on the user's PDS, signed by the PDS's instance of the user's `#atproto` signing key. Identified by the user's DID. Exportable as a CAR file. See [architecture.md](architecture.md) §Repository.

## Rotation Key
A high-authority key that controls reconfiguration of a `did:plc` identity. 1-5 rotation keys per identity, in priority order. Stored in `did:key` format, restricted to secp256k1 or NIST P-256 curves. **Not** included in the resolved DID document. See [identity.md](identity.md) §"Rotation keys vs signing keys".

## Signing Key (atproto signing key, `#atproto` verification method)
The low-authority key that signs repository commits. One per DID, declared in the DID document under `verificationMethod` with id ending `#atproto`. Cannot reconfigure identity unless also listed in `rotationKeys`. See [identity.md](identity.md) §"Signing keys".

## Subscribe Repos
The Lexicon name for the firehose endpoint: `com.atproto.sync.subscribeRepos`. WebSocket-based, CBOR-framed, cursor-resumable. See [networking.md](networking.md) §Firehose.

## Sync v1.1
The current firehose protocol version, rolled out May 2025. Cutover from v1.0 to v1.1 on the main `bsky.network` endpoint happened January 27, 2026. Added cursor stability across Relay restarts and separated account events from the repo firehose. See [networking.md](networking.md) §"Sync v1.1".

## TID
**Timestamp Identifier** — atproto's base32-encoded 64-bit timestamp+clock used as record keys within a collection. Sortable, monotonic per-PDS, collision-resistant.

## XRPC
Atproto's RPC layer. Conventions over HTTPS (GET for query, POST for procedure) and WebSocket (for subscription). Every endpoint identified by an NSID. Request/response shapes validated against Lexicon schemas. See [networking.md](networking.md) §XRPC.

## Sources

- AT Protocol glossary (official): <https://atproto.com/guides/glossary>
- atproto-community-wiki: <https://atproto.wiki/>
- AT Protocol Wikipedia: <https://en.wikipedia.org/wiki/AT_Protocol>
- Various atproto specs at <https://atproto.com/specs>
