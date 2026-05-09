**Date:** 2026-05-09
**Status:** active
**Subject:** Spin — language SDKs, CLI, and OCI distribution tooling

# Spin SDKs & Tooling

Snapshot of Spin's developer-facing surface — the CLI, language SDKs, composition tools, and OCI-distribution path. Written so Myrhiza spec authors can borrow what works without inheriting Spin's K8s-deployment assumptions. Companion files in this folder: [glossary](./glossary.md), [architecture](./architecture.md), [triggers-and-components](./triggers-and-components.md), [governance](./governance.md), [comparisons](./comparisons.md), [lessons](./lessons.md), [open-problems](./open-problems.md), [SpinKube](./spinkube.md). Cross-prior-art neighbours: [wasmCloud tooling](../wasmcloud/tooling.md), [WASM Component Model tooling](../wasm-component-model/).

## Verified versions (2026-05-09)

| Component | Version | Notes |
|---|---|---|
| `spin` CLI / runtime | v4.0.0 | 2026-04-20, Apache-2.0, github.com/spinframework/spin |
| `spin-sdk` (Rust crate) | tracks v4.0.0 series | `#[http_component]` macro; key-value, SQLite, MySQL, Redis, LLM clients |
| `@spinframework/spin-sdk` (JS/TS) | build-tools v2.0.0 (2026-04-29) | supersedes `@fermyon/spin-sdk`; based on ComponentizeJS |
| `spin-go-sdk` | tracks v4 | TinyGo + wasm32-wasip2 |
| `spin-python-sdk` | v4.0.0 series | componentize-py-based |
| `wac` (composition) | v0.10.0 (2026-04-17) | Bytecode Alliance, `cargo install wac-cli` |
| `wkg` (wasm-pkg-tools) | 0.8.x line | BA-stewarded; Spin uses it for package resolution since 2.6 |
| `containerd-shim-spin` | v0.24.0 (April 2026) | runs Spin v3.6.3-compatible workloads |
| `spin-operator` | v0.6.1 (2025-07-09) | Kubernetes operator |

## Spin CLI

Single Rust binary. Installation: `curl -fsSL https://spinframework.dev/downloads/install.sh | bash` or `brew install fermyon/tap/spin` (Homebrew tap pre-acquisition; expected to migrate to a `spinframework` tap).

