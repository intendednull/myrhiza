**Date:** 2026-05-22
**Status:** active
**Subject:** Elixir + Phoenix as the BEAM's modern surface — and where Elixir diverges from Erlang

# Elixir

Distinct language with its own governance, targeting BEAM bytecode and OTP libraries. **Do not conflate with Erlang.**

## Origins and governance

- **Created by José Valim**, 2011-01 (first commit), 2014-09 (1.0.0).
- Valim previously a Rails core team member; brought a Ruby-flavoured sensibility to the BEAM ecosystem.
- **Governance:** BDFL model. Valim has consistently named the core team in release blog posts; he remains the final decision-maker on language semantics.
- **License:** Apache-2.0.
- **Current version:** 1.19.0 (released 2025-10-16). Release cadence ~6 months for minor versions; major version bumps rare (1.0 in 2014, no 2.0 to date).
- **Hex package manager:** the BEAM ecosystem's npm-equivalent, started by Eric Meadows-Jönsson 2014. Lives at <https://hex.pm>.

## What Elixir is, technically

A meta-programming-heavy functional language that compiles to BEAM bytecode and interoperates with Erlang/OTP at the module level. Calling Erlang from Elixir is just `:gen_server.call(pid, msg)`. Calling Elixir from Erlang is just `'Elixir.MyApp.MyMod':my_fun(args)`.

The compiled output is BEAM bytecode. There is no Elixir VM. At runtime, an Elixir process and an Erlang process are indistinguishable.

## What Elixir adds over Erlang

