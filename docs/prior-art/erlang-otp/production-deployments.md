**Date:** 2026-05-22
**Status:** active
**Subject:** Production deployments — what each shop actually uses (vs. marketing claims)

# Production deployments

Verified as of 2026-05-22. Where claims are uncertain, we say so.

## WhatsApp (Meta)

- **On Erlang since founding (~2009).** Acquired by Facebook 2014-02-19. Status as of 2026: still primarily Erlang on the chat-server backbone.
- **Headline number:** >2B daily active users, single-digit-engineer team running the chat-server tier (the team's exact size has changed over time; the "<50 engineers for hundreds of millions of users" framing was Jamie Allen / Rick Reed-era circa 2014).
- **Patterns publicly known:**
  - Heavily-modified BEAM (FreeBSD-tuned, custom schedulers, NIF-heavy).
  - Authored the new `pg` module (OTP 23, 2020) when they outgrew `pg2`. Codename internally was *`erlang_pg`*.
  - Custom message-store, custom NIF-heavy crypto, custom routing. Not stock OTP.
- **What changed at the Meta acquisition:** WhatsApp's infrastructure was kept largely intact. Reports of Meta-internal pressure to migrate to Hack/HHVM never materialised; the Erlang stack survived.
- **Hot code loading?** Historically yes. Current practice undocumented; deployment patterns are not public.

**This is the single most-cited "Erlang in production at scale" example.** The reason: nobody else matches the user/engineer ratio. The discount factor: WhatsApp's Erlang is *heavily customised*; "WhatsApp uses Erlang" is not the same as "you should run stock OTP."

## Discord

- **Elixir + Erlang since founding (~2015).** Hansihe (the Rustler author) is a Discord engineer.
- **Patterns publicly known:**
  - Phoenix Channels for WebSocket connections.
  - Custom message router; `pg`-based presence sharding.
  - **Rustler for performance-critical NIFs** — sorted sets, message parsing.
  - Stated scale: >12M concurrent users, >26M WebSocket events/sec, multi-million-concurrent-users-per-node footprint.
- **Custom infrastructure:**
  - "ManifoldWork" (announced ~2020) — internal job scheduler for cross-node task fan-out, replacing earlier `:rpc.multicall` patterns that didn't scale.
  - Custom presence implementation (multiple iterations published on the Discord engineering blog).
- **Hot code loading?** Discord engineers have publicly described their deploy pipeline as `:code.load_file`-based without full `relup`; gen_servers get reloaded on next message naturally.

**Discord is the canonical "Elixir at scale" case study.** Their engineering blog is some of the best free OTP literature in existence.

## RabbitMQ

- **Pivotal Software → VMware → Broadcom (acquired 2023-11) custody.** Open-source MPL-2.0 / Apache-2.0 hybrid.
- **Written in Erlang since ~2007.**
- **Patterns publicly known:**
  - Mnesia historically for cluster metadata; migrating to **Khepri** (Raft-based, built on `ra`) — feature-flagged in RabbitMQ 3.13.0 (2024-03-11), default for new deployments in RabbitMQ 4.2.0 (2024-10-27). See [`storage.md`](storage.md).
  - Plugin architecture is OTP-application-based.
  - Multiple peer-discovery backends (k8s, Consul, etcd, AWS).
- **Net-split history:** RabbitMQ has shipped multiple "partition handling strategies" (autoheal, pause_minority, ignore) over the years. Each one's failure modes are documented. This is some of the most honest in-production-postmortem documentation in the BEAM world.

## Riak KV (Basho → archived → community-maintained)

- **Eventually-consistent distributed KV store, Erlang.**
- **Basho Technologies went bankrupt 2017-08.** Source open-sourced; community-maintained as TI Tokyo since 2018. Still active in 2026 (notably Riak 3.x maintained by TI Tokyo and a small community).
- **Patterns:**
  - `riak_core` library underlies the ring topology; widely lifted into other Erlang systems (Antidote DB, ChicagoBoss historical).
  - Custom CRDT library `riak_dt`. Canonical reference for production-deployed CRDTs.
  - Used `riak_ensemble` (a Raft implementation) for strongly-consistent buckets.

**The cautionary tale:** Basho's collapse despite real engineering excellence. The fact that Riak's bankruptcy didn't kill the technology — community-maintained, repos live, customer deployments persisted — is itself a lesson about how OSS stewardship can outlast corporate stewardship. See [`history.md`](history.md).

## CouchDB

- **Apache Foundation custody since 2008.**
- **Originally Erlang; "rewrite parts in Rust" project SpiderMonkey-replacement landed in CouchDB 4.x but the storage layer remains Erlang.**
- Uses Mnesia internally for cluster state.
- Modest active development; the project shipped 3.4 in 2024 and is steady but not booming.

## EMQX

- **MQTT broker, Erlang.** Originally by Feng Lee, now maintained by EMQ Technologies.
- **Apache-2.0 license**, has both an open-source community edition and a commercial enterprise edition.
- **Patterns:**
  - Custom shared-subscriptions, MQTT bridges, rule engine.
  - Sponsors of the Erlang Ecosystem Foundation.
- **Scale claims:** EMQX customers reportedly run multi-million MQTT connections per node. Independently verified examples include Volkswagen, HiveMQ-competitor deployments.

## Bet365

- **Online betting, Erlang since at least 2013.**
- **Notable for being one of the few high-frequency-trading-adjacent users of Erlang.** Their engineering posts (mostly via Code BEAM conference talks) describe sub-millisecond Erlang call paths and heavy NIF use.

## Cisco / Tail-f

- **Tail-f Systems (Swedish startup, ConfD network device config product) acquired by Cisco 2014.**
- **ConfD is Erlang-based;** Cisco continues to ship it as a network-device management framework.
- This is the rare "Erlang as embedded product inside a larger company's commercial offering" case study.

## "WhatsApp moved off Erlang" — the persistent rumour

Periodic HN/Reddit speculation that Meta is migrating WhatsApp off Erlang. **As of 2026-05-22, no public confirmation.** The original 2014-acquisition-era worries did not materialise. New-service skirmishes (some chat features written in non-Erlang) appear from time to time but the chat-server tier remains Erlang.

This is the kind of fact worth re-verifying every year. Treat as "still on Erlang to the best public information" with the standard caveats.

## What the deployment list says

**Pattern: the BEAM's killer-app domain has always been "millions of long-lived stateful connections with low per-connection latency"** — chat, MQTT, messaging brokers, presence services. The list is dominated by exactly that shape:

- WhatsApp: chat backbone.
- Discord: chat.
- RabbitMQ: message broker.
- EMQX: MQTT broker.
- Bet365: real-time odds.
- Phoenix LiveView apps: stateful web sessions.

**Domains where BEAM has *not* taken hold:**

- Numerical / scientific computing — BEAM's poor numerical performance is the blocker.
- Batch data processing — Spark/Flink/Beam ecosystems own this; no BEAM equivalent at scale.
- ML / inference serving — entirely Python + Rust + GPU territory.
- Game engines — single-frame latency budgets are wrong shape for BEAM's preemption.
- Embedded resource-constrained — BEAM's RAM footprint is too high (Nerves project notwithstanding; great for Raspberry Pi class, not microcontroller class).

**Implications for Myrhiza:** Myrhiza is targeting **peer-symmetric apps with many long-lived connections and event-driven state** — squarely in BEAM's sweet spot, just over WASM substrate instead of BEAM. The production validation that "this shape of system survives at planetary scale" is real and is encouraging. The discount factor: BEAM's sweet-spot shops customise heavily; "stock-OTP is enough" is a marketing claim, not an operational one. See [`lessons.md`](lessons.md).

## Sources

- WhatsApp engineering (Rick Reed, 2014): <https://www.youtube.com/watch?v=c12cYAUTXXs> ("That's 'Billion' with a B")
- Discord blog: <https://discord.com/category/engineering>
- "How Discord Scaled Elixir to 5,000,000 Concurrent Users": <https://discord.com/blog/how-discord-scaled-elixir-to-5-000-000-concurrent-users>
- "Using Rust to Scale Elixir for 11 Million Concurrent Users" (Discord blog): <https://discord.com/blog/using-rust-to-scale-elixir-for-11-million-concurrent-users>
- RabbitMQ blog: <https://www.rabbitmq.com/blog/>
- Riak community fork: <https://github.com/basho/riak>
- TI Tokyo Riak releases: <https://tiot.jp/riak-distribution/>
- EMQX: <https://www.emqx.com/>
- Bet365 Erlang case studies: <https://codesync.global/speakers/?query=bet365>
- Tail-f / ConfD (Cisco): <https://www.tail-f.com/>
- "Erlang at WhatsApp" (Pierre Fenoll, 2018): <https://www.erlang-factory.com/sfbay2014/anton-lavrik>
