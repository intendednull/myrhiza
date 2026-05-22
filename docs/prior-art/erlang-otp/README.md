**Date:** 2026-05-22
**Status:** active
**Subject:** Erlang/OTP / BEAM — the canonical actor-runtime + hot-code-loading + supervision-tree ancestor

# Erlang / OTP / BEAM

Three layers that often get conflated, and the precision matters for the rest of this corpus:

- **Erlang** — the language (Prolog-flavoured pattern-matching functional language with built-in send/receive). Created at Ericsson Computer Science Lab 1986 by Joe Armstrong, Robert Virding, and Mike Williams; open-sourced 1998-12-08.
- **OTP** — "Open Telecom Platform", the standard library + design-principles framework. `gen_server`, `gen_statem`, `supervisor`, `application`, `release_handler`, `mnesia`, `ssl`, `crypto`, et al. live here. OTP is not the VM; it is library code written *in* Erlang.
- **BEAM** — the bytecode runtime (Bogdan/Björn Erlang Abstract Machine, written ~1997, replaced the older JAM interpreter). BeamAsm is the JIT *inside* BEAM, not a replacement for it. When somebody says "BEAM VM" they usually mean the whole runtime; when somebody says "Erlang VM" they almost always mean BEAM. We use **BEAM** for the runtime throughout this corpus.

Elixir (José Valim, 2011) is a separate language with its own governance that targets BEAM. Discord runs on Elixir; WhatsApp runs on Erlang; both run on BEAM. Treat Elixir as a sibling-on-BEAM project, not "Erlang with better syntax" — the governance distinction matters.

This folder exists because two Myrhiza open problems explicitly point at OTP patterns:

- **Hot-reload v2** (`prior-art/willow/open-problems.md:131-140`) — "Erlang/OTP hot code loading … the canonical reference for live code swap with state migration. Relevant patterns: `code_change/3`, supervisor restart strategies."
- **Behaviour coordination at scale** (`prior-art/willow/open-problems.md:303`) — `pg2` (now `pg`) and `global` as distributed-registry primitives for leader-election and group-membership in a peer mesh.

OTP is the only multi-decade production deployment of `(supervised actor tree + hot code swap + state migration)` on Earth. The lessons are real. The unflattering facts are also real and we surface them honestly in [`critiques.md`](critiques.md) — most production OTP shops have abandoned `release_handler`/`relup` hot-upgrades in favour of rolling restarts, BEAM has no shared-memory parallelism inside a process, RAM cost per process is real, numerical performance is poor, and the distribution protocol's default cookie-only auth is structurally unsafe outside trusted networks.

## Key facts (verified 2026-05-22)

| Fact | Value |
|---|---|
| Creators | Joe Armstrong, Robert Virding, Mike Williams (CSLab/Ericsson, 1986) |
| CSLab founders | Bjarne Däcker + Mike Williams, 1984. Däcker is credited with the *name* Erlang. |
| First language design | 1986. AXD301 production deploy 1998-03. Open-sourced 1998-12-08. |
| Joe Armstrong death | 2019-04-20 (pulmonary fibrosis); bus-factor history, not present-day signal. |
| Current major | **OTP 29.0**, released **2026-05-13**. |
| Previous major | OTP 28.0 (2025-05-21); OTP 28.5 maintenance 2026-04-23. |
| Supported series | 29.x, 28.x, 27.x (27.3.4.11 maintenance 2026-04-21). |
| JIT (BeamAsm) | x86-64 since OTP 24 (2021), AArch64 since OTP 24.2 (2021-12); both production. |
| Steward | Ericsson AB owns the trademark and the canonical implementation; community development via `erlang/otp` GitHub with multi-vendor maintainers. |
| Community foundation | Erlang Ecosystem Foundation (EEF), CA 501(c)(3), incorporated 2019. Industry working groups. |
| Elixir current | v1.19.0 (2025-10-16). BDFL: José Valim. |
| License | Apache-2.0 since OTP 18 (2015). Prior versions: EPL-1.1 (Erlang Public License). |
| Largest production deployment | WhatsApp (Erlang) — >2B daily users on Erlang infra; chat-server backbone of Meta's messaging. |
| Other flagship deployments | Discord (Elixir; ~12M concurrent users), RabbitMQ (Erlang; messaging infra), Riak KV (Erlang; basho/riak), CouchDB (Erlang). |
| Distributed registry — current | `pg` (rewritten in OTP 23, 2020-05, by WhatsApp/Meta team based on `cpg`). |
| Distributed registry — old | `pg2` — deprecated in OTP 23, **removed in OTP 24** (not deprecated-in-place; gone). |
| State-machine behaviour | `gen_statem` (introduced OTP 19.0, 2016; replaced `gen_fsm` which was deprecated in OTP 20.0, 2017). |
| Hot code load callback | `code_change/3` (and `system_code_change/4` for `sys`-bound processes); pinned across OTP 27, 28, 29 with no signature changes. |
| In-tree distributed database | `mnesia` (still ships in OTP 29; mnesia-4.x). ETS / DETS likewise. |
| Distribution default auth | Shared cookie (challenge/response, but not cryptographically secure). TLS distribution available but opt-in. |

## Contents