- **Syntax** — `def`, `defmodule`, `do ... end` blocks, pipe operator `|>`, Ruby-flavoured names. Subjectively friendlier; the most-cited reason for the language's existence.
- **Macros and metaprogramming.** Elixir is built on a small core + macros all the way up. `unless`, `if`, even `case` are macros. Custom DSLs are tractable (Ecto's query DSL, Plug's middleware, Phoenix's router).
- **Protocols** — ad-hoc polymorphism: define a protocol, implement it for arbitrary types. Cleaner than Erlang's "everything is pattern-match" model.
- **Built-in `mix`** — project tool, test runner, dependency manager. The "what `rebar3` is to Erlang" story but better-integrated.
- **`exunit`** — test framework with property-testing via `stream_data`.
- **`Task` and `GenStage` / `Flow`** — higher-level concurrency primitives layered on BEAM processes.
- **String handling.** Erlang strings are lists of integers (cons cells); Elixir strings are UTF-8 binaries. Massive ergonomics win, also a massive interop wart when calling Erlang libraries that expect char-lists.
- **Charlists vs strings** — the migration cost above is real; the convention is `~c"hello"` for char-lists, `"hello"` for binaries.

## What Elixir does not add

- No new VM features. Everything Elixir does runs on BEAM.
- No new concurrency primitives. Processes, mailboxes, `Process.monitor`, `Process.link` are all the same.
- No native types BEAM doesn't have. Algebraic data types are tuples + atoms by convention.

## Phoenix — the killer framework

- **Created by Chris McCord**, first release 2014-04. Current: Phoenix 1.7.x, with 1.8 in active dev.
- HTTP web framework + WebSocket layer.
- **LiveView** (2018+) — server-rendered reactive UI over WebSocket. The killer feature; functionally a competitor to React with the server holding state.
- **Channels** — WebSocket message routing. The Discord-style "millions of concurrent socket connections per node" use case.
- **PubSub** — distributed pub/sub on top of `pg`.

Phoenix and LiveView are *the* reason most new BEAM projects since ~2018 picked Elixir over Erlang. Erlang has nothing remotely comparable in-tree or community-supported.

## Adoption split

Rough community-survey shape (no hard 2026 numbers, but trend stable for years):

- **Erlang** — telecom (Ericsson core), Riak / RabbitMQ / EMQX (messaging infra), some legacy financial systems, large parts of WhatsApp.
- **Elixir** — Discord (chat), Pinterest (parts), Spawn Wave (e-commerce), large Rails-migration cohort (Bleacher Report famously migrated, Heroku Phoenix template), most new BEAM startups since 2017.
- **Erlang-by-WhatsApp-acquisition** — WhatsApp was already deep on Erlang when Facebook acquired it (2014); has reportedly continued on Erlang though there are persistent rumours of internal Elixir adoption for new services. No public confirmation.
- **LFE (Lisp Flavoured Erlang)** — Robert Virding's BEAM-targeting Lisp. Niche; alive but small community.
- **Gleam** — statically-typed BEAM language by Louis Pilfold. Notable growth 2022–2025; v1.0 in 2024-03. Adds Hindley-Milner types on top of BEAM. The most interesting new BEAM language.

## How Elixir's tooling exceeded Erlang's

This is a politely-undersold-by-Erlang-elders fact: Elixir + `mix` + `hex` has substantially better project-tooling ergonomics than Erlang + `rebar3`. The Elixir community has invested heavily in tooling, documentation, and beginner experience; the Erlang community largely has not.

Concrete:

- `mix new my_app` → working project with tests, formatter, deps, README — in seconds.
- `mix deps.get` + `mix compile` → consistent, well-understood dep flow.
- `mix release` → self-contained deployable tarball.
- `mix format` → in-tree, configurable, universally adopted.
- Erlang community has equivalents via `rebar3` but the integration and discoverability are notably weaker.

This matters because **Elixir's tooling is the de-facto modern interface to OTP** for most new BEAM developers. When this corpus says "borrow `gen_server`'s shape," in practice that means borrowing an `Elixir.GenServer`-flavoured shape, not raw `gen_server.erl`. The patterns are the same; the documentation and learnability are not.

## Where Elixir and Erlang diverge

A small number of places where the two languages observably differ at the BEAM level:

- **Compile-time deprecation warnings.** Elixir's compiler tracks deprecations across the dependency graph; Erlang's does not.
- **Module attributes.** Elixir adds `@moduledoc`, `@doc`, `@spec` as first-class; Erlang has `-spec` but the doc attributes are libraries.
- **The `with` statement.** Elixir's `with do ... else ... end` for monadic error chains. No Erlang equivalent; Erlang's pattern is nested `case`s or third-party libraries.
- **The pipe operator `|>`** is Elixir-only. Erlang's equivalent: explicit nested calls.

These are surface differences; the BEAM-level interop is symmetric.

## Implications for Myrhiza

The Elixir-vs-Erlang story is informative for Myrhiza in one specific way: it shows that **a BEAM-flavoured language can be repackaged in a more modern surface without changing the underlying runtime**. Elixir didn't change anything about BEAM — same processes, same supervisor trees, same `code_change/3` — but it changed everything about who chose to use BEAM.

The transferable lesson: WASM Component Model + Wasmtime is the substrate. The Myrhiza app developer experience (the tooling, the language bindings, the ergonomics) is a separate concern that can evolve independently. Don't conflate "the runtime is good" with "the developer experience is good." OTP shipped good runtime + meh tooling for ~20 years before Elixir; Myrhiza can ship good runtime first and good DX later, but should be honest about the gap.

See [`lessons.md`](lessons.md) "Borrow."

## Sources

- Elixir: <https://elixir-lang.org/>
- Elixir 1.19 release notes: <https://elixir-lang.org/blog/2025/10/16/elixir-v1-19-0-released/>
- José Valim's BDFL position (Elixir Forum, multiple posts): <https://elixirforum.com/u/josevalim>
- Hex package manager: <https://hex.pm/>
- Phoenix framework: <https://www.phoenixframework.org/>
- LiveView: <https://github.com/phoenixframework/phoenix_live_view>
- Gleam: <https://gleam.run/>
- LFE: <https://lfe.io/>
- "How Discord Scaled Elixir to 5M Concurrent Users" (Discord blog, 2017): <https://discord.com/blog/how-discord-scaled-elixir-to-5-000-000-concurrent-users>
- "Real time communication at scale with Elixir at Discord" (Elixir blog, 2020): <https://elixir-lang.org/blog/2020/10/08/real-time-communication-at-scale-with-elixir-at-discord/>