Core commands ([v4 CLI reference](https://spinframework.dev/v4/cli-reference)):

| Command | Purpose |
|---|---|
| `spin new` | scaffold from a template (`-t http-rust`, `-t http-ts`, `-t http-py`, `-t http-go`, etc.) |
| `spin build` | invoke per-component build commands declared in `spin.toml`, produce WASM artifacts |
| `spin up` | run the application locally, dispatching on declared triggers |
| `spin watch` | rebuild + restart on filesystem change — the dev-loop primitive |
| `spin registry push` | package the app + its components as an OCI artifact and push to a registry |
| `spin registry pull` | fetch a Spin app by OCI reference, into the local cache |
| `spin deploy` | plugin-routed deploy (Fermyon Cloud, SpinKube via `spin kube`, etc.) |
| `spin templates` | manage template repositories (install from Git, list, upgrade) |
| `spin plugins` | install/uninstall first-party and community plugins |

Plugins extend the surface: `spin kube`, `spin cloud`, `spin test`, `spin doctor`, `spin trigger-mqtt`. The plugin manifest lives in a registry; `spin plugins install kube` fetches and installs.

## JavaScript / TypeScript SDK

Repository: [github.com/spinframework/spin-js-sdk](https://github.com/spinframework/spin-js-sdk) (migrated from `fermyon/spin-js-sdk`). The current generation supersedes an experimental v1 (`sdk-v1` branch).

Build path: TypeScript/JS sources are componentized via [ComponentizeJS](https://github.com/bytecodealliance/ComponentizeJS) — a Bytecode Alliance project that embeds a stripped-down JS engine (SpiderMonkey) into a WASM Component Model artifact. The SDK provides typed wrappers around imported world interfaces (HTTP, KV, SQLite, etc.).

Hello-world flow:
```bash
spin new -t http-ts hello -a
cd hello && npm install
spin build && spin up
```

Note on package naming: the npm package was historically `@fermyon/spin-sdk`. As of 2026 it is being mirrored / republished under `@spinframework/spin-sdk` following the Fermyon→spinframework org migration; both are presently in flight.

## Rust SDK

Crate: [`spin-sdk`](https://crates.io/crates/spin-sdk). Repository: [github.com/spinframework/spin-rust-sdk](https://github.com/spinframework/spin-rust-sdk).

API shape:
- `#[http_component]` attribute macro on the entry function — generates the WIT-compliant export.
- `IntoResponse` / `FromRequest` traits for ergonomic handler signatures.
- Submodules for each capability: `spin_sdk::key_value`, `spin_sdk::sqlite`, `spin_sdk::mysql`, `spin_sdk::redis`, `spin_sdk::llm`, `spin_sdk::variables`.
- Compiled to `wasm32-wasip2` (WASI 0.2). The `wasm32-wasip1` target is no longer the default.

The Rust SDK is the most mature of the four — Spin itself is Rust, and the SDK was the first to land Component Model support.

## Go SDK

Repository: [github.com/spinframework/spin-go-sdk](https://github.com/spinframework/spin-go-sdk). Build path: TinyGo with `GOOS=wasip2 tinygo build -target=wasip2`. TinyGo's wasip2 support landed late-2024 and stabilized through 2025; the SDK exposes HTTP, KV, SQLite, Postgres, MySQL, Redis, LLM, MQTT helpers. Production-ready for HTTP services; the long tail (concurrent goroutines + WASI threads) still has rough edges shared with TinyGo upstream.

## Python SDK

Repository: [github.com/spinframework/spin-python-sdk](https://github.com/spinframework/spin-python-sdk). Build path: [componentize-py](https://github.com/bytecodealliance/componentize-py) (Bytecode Alliance) bundles a CPython-equivalent interpreter into a Component Model artifact. The current SDK targets Spin 4.0+ (`spin-sdk==4.0.0` on PyPI). Async/await supported. Cold-start cost is non-trivial (interpreter + stdlib in the artifact); fine for non-edge workloads, less suited to per-request scaling.

## Third-party SDKs

- **Zig** — community-maintained; partial coverage; not in the official template list.
- **Moonbit** — third-party experiment; useful as a proof point that Spin's WIT-driven SDK story scales to new languages, not yet a production option.

Honest read: production usage skews Rust > JS/TS > Go > Python. Anything else is exploratory.

## Component composition: `wac`

[wac](https://github.com/bytecodealliance/wac) (WebAssembly Composition, "whack") is a Bytecode Alliance tool that wires multiple components into one. The composition is declarative: a `.wac` file specifies which exports of one component satisfy the imports of another, and `wac compose` produces a single artifact. Spin uses `wac` internally during `spin build` when an app declares cross-component dependencies. Latest: v0.10.0, 2026-04-17.

This matters for Myrhiza: composition is the right primitive for "one app, several state-apply / state-propose / interaction components in one artifact." `wac` already solves the build-time composition problem.

## OCI distribution

Spin treats components as OCI artifacts. `spin registry push ghcr.io/foo/myapp:0.1.0` packages `spin.toml` + every referenced WASM component into a multi-layer OCI artifact and pushes to any standards-compliant registry (GHCR, Docker Hub, ECR, ACR, Harbor, ttl.sh). `spin registry pull` is the inverse. The wire format follows the [CNCF TAG Runtime WASM OCI artifact layout](https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/).

Since Spin 2.6, package resolution (the `package = "ns:name@version"` field in `spin.toml`) goes through [`wkg`](https://github.com/bytecodealliance/wasm-pkg-tools) — Bytecode Alliance's `wasm-pkg-tools` — which reads `~/.config/wasm-pkg/config.toml` and supports OCI, Warg, and local-file backends. `wkg oci push` is the lower-level primitive.

## Local dev experience

- `spin watch` — file-watcher → rebuild → restart loop. Configurable via `[component.X.build]` watch globs in `spin.toml`.
- `spin up --listen 0.0.0.0:3000` — bind. Supports environment-variable injection and a `.env`-style runtime config file.
- Variables and secrets: `[variables]` section in `spin.toml` declares typed variables; runtime config file (`runtime-config.toml`) supplies values per environment, with optional Vault / Azure Key Vault providers.

## Testing tooling

- `spin test` plugin — runs WASM unit tests against component exports.
- HTTP integration tests are typically written in the host language (`reqwest` against a live `spin up`); first-class WASM-side fixtures remain limited.
- The Rust SDK ships `#[cfg(test)]` mocks for KV, SQLite, etc., usable in standard `cargo test`.

## Implications for Myrhiza

- **componentize-js / componentize-py / TinyGo wasip2 are the right build paths.** Don't reinvent. If Myrhiza apps target the WASM Component Model (and they do), every language story goes through one of these tools. Track them as load-bearing dependencies.
- **`wac` is the right composition primitive.** Myrhiza apps with multiple component profiles (state-apply, state-propose, interaction, behavior) compose at build time; `wac` already handles import/export wiring.
- **OCI-as-component-registry is a clean pattern.** Apps as OCI artifacts means the entire Docker/OCI tooling ecosystem (signing via cosign, SBOM, vulnerability scanning, mirroring) applies for free. Myrhiza should adopt the WASM OCI artifact layout, not invent its own.
- **`wkg` resolves the dependency-graph problem already.** When Myrhiza apps import shared interfaces (e.g. a deterministic-helpers world), `wkg`-style package resolution beats hand-rolled URL fetching.
- **The CLI shape — `new`/`build`/`up`/`watch`/`registry push` — is a proven local-dev loop.** Worth borrowing verbatim. Note `spin deploy` is plugin-routed; Myrhiza's deploy story (gossip-publish to peers) maps cleanly to a plugin-style verb.

## Sources

- Spin v4.0.0 release: [github.com/spinframework/spin/releases](https://github.com/spinframework/spin/releases)
- Spin CLI reference: [spinframework.dev/v4/cli-reference](https://spinframework.dev/v4/cli-reference)
- Spin Rust SDK: [github.com/spinframework/spin-rust-sdk](https://github.com/spinframework/spin-rust-sdk), [crates.io/crates/spin-sdk](https://crates.io/crates/spin-sdk)
- Spin JS SDK: [github.com/spinframework/spin-js-sdk](https://github.com/spinframework/spin-js-sdk)
- Spin Go SDK: [github.com/spinframework/spin-go-sdk](https://github.com/spinframework/spin-go-sdk)
- Spin Python SDK: [github.com/spinframework/spin-python-sdk](https://github.com/spinframework/spin-python-sdk)
- ComponentizeJS: [github.com/bytecodealliance/ComponentizeJS](https://github.com/bytecodealliance/ComponentizeJS)
- componentize-py: [github.com/bytecodealliance/componentize-py](https://github.com/bytecodealliance/componentize-py)
- wac: [github.com/bytecodealliance/wac](https://github.com/bytecodealliance/wac)
- wasm-pkg-tools / wkg: [github.com/bytecodealliance/wasm-pkg-tools](https://github.com/bytecodealliance/wasm-pkg-tools)
- Spin SIP-008 (OCI registries): [github.com/spinframework/spin/blob/main/docs/content/sips/008-using-oci-registries.md](https://github.com/spinframework/spin/blob/main/docs/content/sips/008-using-oci-registries.md)
- CNCF TAG Runtime WASM OCI artifact layout: [tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact](https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/)
- Microsoft OSS blog on WASM-via-OCI: [opensource.microsoft.com/blog/2024/09/25](https://opensource.microsoft.com/blog/2024/09/25/distributing-webassembly-components-using-oci-registries/)
- Akamai acquires Fermyon (2025-12-01): [akamai.com/newsroom/press-release](https://www.akamai.com/newsroom/press-release/akamai-announces-acquisition-of-function-as-a-service-company-fermyon)
