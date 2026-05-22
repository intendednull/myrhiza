**Date:** 2026-05-22
**Status:** active
**Subject:** Sandstorm — capability-secure self-hostable web app platform. Founded 2014; team acqui-hired into Cloudflare 2017-03-13; community-owned since 2024-01-14. Effectively stalled but historically critical.

# Sandstorm

The origin story for everything in this folder. Sandstorm is the project that made Cap'n Proto exist, and the team that built Sandstorm is the same team that now ships Workers RPC + Cap'n Web inside Cloudflare. Operationally Sandstorm is *not* a current platform worth betting on; historically it is the most important capability-based-OS-on-the-internet attempt of the last decade.

## Key facts

| Fact | Value |
|---|---|
| Repo | [`github.com/sandstorm-io/sandstorm`](https://github.com/sandstorm-io/sandstorm) — 7,025 stars, 705 forks, 649 open issues |
| Created | 2014-01-08 |
| Last code push | 2026-05-16 (active commits, but mostly maintenance — community-led) |
| License | Apache-2.0 |
| Co-founders | Kenton Varda, Jade Wang, Asheesh Laroia, others (Sandstorm Development Group, Inc.) |
| Operating period as for-profit | 2014–2017 (~3 years) |
| Acquired by Cloudflare | 2017-03-13, as acqui-hire for $0 (the team joined Cloudflare; the company shut down) |
| Open-sourced from | Day one — Sandstorm was always Apache-2.0, even as a startup |
| Community-owner | Sandstorm Community / Open Source Collective (since 2024-01-14) |
| Current community lead | Jacob "ocdtrekkie" Weisz |
| Current sponsor | TestMu AI (mentioned in current GitHub description) |
| Sandstorm "grain" model | Each app instance runs in an isolated grain (Docker-like, but capability-secure) |
| Tempest (rewrite) | Started late-2022 by Ian "zenhack" Denhardt; Ian died mid-2023 in an accident; Tempest stalled |

## What Sandstorm was

A self-hostable platform-as-a-service. The pitch: install Sandstorm on your own server, then install web apps (Wekan, Etherpad, GitWeb, RocketChat, etc.) by clicking a button. Each app instance is a **grain** — a capability-confined container that can talk to other grains only via capabilities the user explicitly granted.

The capability model was the strategic differentiator. Per [`sandstorm.org/about`](https://sandstorm.org/about):

- Apps run in isolated containers ("grains")
- Apps cannot access the network or filesystem except through capabilities the user grants
- Users hold capabilities; apps must request specific authorities; the user grants/denies
- Security claim: *"automatic mitigation of 95% of vulnerabilities"* — because most CVEs require ambient authority the grain doesn't have

This is the same ocap discipline Spritely talks about today, deployed in production for ~5 years (2014-2019), then stalled, then technically continued in community mode but with stuck dependencies (MongoDB 2.6 is still the database).

## How Cap'n Proto came out of it

Cap'n Proto was built *for* Sandstorm. The grain-to-grain RPC needed something capability-passing-aware (Protobuf wasn't it). Varda had been the Protocol Buffers maintainer at Google; he wrote Cap'n Proto to be "what Protobuf should have been, plus capabilities, plus zero-copy."

Per the 2017-03-13 *"Sandstorm Team Joins Cloudflare"* post, Varda wrote:

> *"they [Cloudflare] are big users – perhaps the biggest users – of Cap'n Proto, the serialization and RPC framework developed by Sandstorm."*

And specifically about Cloudflare's pre-existing use:

> *"[Cloudflare] developed Lua bindings for it and have spoken publicly about using Cap'n Proto in their logging pipeline."*

So Cloudflare had already been using Cap'n Proto in production (via `cloudflare/lua-capnproto`, still active per the otherlang.html page) *before* hiring Varda. The acqui-hire formalized a pre-existing technical dependency.

## Why the company failed

The 2017-03-13 post does not state the commercial reason in detail. Varda's framing is *"Sandstorm will no longer be our full-time jobs"* and that *"the team will continue maintaining it part-time."* External commentary (HN threads, the [Pythonpodcast](https://www.pythonpodcast.com/episodepage/episode-75-sandstorm-io-with-asheesh-laroia) episode with Asheesh Laroia, the [Wikipedia article](https://en.wikipedia.org/wiki/Sandstorm_(software))) suggests: classic self-hosting market problem (most users will pay $5/mo for hosted SaaS rather than $0 + own-server-skills for self-hosted), apps-marketplace chicken-and-egg, and the inability to fund a sales motion against incumbents like Google Workspace.

The acqui-hire structure (per outside reporting: $0 cash, the team joined Cloudflare with good packages) is consistent with "company has good people + good tech + bad business" — Cloudflare wanted the team and the Cap'n Proto expertise; the Sandstorm IP was not the prize.

## What happened post-acquisition

**2017-2019.** The Sandstorm team is at Cloudflare working on Workers (which becomes the foundation of Cap'n Proto's biggest production deployment). Varda continues maintaining Cap'n Proto as part of his job. Sandstorm-the-platform receives part-time maintenance from Varda + community contributors. Monthly releases continued through this period.

**2019-2022.** Active community development under the Sandstorm GitHub org, with Ian "zenhack" Denhardt as a key contributor. Various improvements; database (MongoDB 2.6) stays old because no one wants to do the migration.

**Late 2022.** Ian Denhardt starts **Tempest**, a from-scratch Sandstorm rewrite in Go ([originally `zenhack/tempest`](https://github.com/zenhack/tempest), now community-forked as [`sandstorm-org/tempest`](https://github.com/sandstorm-org/tempest) — *"a modern take on Sandstorm written in Go"*). Contributions to the original Sandstorm slow as energy shifts.

**Mid-2023.** Ian Denhardt dies in an accident. Tempest stalls. Sandstorm contributions, already slowing, dry up further.

**Early 2023.** Varda's own framing from the 2024-01-14 handoff: *"I gave up pushing monthly releases, since there seemed to be no point: no code changes had been made and no dependencies could be updated."*

**2024-01-14.** Varda hands ownership to the Sandstorm Community under Open Source Collective, led by Jacob "ocdtrekkie" Weisz. The blog post title: *"Sandstorm now belongs to Sandstorm.org."* This is not a Cloudflare-to-community handoff (Cloudflare never owned Sandstorm); it is Varda-personally to community.

**2024-2026.** Sandstorm Community continues maintenance under sandstorm.org. The GitHub repo (`sandstorm-io/sandstorm`) is sponsored by TestMu AI. New work is mostly dependency-updates, packaging, documentation rather than new features. Tempest remains stalled.

## Sandstorm's contribution to the lineage

Two durable artifacts:

1. **Cap'n Proto exists.** Without Sandstorm, Varda probably stays at Google; Protobuf-2 happens differently; Cloudflare doesn't have a CapTP-shaped RPC story. The entire production ocap-RPC lineage in this folder traces to Sandstorm-as-context.
2. **The grain model is a working capability OS in user-space.** It was deployed on real servers running real apps for real users for several years. The ocap academic literature (E, KeyKOS, EROS, CapDesk) had not previously shipped to that scope. Sandstorm is the existence proof that ocap-confined-process can run real apps without users-have-to-understand-it.

What did *not* get carried forward:
- The grain-as-deployment-unit model died with Sandstorm. Workers + Durable Objects are *not* grains — they're more like "the kernel exposes capabilities to your code" rather than "your code runs in a sandbox that brokers capabilities."
- The user-facing "apps marketplace" died with the company. There's no equivalent today.
- The Apache-2.0 + community-led governance survived but at maintenance-mode intensity.

## What this means for Myrhiza

Sandstorm is the cautionary tale of "do the right architectural thing + ship it + have it not catch on." The technical correctness was not what failed — the commercial model was. If Myrhiza ships a P2P runtime with the right capability discipline, it will not automatically win adoption; the adoption story is independent of the technical story. The honest read of Sandstorm: 7,025 stars, 11 years, never crossed into mass deployment, and the team's energy moved to Cloudflare.

The relevant operational takeaway: **even when the acqui-hiring company is sympathetic and the project is permissively-licensed and the team continues part-time**, the project drifts to maintenance mode within ~3-5 years. Plan Myrhiza's commercial-or-foundation backstop deliberately.

## Implications for Myrhiza

- **The capability-secure-OS-in-userspace pattern is technically tractable.** Sandstorm proved this. Myrhiza's kernel-mediated-capabilities model is the same shape and inherits the same evidence: it works, with discipline.
- **Don't bet a Myrhiza-as-product story on self-hosting adoption.** Sandstorm is the data point. The mass adoption of capability-secure self-hosting did not happen even with a polished UX, an app marketplace, and three years of full-time founder effort.
- **The acqui-hire path is real and probably the most likely Cloudflare-shaped outcome.** Plan governance accordingly — vest ownership in a foundation or community structure before the founders' incentives drift.
- **The "grain" abstraction is *not* a Myrhiza primitive.** Sandstorm grains are heavyweight Linux containers with capability brokering on the IPC; Myrhiza grains-equivalent (apps as WASM components) are lighter and more strongly-isolated. Don't borrow Sandstorm's container model; borrow only the capability-brokering pattern.
- **Apache-2.0 was the right license choice.** It enabled Cap'n Proto MIT + workerd Apache-2.0 + Sandstorm Apache-2.0 to coexist without friction. Myrhiza should pick MIT or Apache-2.0 and apply consistently.

## Sources

- [github.com/sandstorm-io/sandstorm](https://github.com/sandstorm-io/sandstorm)
- [sandstorm.io/news/2017-03-13-joining-cloudflare](https://sandstorm.io/news/2017-03-13-joining-cloudflare) — *"the team will continue maintaining it part-time"*, Cloudflare uses Cap'n Proto extensively
- [sandstorm.io/news/2024-01-14-move-to-sandstorm-org](https://sandstorm.io/news/2024-01-14-move-to-sandstorm-org) — Varda's hand-off post; *"no code changes had been made"*
- [sandstorm.org/about](https://sandstorm.org/about) — current community-led framing
- [github.com/sandstorm-org/tempest](https://github.com/sandstorm-org/tempest) — the community-fork of the rewrite (originally at `zenhack/tempest`)
- [Pythonpodcast: Sandstorm.io with Asheesh Laroia (ep. 75)](https://www.pythonpodcast.com/episodepage/episode-75-sandstorm-io-with-asheesh-laroia) — context
- [Wikipedia: Sandstorm (software)](https://en.wikipedia.org/wiki/Sandstorm_(software)) — *"acqui-hired by Cloudflare"*
