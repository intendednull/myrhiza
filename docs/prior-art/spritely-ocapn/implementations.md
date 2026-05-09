# Goblins Implementations

Goblins is the actor model / object-capability framework that anchors Spritely's stack. Multiple language implementations exist at varying levels of maturity, and they interoperate over the [OCapN](https://github.com/ocapn/ocapn) protocol. The implementation story is best understood as "two first-class implementations (Guile, Racket), a browser path via Hoot, and a small constellation of partner-language implementations driven mostly by NLnet grants."

## Goblins (Guile Scheme) — primary implementation

The Guile port is now the de facto primary implementation. The repo is at [codeberg.org/spritely/goblins](https://codeberg.org/spritely/goblins). As of April 2026 the latest release is **0.18.0** ("sleepy actors"), maintained by **David Thompson** (Spritely CTO) with significant contributions from **Jessica Tallon** (Founding Technologist) and **Christine Lemmer-Webber** ([release notes](https://spritely.institute/news/spritely-goblins-v0-18-0-sleepy-actors.html)).

Features include the actor/vat model, capability-secure references, automatic local transactions, time-travel distributed debugging, persistence (originally "Aurie", landed in 0.13), the Prelay relay netlayer, TCP-TLS, Tor onion service netlayer, CapTP version `goblins-0.16`, and as of 0.18 actor-sleeping/LRU caching to disk. The Guile implementation is where new features land first.

## Goblins (Racket) — original, now in maintenance

The Racket implementation is the original announced in 2020 ([Dustycloud "Announcing Goblins"](https://dustycloud.org/blog/announcing-goblins/)). It was historically published on GitLab and has been moved to [codeberg.org/spritely/racket-goblins](https://codeberg.org/spritely/racket-goblins). The Racket version is "canonical" semantically and remains network-compatible with Guile Goblins over OCapN, but Spritely has stated it is **no longer the venue for new feature development** — it gets bug fixes only ([docs.racket-lang.org/goblins](https://docs.racket-lang.org/goblins/)). Last major release tagged on the GitLab mirror is v0.12.0. Christine Lemmer-Webber wrote most of it; current maintenance is shared with the broader Spritely team.

## Goblins (Rust port) — research/experimental

A Rust port lives at [gitlab.com/spritely/goblins-rust](https://gitlab.com/spritely/goblins-rust). It is research-shaped, not production-shaped: the repo is small, sporadically updated, and is not the focus of any of Spritely's funded work tracks. None of the recent Spritely release announcements, FOSDEM 2025 talks, or supporter-drive retrospectives mention Rust as a near-term target. Treat it as a community/exploratory effort rather than a parallel first-class runtime.

## Hoot — Scheme to WebAssembly toolchain

[Hoot](https://spritely.institute/hoot/) is Spritely's whole-program ahead-of-time compiler from Guile Scheme to WebAssembly, plus an in-house wasm assembler/disassembler/linker/interpreter. Repo: [codeberg.org/spritely/hoot](https://codeberg.org/spritely/hoot). Latest release is **0.7.0** (October 2025; see [release announcement](https://spritely.institute/news/hoot-0-7-0-released.html)). Hoot targets the Wasm 3.0 spec including tail calls and GC-managed reference types. Andy Wingo (Igalia) has been a major contributor.

Hoot is **not itself Goblins** but it is the path that puts Goblins in the browser. The "Cirkoban" jam game (Spring 2024) was the first publicly accessible app with a Goblins port-on-Hoot, though that port was partial. As of 2025–2026 there is active work in [community thread "Running Goblins on Hoot"](https://community.spritely.institute/t/running-goblins-on-hoot/690) to compile full Goblins via Hoot; the Goblinville Spring Lisp Game Jam 2025 entry was the first multiplayer demo of Hoot + Goblins together in a browser.

## Goblins for JavaScript — not a thing, by design

There is **no Spritely-shipped JavaScript port of Goblins**. The JavaScript object-capability story is owned by **Endo** (Agoric's [endojs/endo](https://github.com/endojs/endo)) which provides `@endo/captp` and is participating in OCapN standardization. Spritely's stance is multi-implementation interop via OCapN rather than rewriting Goblins per language: in late April 2026 the team confirmed Goblin Chat working between Goblins (Guile/Racket), the Dart "DObjects" implementation (Ridley, NLnet-funded), and EndoJS ([interop progress thread](https://community.spritely.institute/t/ocapn-interoperability-progress/810)). Cap'n Proto is a longer-term interop target.

## Interop matrix summary

| Implementation | Language | Status (May 2026) | OCapN interop confirmed |
|---|---|---|---|
| Guile Goblins | Guile Scheme | Active, 0.18.0 | Racket, EndoJS, DObjects |
| Racket Goblins | Racket | Maintenance only, 0.12.x | Guile, EndoJS, DObjects |
| Goblins Rust | Rust | Experimental, not funded | None publicly demonstrated |
| Hoot (browser path) | Guile→Wasm | Active, 0.7.0; partial Goblins port | Demoed via Goblinville 2025 |
| EndoJS | JavaScript | Active (Agoric/MetaMask) | Goblins (April 2026) |
| DObjects | Dart | NLnet-funded (Ridley) | Goblins (chat works) |

## Sources

- [Goblins (Guile) repo on Codeberg](https://codeberg.org/spritely/goblins)
- [Racket Goblins repo on Codeberg](https://codeberg.org/spritely/racket-goblins)
- [Goblins Rust on GitLab](https://gitlab.com/spritely/goblins-rust)
- [Hoot on Codeberg](https://codeberg.org/spritely/hoot)
- [Goblins v0.18.0 release notes](https://spritely.institute/news/spritely-goblins-v0-18-0-sleepy-actors.html)
- [Goblins v0.12.0 release notes (netlayers)](https://spritely.institute/news/spritely-goblins-v0-12-0-released-two-new-netlayers-join-the-ocapn-family-and-more.html)
- [Hoot 0.7.0 release notes](https://spritely.institute/news/hoot-0-7-0-released.html)
- [OCapN interoperability progress](https://community.spritely.institute/t/ocapn-interoperability-progress/810)
- [Running Goblins on Hoot thread](https://community.spritely.institute/t/running-goblins-on-hoot/690)
- [Endo (JavaScript ocap) repo](https://github.com/endojs/endo)
- [Racket Goblins docs](https://docs.racket-lang.org/goblins/)
