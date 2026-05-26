**Date:** 2026-05-22
**Status:** active
**Subject:** AT Protocol applications — bsky.app, the Atmosphere, Germ DM, alternative AppViews

# Applications shipping on AT Protocol

The "Atmosphere" is Bluesky's term for the ecosystem of apps that share the atproto substrate. It's structurally interesting because **identity, data, and follower-graph are shared between apps** — write a post via `bsky.app`, write a blog entry via Whitewind, and both records live in your same repository under your same DID. Other apps see both. This is the closest deployed approximation of the "unified data substrate" that p2p-app-runtime designs (including Myrhiza) often aim for.

The Atmosphere is also dominated by `bsky.app` to a degree that makes the framing partly aspirational. Most users only ever use `bsky.app`. The diversity is more interesting at the AppView/feed level than at the standalone-app level.

## bsky.app

The flagship: a Twitter-shaped microblogging app operated by Bluesky PBC. ~27.5M MAU as of February 2026 (third-party estimate).

Records it writes to your repo:

- `app.bsky.feed.post` — posts (text + optional images/embeds)
- `app.bsky.feed.like` — likes
- `app.bsky.feed.repost` — reposts
- `app.bsky.graph.follow` — follows
- `app.bsky.graph.block` — blocks
- `app.bsky.actor.profile` — profile data
- `app.bsky.feed.generator` — custom feed definitions
- `app.bsky.graph.list` — user lists
- `chat.bsky.message` — DMs (server-side-readable encryption; **not** E2E)

The schemas under `app.bsky.*` are the dominant Lexicon namespace by volume. Anyone else can read your `app.bsky.*` records (they're public). Anyone with the right Lexicon-typed records can write similar records via their own client.

**Direct messages** on `bsky.app` are server-side-readable. Bluesky can see them. End-to-end encryption requires Germ DM (see below).

## Germ DM (third-party, February 2026)

The first private messenger to launch natively in `bsky.app`. Architecturally:

- **Identity**: your atproto DID + handle.
- **Encryption**: MLS (Messaging Layer Security, RFC 9420) — see `prior-art/mls/`.
- **Storage**: Germ's own infrastructure, not atproto repos. (MLS messages are not atproto records — they're MLS protocol messages.)
- **Discovery**: a `germ.app` profile badge that appears on your `bsky.app` profile; tapping it starts an E2E chat.

The integration model is **"identity from atproto, encryption from MLS, data in Germ's silo."** This is a deliberate separation — Germ doesn't try to put MLS messages in atproto repos (those are public; MLS demands confidentiality), and atproto doesn't try to define an MLS schema (the protocols compose at the identity boundary).

Available on iOS in North America and Europe as of February 2026; Android version not yet announced.

**For Myrhiza**: this is a clean integration pattern. Identity-from-one-system, encryption-from-another, with a clear boundary. Myrhiza-equivalent would be "identity from Myrhiza kernel, encryption from `host.mls` capability, with the MLS state living in a host-managed capability rather than as state-apply events." See [`prior-art/willow/runtime-vision.md`](../willow/runtime-vision.md) for PR #636's framing.

## Alternative AppViews and apps

A non-exhaustive selection of community AppViews / apps that demonstrate the Atmosphere's actual diversity:

### Whitewind — long-form blog

