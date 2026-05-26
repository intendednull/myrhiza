**Date:** 2026-05-22
**Status:** active
**Subject:** AT Protocol federation — the honest assessment of who runs what, the ~99% concentration figure, and the hardware barriers

# Federation — the honest assessment

This is the "atproto is federation-shaped, not P2P" file. The user-facing claim is that atproto is decentralized, federated, and that users have "credible exit." All three are technically true and substantially misleading. This file disentangles what's true from what's marketing.

## The headline number

**As of early 2026, approximately 99% of users are hosted on Bluesky PBC's infrastructure.** This figure comes from multiple third-party analyses cross-referenced with Bluesky's own protocol check-in posts.

The breakdown:

- **PDS tier**: ~99% of users on Bluesky-operated PDSes (`*.bsky.network`). The remaining ~1% are spread across hundreds of independent operators, most hosting <100 users each.
- **Relay tier**: **Bluesky operates the only Relay most consumers ever talk to** (`relay1.us-west.bsky.network`, `relay1.us-east.bsky.network`). A small number of independent Relays exist (Northsky, a few researcher-operated ones) but they are not at production parity.
- **AppView tier**: `bsky.app` is operated by Bluesky and is the AppView used by ~98%+ of users. Other AppViews (Statusphere, Whitewind, Smoke Signal, Frontpage, etc.) exist but each serves <1% of users.
- **DID resolution**: `plc.directory` is operated by Bluesky and is the canonical resolver for the ~99% of users on `did:plc`.

If Bluesky PBC vanished tomorrow, the protocol would not gracefully degrade. The DID registry would freeze. The Relay tier would lose its primary node. The `bsky.app` AppView would go offline. Self-hosted users would survive (PDSes continue to function) but the network would have collapsed.

## Why is concentration this high?

Three structural reasons:

### 1. Relay hosting is expensive

A Relay subscribes to the firehose from every PDS it watches. To replicate Bluesky's full-network coverage, a Relay needs to:

- Maintain **terabytes of storage** for the recent backfill window.
- Sustain **gigabit-class throughput** during peak hours.
- Run **redundant infrastructure** because cursor-stable downtime is unacceptable for downstream AppViews.

This is enterprise-grade ops. Individual hobbyists do not run a full-network Relay. Bluesky's own published Relay specs are multi-machine deployments with substantial database backing. The cost floor is hundreds-to-thousands of dollars per month, plus the operational expertise.

### 2. PDS hosting is *technically* accessible but *operationally* niche

PDS hosting is genuinely cheap — 1 vCPU, 1 GB RAM, 20 GB SSD for 1-20 users. The published self-host repo at `github.com/bluesky-social/pds` is a one-command install.

What it requires:

- A **domain you control** (for handle resolution).
- An **HTTPS-capable server with a public IP**.
- **DNS administration** for the `_atproto` TXT record.
- **Mail server integration** for password recovery (or accept the password-loss recovery story).
- **Ongoing patching, monitoring, backups** because you're now responsible for your friends' data.

This is the same skill profile as running a personal Mastodon instance. The Mastodon experience tells us: a small fraction of people will run one, the vast majority will join an existing one, and only a fraction of those who start small instances will keep them running for years.

### 3. The "lazy trust" assumption + credible exit

The atproto-ethos article frames the design as **"lazy trust"**: users trust their PDS by default; if the PDS misbehaves, they can migrate via the rotation key. The credible-exit story is real (see [identity.md](identity.md) §"Account migration") and works in production.

But credible exit is not the same as federation diversity. Most users don't exit because their current host is fine. The migration path is for emergencies, not for routine ecosystem balance. The ecosystem's de facto centralization comes from this: nobody migrates without a reason, and Bluesky doesn't give most users a reason.

## What independent operators exist?

A non-exhaustive list of known non-Bluesky operators (early 2026):

**PDSes**: hundreds, most small. Notable ones include:
- Various community-run PDSes for specific interest groups
- Organization-operated PDSes (a few news orgs, some universities)
- The "personal PDS" cohort — individual self-hosters
- Cobalt, a managed-PDS-as-a-service offering

**Relays**: a handful. Known third-party operators:
- **Northsky** — community-operated Relay (associated with Northsky algorithms project)
- Various research Relays at universities (not production)
- Some commercial AppView operators run their own private Relays for their own consumption

**AppViews**: dozens, mostly small. Notable non-Bluesky AppViews:
- **Statusphere** — status/availability
- **Whitewind** — long-form blog
- **Smoke Signal** — events
- **Frontpage** — link aggregation
- **Bluefeed** — alternative feed AppView
- Multiple feed-generator services (custom feeds appear in `bsky.app` but are operated by third parties)

