# Tooling

## `hc` CLI

The Holochain command-line tool, distributed alongside the conductor binary ([hc-cli post](https://blog.holochain.org/hc-cli-test-run-and-package-your-happ/)). Subcommand groups:

| Subcommand | Purpose |
|---|---|
| `hc dna init / pack / unpack` | Create a `dna.yaml`; pack zome WASMs into a `.dna`; reverse. |
| `hc app init / pack / unpack` | Same flow for `happ.yaml` → `.happ`. |
| `hc web-app init / pack / unpack` | Wrap a `.happ` plus a UI bundle into a `.webhapp`. |
| `hc sandbox create / generate / run / call / list / clean` | Spin up disposable conductor instances for local dev. `generate` pre-loads a hApp; `call` hits the admin API. |
| `hc launch` | Launch local instances with live UI reload + the [Holochain Playground](https://github.com/darksoil-studio/holochain-playground) inspector attached. |
| `hc spin` | Newer multi-conductor dev runner with an Electron mini-Launcher for the UI ([0.2 announcement](https://blog.holochain.org/holochain-0-2-is-here/)). |
| `hc scaffold` | The scaffolder (separate crate, exposed via `hc`). |
| `hc run-local-services` | Bootstrap + signal servers for offline dev (renamed `kitsune2-bootstrap-srv` in 0.5+). |

## Scaffolder (`hc-scaffold`)

A code generator at [holochain/scaffolding](https://github.com/holochain/scaffolding) (current version `0.600.x` for Holochain 0.6, ~1.2k commits, 17 releases). Subcommands: `web-app`, `dna`, `zome`, `entry-type`, `link-type`, `collection`, `example`, `template`. Generates the full happ/dna manifests, a Rust integrity+coordinator zome pair with CRUD functions for each entry type, the matching JS client calls, and a UI scaffold using one of the built-in templates (`vanilla`, `svelte`, `headless`) or a custom Handlebars template path. Output project ships with a `flake.nix`, a `default.nix`, npm scripts (`npm run package`, `npm run start`), and a working tryorama test harness — a single `hc scaffold web-app` produces a runnable end-to-end project in under a minute.

## Holochain Launcher (Tauri/Electron)

[holochain/launcher](https://github.com/holochain/launcher). A desktop hApp browser: install `.webhapp` files from a built-in app store, multiple agent identities, runs one shared conductor across all installed apps. Originally Tauri, **reverted to Electron** in 2023 because Tauri's reliance on the OS WebView caused inconsistent rendering across users' machines. **Status as of May 2026: not actively maintained.** Last release v0.400.0, March 2025, pinned to Holochain 0.4. Officially recommended replacement: ship as a standalone Electron app via [kangaroo-electron](https://github.com/holochain-apps/kangaroo-electron). License: CAL-1.0.

## Kangaroo & p2p Shipyard

Two converging answers to "how do I get my hApp to a user."

- **kangaroo-electron** ([repo](https://github.com/holochain-apps/kangaroo-electron)) — official template, maintained by the Holochain Apps org. Bundles one hApp with one conductor and one lair keystore into a single Electron desktop binary. Branches per Holochain version (0.3 through 0.7). Auto-updates via GitHub releases, supports macOS/Windows code-signing CI. The path the core team now points everyone to.
- **p2p Shipyard** ([darksoil.studio/p2p-shipyard](https://darksoil.studio/p2p-shipyard/)) — Tauri+Nix-based, built by [darksoil studio](https://github.com/darksoil-studio). Targets Windows, macOS, Linux, **and Android** (ships as APK suitable for the Play Store) — currently the only supported route to mobile Holochain. Beta as of mid-2025; **dual-licensed** (free for non-commercial / community, paid commercial license). Funded via [Open Collective](https://opencollective.com/darksoil/projects/p2pshipyard). The substack post ["Introducing the p2p Shipyard"](https://darksoil.substack.com/p/introducing-the-p2p-shipyard) describes the goal as compressing the conductor API to "install web app / update web app / open app."

## `@holochain/client` (npm)

[npm package](https://www.npmjs.com/package/@holochain/client). The JS/TS WebSocket client. Surfaces two main classes:

- **`AdminWebsocket`** (typically `ws://127.0.0.1:65000`) — `generateAgentPubKey`, `installApp`, `enableApp`, `attachAppInterface`, `dumpFullState`, `dumpNetworkMetrics`, etc.
- **`AppWebsocket`** (typically `ws://127.0.0.1:65001`) — `callZome({ cell_id, zome_name, fn_name, payload })`, `appInfo()`, signal handler via `on(...)`. Recent versions added `dumpNetworkMetrics`; `networkInfo` was removed in 0.5.

Marked beta; maintainers recommend pinning to an exact version because every Holochain minor breaks the wire shape (notably the enum serialization change at 0.5).

## `@holochain-open-dev/*` libraries

Community-maintained reusable modules at [github.com/holochain-open-dev](https://github.com/holochain-open-dev) (~43 repos). Each module ships as a coupled pair: a Rust zome crate (entry types + zome functions) plus a TypeScript package of Lit-based Web Components and stores (so they're framework-agnostic, used by Svelte/Vue/React projects). Notable modules:

- **profiles** — nickname + avatar profile zome with a directory of agents; the de-facto identity primitive for almost every hApp.
- **file-storage** — chunked file storage on the DHT.
- **contacts** — agent contact list zome.
- **peer-status** — online/offline/busy presence.
- **notifications** — external (push) notification routing.
- **holochain-time-index** / **holochain-prefix-index** — secondary indexing primitives over DHT entries.
- **infrastructure** — scaffolding/build conventions used across the org.
- **tryorama** — the multi-conductor integration test harness (forked from holochain/tryorama into hod for tighter iteration).
- **zits** — generates TS types from Rust zome types so client and server stay in sync.
- **holochain-client-csharp** — .NET/Unity client (community port).

## Holochain VS Code extension

**There is no first-party Holochain VS Code extension.** Developers use [rust-analyzer](https://forum.holochain.org/t/rust-analyzer/2923) for the Rust side and standard Svelte/Vue/Lit extensions for UI. A 2019 Twitter poll picked VS Code as the target IDE but the dedicated extension never materialized. An old Atom snippets package for `rust-hdk` exists, unmaintained.

## Nix-based dev environment

[holochain/holonix](https://github.com/holochain/holonix). The reason Holochain insists on Nix: every hApp build pins **three coupled toolchains** — the Holochain conductor binary, the lair keystore, and a Rust toolchain with a specific WASM target — and any drift between them produces silent build/runtime breakage that's diagnostically painful. Holonix is a flake that exposes a `nix develop` shell containing matching versions of `holochain`, `lair-keystore`, `hc-scaffold`, `hn-introspect`, `rust` (with `wasm32-unknown-unknown`), plus `kitsune2-bootstrap-srv` on 0.5+. Initialized into a project with `nix flake init -t "github:holochain/holonix/main#holonix-default"`. The scaffolder generates this for you.

## direnv + `.envrc` patterns

Standard Holochain project root contains:

```
flake.nix         # holonix-derived dev shell
.envrc            # one line: `use flake`
```

With [nix-direnv](https://github.com/nix-community/nix-direnv) installed, entering the directory auto-loads the toolchain — `cargo`, `hc`, `hc-scaffold`, etc. all become available without manual shell activation. The canonical onboarding flow in every Holochain blog post and the developer portal's [setup guide](https://developer.holochain.org/get-started/install-advanced/). Works well *if* the user has Nix and direnv; it is also the single most common drop-off point for new developers, because Nix on macOS/Windows is a meaningful prerequisite.

## Implications for Myrhiza

- **Sandbox CLI verb shape is good.** Borrow `hc sandbox`'s `(create | generate | run | call | clean)` shape.
- **Scaffolder pays for itself.** A working `myrhiza scaffold` from day 0 is the single biggest onboarding accelerant. Don't ship the runtime without one.
- **Don't insist on Nix.** Holochain's Nix dependency is a recurring drop-off for new contributors. Default to a `cargo install` path with reproducible builds; let Nix be optional.
- **Don't ship a Launcher early.** The Holochain Launcher absorbed years of effort and was deprecated. Standalone-app-per-hApp (kangaroo-style) is the simpler shape; let users run multiple apps as multiple processes.

## Sources

- [hc-cli blog post](https://blog.holochain.org/hc-cli-test-run-and-package-your-happ/)
- [scaffolding repo](https://github.com/holochain/scaffolding)
- [scaffolding blog post](https://blog.holochain.org/generate-your-happs-source-code-in-seconds/)
- [launcher repo](https://github.com/holochain/launcher)
- [kangaroo-electron](https://github.com/holochain-apps/kangaroo-electron)
- [p2p Shipyard](https://darksoil.studio/p2p-shipyard/) / [substack intro](https://darksoil.substack.com/p/introducing-the-p2p-shipyard)
- [@holochain/client npm](https://www.npmjs.com/package/@holochain/client)
- [holochain-open-dev org](https://github.com/holochain-open-dev)
- [holonix repo](https://github.com/holochain/holonix)
- [Setup with Nix flakes](https://developer.holochain.org/get-started/install-advanced/)
- [hc spin announcement (0.2)](https://blog.holochain.org/holochain-0-2-is-here/)