- **NSID**: `com.whtwnd.blog.entry`
- **Purpose**: longer-form Markdown blog posts, separate from the 3000-char `app.bsky.feed.post` limit.
- **Architecture**: your blog entries live as records in your atproto repo; Whitewind's AppView indexes them and presents a blog-shaped UI.
- **Cross-app effect**: people who follow you via `bsky.app` see your blog entries appear in their timeline if their client supports the `com.whtwnd.blog.entry` Lexicon (most don't, but the records are still there in your repo).

### Statusphere — status / availability

- **NSID**: `xyz.statusphere.status`
- **Purpose**: per-user availability indicators (online, away, busy, etc.).
- **Architecture**: a single typed-record-per-status, overwritten on changes. Statusphere's AppView surfaces the current status for users you follow.

### Smoke Signal — events and RSVPs

- **NSID**: `events.smokesignal.calendar.event`, `events.smokesignal.calendar.rsvp`
- **Purpose**: event creation and RSVP tracking.
- **Architecture**: events as records in the organizer's repo; RSVPs as records in attendees' repos. The AppView aggregates RSVPs into a per-event view.

### Frontpage — link aggregation

- **NSID**: `fyi.unravel.frontpage.post`, `fyi.unravel.frontpage.comment`, `fyi.unravel.frontpage.vote`
- **Purpose**: Hacker-News-like link discussion.
- **Architecture**: posts, comments, and votes as separate record types; ranking algorithm runs in the AppView.

### Custom feeds (within bsky.app)

- **NSID**: `app.bsky.feed.generator`
- **Purpose**: third-party algorithmic feeds within `bsky.app`'s UI.
- **Architecture**: a "feed generator" is an HTTP service that takes a request and returns a list of `at://` post URIs. The user's `bsky.app` client subscribes to the feed; the AppView fetches posts at those URIs from the relevant PDSes/Relay.
- **Notable**: this is the most-used third-party-extension surface. There are thousands of custom feeds in production, contributed by individual developers.

## The "share-the-substrate" pattern

The interesting architectural property: **all of these apps share my one repository under my one DID.** Whitewind doesn't have its own identity for me; it uses my atproto DID. My follower graph from `bsky.app` is, in principle, accessible to any AppView that wants to surface it.

In practice the sharing is one-directional in a specific way:

- **Reading is universal.** Any AppView can read records of any NSID from any user's repo (subject to access controls — most are public).
- **Writing is per-app.** A user explicitly chooses which app writes which records. `bsky.app` writes `app.bsky.*`; Whitewind writes `com.whtwnd.blog.*`. There's no automatic cross-app write authority.
- **Identity is shared.** All apps see you as the same DID. No per-app identities.

This is roughly the architectural shape Myrhiza's master spec envisions for the kernel — one identity, many apps writing to a shared content-addressed substrate. The difference is that atproto's substrate is the PDS-hosted repository (server-authoritative), while Myrhiza's would be the peer-replicated state DAG (peer-authoritative).

## App SDK story

For developers building atproto apps:

- **TypeScript**: `@atproto/api` is the reference client. Code-generated from Lexicon schemas.
- **Go**: `bluesky-social/indigo` includes a client and is the basis for Relay implementations.
- **Rust**: `atrium` (community-maintained by Yuki Sugyan) is the dominant Rust SDK.
- **Python**: `atproto-py` is well-maintained.

Most apps use the TypeScript SDK because most clients are web-based. The desktop ecosystem is essentially absent (a few Tauri-wrapped clients exist but no major desktop app). The mobile ecosystem is dominated by `bsky.app`'s React Native client; alternative mobile clients exist but are niche.

## What this prior art tells Myrhiza

On the apps-on-shared-substrate model:

- **It works.** The "Atmosphere" framing is real — Whitewind users genuinely have their data living in the same repo as their `bsky.app` posts. The pattern is deployed.
- **It works because one organization controls the substrate.** Bluesky owns `plc.directory`, owns the Relays, operates `bsky.app`. The Atmosphere coheres because Bluesky enforces it. In a peer-symmetric setting (Myrhiza), what's the equivalent enforcement?
- **Diversity is at the AppView/feed level, not the app level.** Most users never leave `bsky.app`. The Atmosphere's diversity is real but lives in the long tail of small audiences. Myrhiza-equivalent: expect this same pattern; the headline-app monoculture is structural, not a design failure.
- **Third-party E2E integration is feasible.** Germ DM proves you can layer MLS on top of an atproto-identity foundation. Myrhiza's `host.mls` capability is structurally similar — the protocol gives you identity, MLS gives you confidentiality, the two compose at the identity boundary.

What Myrhiza should *not* take from this:

- **The PDS-as-authoritative-writer pattern.** Atproto's shared-substrate model depends on one server (the user's PDS) being the sole writer for their records. Multi-device writes are serialized through the PDS. Myrhiza's peer-symmetric model rejects this; the equivalent must be CRDT-style or consensus-style coordination.
- **The AppView monoculture as inevitable.** Bluesky's structural reasons for `bsky.app` dominance (it's first, it's well-funded, it's the on-ramp) don't necessarily apply to Myrhiza. The Myrhiza-equivalent shape might actually be more diverse — or might be the same; we don't have evidence yet.

## Sources

- bsky.app: <https://bsky.app/>
- Whitewind: <https://whtwnd.com/>
- Statusphere: example listing in atproto community apps repo
- Smoke Signal: <https://smokesignal.events/>
- Frontpage: <https://frontpage.fyi/>
- Custom feeds documentation: <https://docs.bsky.app/docs/starter-templates/custom-feeds>
- Atmosphere overview: <https://atproto.com/articles/the-atmosphere>
- atrium Rust SDK: <https://github.com/sugyan/atrium>
- atproto-py: <https://github.com/MarshalX/atproto>
- Germ DM launch: <https://techcrunch.com/2026/02/18/a-startup-called-germ-becomes-the-first-private-messenger-that-launches-directly-from-blueskys-app/>