- [`architecture.md`](architecture.md) — BEAM scheduler model, process abstraction, message passing, supervision tree mechanics.
- [`behaviours.md`](behaviours.md) — `gen_server`, `gen_statem`, `supervisor`, `application`. The OTP design principles canon.
- [`hot-code-loading.md`](hot-code-loading.md) — Code server, two-version invariant, `code_change/3`, `release_handler` / `appup` / `relup`. **The load-bearing file for Myrhiza's hot-reload v2 question.**
- [`distribution.md`](distribution.md) — Distributed Erlang, EPMD, cookie auth, the tight-trust assumption, `global`, `pg`, `syn`, net-splits.
- [`storage.md`](storage.md) — ETS, DETS, Mnesia. What each is, what each isn't.
- [`runtime-internals.md`](runtime-internals.md) — BeamAsm JIT, scheduler binding, NIFs / dirty schedulers, reduction counting, garbage collection per-process.
- [`elixir.md`](elixir.md) — Elixir + Phoenix as the BEAM's modern surface. Where Elixir and Erlang differ in governance, syntax, and metaprogramming.
- [`production-deployments.md`](production-deployments.md) — WhatsApp, Discord, RabbitMQ, Riak, CouchDB, EMQX, Telegram bot infra. What patterns each actually uses in production (vs. what marketing claims).
- [`critiques.md`](critiques.md) — Honest weaknesses. RAM hunger, numerical perf, no shared-memory parallelism, the relup/hot-upgrade abandonment by most shops, distribution-protocol trust model, Mnesia consistency caveats.
- [`history.md`](history.md) — 1986 CSLab → 1998 AXD301 + open-source → 2014 Acquired-by-Cisco-via-Tail-f rumours that didn't happen → 2019 Armstrong death → 2019 EEF formation → 2021 BeamAsm JIT.
- [`comparisons.md`](comparisons.md) — Side-by-side with Spritely/OCapN (E-lineage vs. actor-lineage), Agoric/SwingSet (vat-snapshot vs. BEAM process state), Akka/Pekko on JVM, Orleans on .NET.
- [`open-problems.md`](open-problems.md) — What BEAM doesn't solve, and what Myrhiza inherits if it borrows from BEAM patterns.
- [`lessons.md`](lessons.md) — **The decision file.** Validates / Avoid / Borrow.
- [`glossary.md`](glossary.md) — Terms specific to this corpus.

## Canonical reading order

For a Myrhiza spec author touching hot-reload, state migration, or behaviour coordination:

1. [`lessons.md`](lessons.md) — the synthesis.
2. [`hot-code-loading.md`](hot-code-loading.md) — the mechanics behind the canonical reference.
3. [`behaviours.md`](behaviours.md) — `gen_server` / `gen_statem` / `supervisor` as the contract shapes.
4. [`critiques.md`](critiques.md) — what to be skeptical of before lifting any pattern.
5. [`distribution.md`](distribution.md) — only if peer-coordination / registry patterns are in scope.

For someone trying to *understand* what BEAM is and why it survived:

1. README (this file).
2. [`history.md`](history.md).
3. [`architecture.md`](architecture.md).
4. [`runtime-internals.md`](runtime-internals.md).

## How to use this prior-art doc

**Framing disclosure.** These docs are written from a Myrhiza-stance — Component-Model-on-WASM, peer-symmetric runtime, capabilities-as-the-only-host-surface, determinism-as-load-bearing — and most "Implications for Myrhiza" sub-sections frame OTP's choices through that lens. BEAM and the Component Model are different substrates with different determinism guarantees and different parallelism stories; the corpus systematically reads BEAM through "what could the WASM-runtime borrow from this?" Future readers auditing whether the Component-Model-on-WASM bet itself is the right primitive should weigh the corpus accordingly: it is a learn-from-OTP-into-Myrhiza artifact, not a neutral catalog of BEAM.

A second axis of bias worth naming explicitly: OTP is not a Myrhiza dependency. We are not committing to ship code that links into BEAM, and we are not committing to wire-compat with Distributed Erlang. The corpus is a *conceptual* prior-art — what patterns survived four decades of production — and the cost of "borrowing wrong" is small (a bad design choice in a future spec) rather than dependency lock-in. This is the opposite end of the spectrum from `prior-art/iroh/` (load-bearing dep).

## Sources

- Erlang/OTP releases: <https://www.erlang.org/downloads>, <https://github.com/erlang/otp/releases>
- OTP 28 release notes (2025-05-21): <https://www.erlang.org/news/180>
- OTP 29 release notes (2026-05-13): <https://www.erlang.org/news/188>
- Joe Armstrong: <https://en.wikipedia.org/wiki/Joe_Armstrong_(programmer)>
- Erlang language history: <https://en.wikipedia.org/wiki/Erlang_(programming_language)>
- Open Source Erlang Story (Erlang Solutions): <https://www.erlang-solutions.com/blog/twenty-years-of-open-source-erlang/>
- Erlang Ecosystem Foundation: <https://erlef.org/>
- BeamAsm intro: <https://blog.erlang.org/a-first-look-at-the-jit/>
- BeamAsm docs: <https://www.erlang.org/doc/apps/erts/beamasm.html>
- Elixir: <https://elixir-lang.org/>, <https://hex.pm/packages/elixir>