The AppView tier is **the most genuinely diverse** because the cost of running one is operational, not bandwidth-bound. Anyone can build an AppView for a niche audience; that's been the bright spot of the "Atmosphere" framing.

## The "credible exit" story — what it does and doesn't deliver

What it delivers:

- **You can migrate your repository to another PDS without losing followers.** This is real and works. The rotation key authorizes the migration; the followers see a new `service` endpoint in your DID document and re-subscribe to your new PDS.
- **You can take your data with you.** The CAR export is complete and content-addressed. You can self-host or import to a new managed PDS.
- **You retain your handle if you control the domain.** A `you.example.com` handle survives PDS migration because it's anchored in DNS, not in the PDS.

What it doesn't deliver:

- **Migration off `plc.directory`.** Your DID is `did:plc:...` and only `plc.directory` can update the operation log. Migrating to `did:web` requires creating a new DID; you lose the old one.
- **Migration off the Relay.** Even if you self-host a PDS, your followers' AppViews still talk to Bluesky's Relay. You haven't actually exited from Bluesky's infrastructure in any way that matters for visibility.
- **Survival of Bluesky's disappearance.** If `plc.directory` goes offline, your DID becomes unresolvable. There's no fallback mechanism for the registry itself.

The lazy-trust framing assumes Bluesky stays trustworthy *and* operational. Both assumptions are reasonable today, neither is structurally guaranteed.

## The "Subway" problem

There's a structural pattern worth naming. AT Protocol is shaped like a tiered transit system: the train operator (Bluesky) owns the rails (Relay), the stations (`plc.directory`), and the headline route (`bsky.app`). Independent operators can run small bus routes that connect to the trains, but they can't run trains.

Compare this to Mastodon, which is shaped like a road system: anyone can run a city's worth of roads, and the federation is *between* cities. No central rails. Less unified, more genuinely distributed.

AT Protocol gets the unified network for free; Mastodon gets the genuinely-distributed-substrate for free. Neither gets both. Atproto's "we'll federate eventually" promise is the bet that the unified network can be backed out of into a distributed substrate. Mastodon's "we federate hard" reality is the bet that the distributed substrate can be polished into a unified-feeling network.

As of 2026, atproto has not yet executed on its "federate eventually" promise in any meaningful way. The promise may still be honored; the question is when, and whether the user base is captive enough by then that nobody will care.

## What this prior art tells Myrhiza

**Myrhiza is making the opposite bet** — peer-symmetric from day one, no central operator, no "we'll federate eventually." That bet has its own costs (bootstrapping is hard, no easy growth curve) and its own benefits (no centralization gravity to fight later).

The relevant lessons:

1. **Hardware barriers determine federation reality.** If your network has a tier that requires terabytes of storage and gigabit throughput to operate, only big operators will run it, and "federation" will be a long-tail aesthetic rather than structural diversity. Myrhiza's tier-equivalents (state-apply replicas, behavior hosting) must be peer-runnable on commodity hardware.
2. **Migration paths are necessary but not sufficient.** Credible exit is real; nobody exercises it. The structural pressure that drives federation diversity has to come from somewhere else — content, monetization, governance, or just the unwillingness to centralize in the first place.
3. **"Federate eventually" is a non-binding commitment.** It survives whatever pressure pushes against it, which is usually growth, regulation, and operator self-interest. The deployment-shape is what matters; the future-tense promise is rhetorical.
4. **DID-as-registry is a centralization vector**, even with rotation-key recovery. If you're going to use DIDs, the registry's operator gets the keys-to-the-kingdom unless the registry itself is decentralized (which atproto's `did:plc` is not). Myrhiza's identity model should not assume a central registry.

## Sources

- Bluesky community wiki on PDSes: <https://atproto.wiki/en/wiki/reference/core-architecture/pds>
- atproto-ethos article (credible exit framing): <https://atproto.com/articles/atproto-ethos>
- Wikipedia AT Protocol entry, federation section: <https://en.wikipedia.org/wiki/AT_Protocol>
- "Glass Floor of Digital Sovereignty" critical analysis: <https://blog.gelbphoenix.de/the-glass-floor-of-digital-sovereignty/>
- "Is Bluesky Billionaire-Proof?" (Intercept 2023): <https://theintercept.com/2023/06/01/bluesky-owner-twitter-elon-musk/>
- Bluesky community-protocol-checkin posts on docs.bsky.app/blog
- Indigo Relay implementation: <https://github.com/bluesky-social/indigo>
- April 2026 DDoS posts (illustrate centralization fragility): bsky.social/about/blog
