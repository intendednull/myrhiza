**Date:** 2026-05-22
**Status:** active
**Subject:** Erlang/OTP timeline — 1984 CSLab to 2026 OTP 29

# History

Date-pinned, verified where possible. Where we say "claim", we're flagging that the dating is community lore rather than primary source.

## 1984 — Computer Science Laboratory founded

Bjarne Däcker and Mike Williams set up the Computer Science Lab at Ericsson, Sweden. Charter: explore new programming-language technologies for Ericsson telecom switches.

## 1986 — Erlang created

Joe Armstrong joins the lab. With Williams and Robert Virding, designs a Prolog-influenced functional language for telephony software. Original name: "Ericsson Language" → Erlang (the name attributed to Bjarne Däcker, possibly punning on A.K. Erlang the queueing theorist — the lab has not authoritatively confirmed).

First implementation: an interpreter in Prolog, by Armstrong. Maybe 100 lines.

## 1991 — First commercial implementation

JAM (Joe's Abstract Machine) interpreter, written in C. Slow but production-deployable.

## 1996 — OTP first release

OTP R1 internal at Ericsson. The library set + behaviour modules + supervision-tree patterns. **OTP became a standalone product within Ericsson at this point** — Erlang the language ships with OTP the framework as a unified release thereafter.

## 1997 — BEAM (Bogdan/Björn Erlang Abstract Machine)

Replaces JAM. New bytecode VM by Bogumil "Bogdan" Hausman, refined by Björn Gustavsson. Major speedup. **This is the runtime that survives to present day**, evolved continuously since 1997.

## 1998-03 — AXD301 launches

The flagship Erlang-in-production deployment. Ericsson's ATM switch with >1.13M lines of Erlang. Reportedly hit "nine 9s" of reliability (99.9999999%) over deployed-lifetime, though that number has caveats — see Joe Armstrong's "Mythology" paper for the honest version.

## 1998-02 — In-house Erlang ban

Ericsson Radio Systems internally bans Erlang for new products, preferring "non-proprietary" languages (C, Java). This is the political fork that leads to:

## 1998-12-08 — Erlang open-sourced

Released under Erlang Public License (EPL 1.1, a Mozilla-1.1 derivative). Most of the original team — including Armstrong, Virding, Williams — leaves Ericsson to form **Bluetail AB**, a startup commercialising Erlang.

The release itself was uneventful: no PR, no media coverage. The team treated it as routine.

## 2000 — Bluetail acquired by Alteon → Nortel

Bluetail (Erlang messaging products) acquired by Alteon WebSystems 2000-02-15, which was itself acquired by Nortel Networks 2000-10-23. Some of the team continued at Nortel.

## 2001 — Erlang user community starts publishing

First Erlang User Conference (EUC) at Ericsson Stockholm. Annual since.

## 2006 — *Programming Erlang* book

Joe Armstrong publishes *Programming Erlang* with Pragmatic Bookshelf. **This is the book that introduces the wider programmer community to Erlang.** Sales modest by industry standards; influence outsized.

## 2007 — Erlang Solutions founded

Francesco Cesarini founds Erlang Solutions, the consultancy that becomes the de-facto commercial nexus for the language. Hosts conferences, employs core team members, sponsors EEF.

## 2007 — RabbitMQ released

LShift releases RabbitMQ (Erlang AMQP broker). Becomes one of the most-deployed Erlang systems ever.

## 2007 — Ericsson buys back Erlang activity

After Bluetail / Nortel diaspora, Ericsson re-acquires Erlang/OTP development under the **Erlang/OTP team** at Ericsson, which it has been ever since. This is where Lukas Larsson, John Högberg, and other current core team members ultimately landed.

## 2008-12 — CouchDB joins Apache

CouchDB (Erlang document database, Damien Katz) becomes an Apache top-level project. Long-running Erlang showcase.

## 2009 — Basho founded; Riak

Basho Technologies founded; ships Riak KV (Erlang Dynamo-clone). Becomes second flagship Erlang database after CouchDB.

## 2009-01-19 — WhatsApp founded

Jan Koum + Brian Acton. Reportedly built on Erlang from earliest days.

## 2011-01-09 — Elixir first commit

José Valim starts the Elixir project. Initial goal: a Ruby-flavoured language on BEAM, but with BEAM's concurrency primitives preserved.

## 2012 — Erlang 15 (R15B)

The R15 release moves to true semantic versioning and modernises the build. The "R" prefix lingered through R16, dropped in OTP 17 (2014).

## 2014-02-19 — Facebook acquires WhatsApp

$19B deal. Erlang community holds its breath. WhatsApp's Erlang stack survives.

## 2014-04 — Phoenix framework

Chris McCord releases Phoenix 0.1. Becomes the Rails-of-BEAM.

## 2014-05-13 — Tail-f acquired by Cisco

The Erlang-based ConfD network device manager. Cisco continues shipping it.

## 2014-09-18 — Elixir 1.0.0

The "stable" Elixir release. Adoption inflection point.

## 2015 — License change: EPL → Apache-2.0

OTP 18.0 (2015-06-24) ships under Apache-2.0. Resolves a long-standing community complaint about EPL's MPL-1.1 derivation. **This was the moment OTP became unambiguously "OSS-license-friendly"** for corporate adoption.

## 2016 — `gen_statem` lands in OTP 19

Replaces `gen_fsm`. Major behaviour-library evolution.

## 2017-08-15 — Basho bankruptcy

Riak's commercial parent collapses. The technology survives via community fork; TI Tokyo eventually picks it up. **Notable as a cautionary tale**: Basho was the most-funded Erlang startup at the time; raised >$60M; collapsed despite real engineering excellence. See [`production-deployments.md`](production-deployments.md).

## 2017 — Discord public Elixir engineering posts

The Discord engineering blog publishes "How Discord Scaled Elixir to 5,000,000 Concurrent Users." Becomes the canonical "Elixir at scale" reference.

## 2019-03 — Erlang Ecosystem Foundation (EEF) formed

Announced at Code BEAM SF 2019 in a Thursday evening keynote by José Valim, Peer Stritzinger, Fred Hébert, Miriam Pena, and Francesco Cesarini. Incorporates as California 501(c)(3), Sunnyvale CA. **The first time the BEAM community has had a vendor-neutral foundation** since Ericsson started the language.

## 2019-04-20 — Joe Armstrong dies

Pulmonary fibrosis. Age 68. **Bus-factor history, not present-day signal** — Armstrong had been a thought-leader and evangelist, not the core implementor for many years; the OTP team at Ericsson continued unchanged. Community memorials and the EEF Joe Armstrong Memorial Lecture series followed.

## 2019-08 — pg2 deprecation announced

For OTP 23. The replacement `pg` module (by the WhatsApp/Meta team) ships in OTP 23 (2020-05-13). pg2 is removed in OTP 24 (2021-05-12).

## 2020-05-13 — OTP 23.0

Notable for the new `pg`, `socket` module improvements, and the floor laid for BeamAsm.

## 2020-09-08 — BeamAsm PR (otp#2745)

Lukas Larsson submits the JIT PR. Merged later that year.

## 2021-05-12 — OTP 24.0

Ships BeamAsm production-default on x86-64. ~50% throughput gain on benchmarks.

## 2021-12-15 — OTP 24.2

Ships AArch64 BeamAsm. Apple Silicon and AWS Graviton become first-class.

## 2023-11 — Broadcom acquires VMware → RabbitMQ

VMware's acquisition of RabbitMQ stewardship transfers to Broadcom. Community concern about future investment; as of 2026 the project continues OSS with paid commercial support.

## 2024-03-04 — Gleam 1.0

Louis Pilfold's statically-typed BEAM language reaches 1.0. The most interesting new BEAM language in a decade.

## 2024-03 / 2024-10 — RabbitMQ Khepri trajectory

Khepri (Raft-based cluster-metadata store, built on `ra`) shipped feature-flagged in RabbitMQ 3.13.0 (2024-03-11). Became the **default for new deployments** in RabbitMQ 4.2.0 (2024-10-27). RabbitMQ 4.0 and 4.1 still defaulted to Mnesia. Mnesia support remains in-tree but slated for removal in a future major version. **The flagship Mnesia user has chosen Raft.**

## 2025-05-21 — OTP 28.0

Priority messages (EEP 76), zip generators in comprehensions, EEP 75 based float literals, atom-size-limit relaxation, PCRE2, zstd module, nominal types in Dialyzer.

## 2025-10-16 — Elixir 1.19.0

Latest Elixir release. Set-theoretic typing further developed.

## 2026-05-13 — OTP 29.0

Native records (EEP-79, experimental), `is_integer/3` range guard BIF, multi-valued comprehensions, variable binding in comprehensions, **SSH "secure by default"** (shell/exec disabled by default — a notable security-posture shift), post-quantum hybrid SSL key exchange (`x25519mlkem768`), `io_ansi` terminal styling, `ct_doctest` for testing documentation examples.

## Patterns worth noting

- **Continuity of stewardship.** Ericsson has stewarded OTP almost continuously since 1986, with one diaspora (Bluetail, 1998–2007) that ended in re-absorption. **Very few language runtimes have this level of single-organisation continuity.**
- **Community foundation, late.** EEF formed 2019 — more than 30 years after the language. The pattern: Ericsson kept the runtime; the community kept the ecosystem (Erlang Solutions, hex.pm, Phoenix, Elixir); the foundation formalised the latter relationship.
- **License clean-up arrived late.** The 2015 EPL → Apache-2.0 move was a deliberate community-friendliness shift. Many OSS-adjacent projects pre-2015 had to negotiate around the EPL.
- **The "new BEAM language" pattern.** Elixir (2011), Gleam (2016), LFE (2008), Hamler. The BEAM-as-substrate-for-multiple-languages pattern is well-established and is itself a load-bearing precedent for "your runtime can host languages it wasn't designed for."

## Sources

- *Programming Erlang*, 2nd ed., Joe Armstrong (Pragmatic Bookshelf, 2013)
- Erlang Wikipedia page: <https://en.wikipedia.org/wiki/Erlang_(programming_language)>
- "Twenty Years of Open Source Erlang" (Erlang Solutions, 2018): <https://www.erlang-solutions.com/blog/twenty-years-of-open-source-erlang/>
- "A (Very) Brief History of Erlang" (Vonage Developer blog): <https://developer.vonage.com/en/blog/a-very-brief-history-of-erlang>
- Joe Armstrong on Wikipedia: <https://en.wikipedia.org/wiki/Joe_Armstrong_(programmer)>
- EEF: <https://erlef.org/about>
- "Erlang the Movie" (Ericsson, 1991, restored): <https://www.youtube.com/watch?v=xrIjfIjssLE>
- OTP releases on GitHub: <https://github.com/erlang/otp/releases>
- OTP 28 release: <https://www.erlang.org/news/180>
- OTP 29 release: <https://www.erlang.org/news/188>
- AXD301 case study (Armstrong, 2003): <https://www.cs.kent.ac.uk/people/staff/sjt/TFP2003/papers/Armstrong/armstrong.pdf>
