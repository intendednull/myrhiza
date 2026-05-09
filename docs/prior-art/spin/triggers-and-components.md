**Date:** 2026-05-09
**Status:** active
**Subject:** Spin trigger model, WIT contracts, `spin.toml` manifest, component composition

> Sister docs: [`architecture.md`](./architecture.md) · [`glossary.md`](./glossary.md) · [`governance.md`](./governance.md) · [`sdks-and-tooling.md`](./sdks-and-tooling.md) · [`spinkube.md`](./spinkube.md) · [`comparisons.md`](./comparisons.md) · [`lessons.md`](./lessons.md)

## Trigger model overview

Spin separates **trigger** from **component**: a trigger watches an external event source and, when an event arrives, invokes a designated component through a known WIT export. The trigger owns the event loop, sockets, timers, and dispatch; the component is a stateless function that receives an event and returns a result. Triggers are first-class extension points — `crates/trigger-http`, `crates/trigger-redis`, plus user-authored triggers (Rust-only as of v4.0). Each trigger is generic over a `RuntimeFactors` set (`crates/runtime-factors`), so it brings its own slice of host capabilities.

The `Trigger` trait's `run` method takes a `TriggerApp<F>` (a precompiled, factor-bound application) and converts it into a server: `let server = self.into_server(trigger_app)?; server.serve().await`.

## HTTP trigger

The canonical trigger. Source: `crates/trigger-http`.

- **Listener**: hyper-based; binds `--listen` (default `localhost:3000`), supports `--tls-cert`/`--tls-key`.
- **Routing**: longest-prefix match against `route` patterns. `/api/foo` is exact; `/api/:user` captures one segment; `/api/...` is a trailing wildcard. Exact wins over wildcard.
- **WIT export**: components implement `wasi:http/incoming-handler.handle(request, response-out)` (WASI HTTP 0.2.x). On `main`, components targeting `wasi:http/handler@0.3.0-rc-2026-03-15` (Preview 3) are also accepted via `crates/trigger-http/src/wasip3.rs`.
- **Executors**: `spin` (Component Model, default) or `wagi` (legacy Preview 1 / CGI for older toolchains).
- **Instance reuse** (v3+): `InstanceReuseConfig` with `max_instance_reuse_count` and `idle_instance_timeout` warms instances across requests; default is fresh-per-request.

## Redis trigger

Pub/sub-shaped. Source: `crates/trigger-redis`.

- **Listener**: a Redis client subscribes only to the channels named in the manifest.
- **WIT export**: components implement `spin:redis/inbound-redis@3.0.0` — single function `handle-message(payload: list<u8>) -> result<_, error>`.
- **Server config**: `[application.trigger.redis] address = "redis://localhost:6379"` sets a default; per-trigger `address` overrides it.
- **Fan-out**: multiple components may subscribe to the same channel; each gets the message.

## WIT contracts

The host world `spin:runtime/host` (in `crates/world/src/lib.rs`) generates the host bindings; the guest worlds live in the `spin:up` package (`wit/world.wit`). The full guest-facing surface as of v4.0:

```wit
package spin:up@4.0.0;

world http-trigger {
  include platform;
  export wasi:http/handler@0.3.0-rc-2026-03-15;
}

world redis-trigger {
  include platform;
  export spin:redis/inbound-redis@3.0.0;
}

world platform {
  include wasi:cli/imports@0.2.6;
  include wasi:cli/imports@0.3.0-rc-2026-03-15;
  import wasi:http/outgoing-handler@0.2.6;
  import wasi:http/client@0.3.0-rc-2026-03-15;
  @unstable(feature = wasi-otel)
  include wasi:otel/imports@0.2.0-rc.2;
  include fermyon:spin/platform@2.0.0;
  include wasi:keyvalue/imports@0.2.0-draft2;
  import spin:key-value/key-value@3.0.0;
  import spin:mqtt/mqtt@3.0.0;
  import spin:postgres/postgres@3.0.0;
  import spin:postgres/postgres@4.2.0;
  import spin:redis/redis@3.0.0;
  import spin:sqlite/sqlite@3.1.0;
  import spin:variables/variables@3.0.0;
  import wasi:config/store@0.2.0-draft-2024-09-27;
}
```

Notable points:
- `wasi:cli/imports` brings WASI clocks, random, env, stdio, sockets, filesystem (gated per-component).
- `wasi:keyvalue` is the upstream WASI standard; `spin:key-value` is the older Spin-native interface — both are kept for back-compat, mediated by the same `factor-key-value`.
- The `fermyon:spin` package was renamed in places to `spin:*` during CNCF onboarding (2025-01-21, *pre-acquisition* — same day the `spinframework` GitHub org was created), but the `fermyon:spin/platform` legacy include is retained for components built against v2.0.

## `spin.toml` manifest

Minimal example illustrating the manifest shape:

```toml
spin_manifest_version = 2

[application]
name = "hello-spin"
version = "0.2.0"

[application.trigger.redis]
address = "redis://localhost:6379"

[[trigger.http]]
route = "/api/..."
component = "api"

[[trigger.redis]]
channel = "events"
component = "worker"

[component.api]
source = "target/wasm32-wasip2/release/api.wasm"
allowed_outbound_hosts = ["https://api.stripe.com", "https://*.cloudflare.com:443"]
key_value_stores = ["default"]
sqlite_databases = ["main"]
[component.api.build]
command = "cargo build --target wasm32-wasip2 --release"

[component.worker]
source = { registry = "ghcr.io/example/worker", reference = "v1.2.0" }

[component.api.dependencies]
"acme:s3" = { path = "../s3-client.wasm", inherit_configuration = ["allowed_outbound_hosts"] }
```

Top-level keys: `[application]` (metadata + global trigger defaults), `[[trigger.<kind>]]` (one per trigger event source), `[component.<id>]` (one per component, with capability grants), `[component.<id>.build]`, `[component.<id>.dependencies]`. `source` accepts a local path or an OCI `{ registry, reference }`.

## Component composition (`wac`)

Spin has two composition tiers:

1. **Plug-style dependencies via `[component.X.dependencies]`** — each named dep maps an imported WIT interface to a sibling component or OCI artifact that exports it. Spin satisfies the imports at build time.
2. **Arbitrary compositions via `wac`** — for transitive or many-to-many wiring beyond plug-style, the user runs `wac` (the WebAssembly Composition tool) as a build step and ships the composed `.wasm` as a single component.

SIP-023 (v4.0) controls capability inheritance for tier 1: `inherit_configuration = true | false | ["allowed_outbound_hosts", ...]`. Default is `false` — a dependency gets *zero* host capabilities unless its parent explicitly delegates them.

## Capability binding semantics

Manifest fields are the grant; the runtime is the enforcer.

- `allowed_outbound_hosts = ["scheme://host:port"]` — checked by `factor-outbound-networking` on every `wasi:sockets` connect and `wasi:http/outgoing-handler` send.
- `key_value_stores = ["name", ...]` — `factor-key-value` allows `open(name)` only for listed stores; runtime config maps `name` → backing impl (`spin`, `redis`, `azure`, `aws`).
- `sqlite_databases = ["name", ...]` — same shape; `factor-sqlite` resolves to in-process or libSQL.
- `ai_models = ["llama2-chat", ...]` — `factor-llm` gates inference targets.
- `files = [...]` — preopened FS dirs.
- `variables = { key = "{{ secret }}" }` — `factor-variables` resolves at instantiation from env, static config, Azure Key Vault, or HashiCorp Vault.

A component cannot escalate. If a WIT import is in the world but not granted in the manifest, the linker still wires it but the host-side closure traps on use.

## Distribution model

Components are OCI artifacts. `spin registry push <ref>` uploads the locked-app manifest plus per-component `.wasm` blobs plus assets, each as a content-addressed layer. `spin up <ref>` pulls, caches by digest, and runs. Notably: a Spin app is **not a single artifact** — it is the locked manifest referencing N content-addressed blobs. The runtime can stream individual components from cache without re-pulling the whole app.

Compared neighbors:
- **wasmCloud** also uses OCI for component distribution but binds capabilities at *link* time via cluster link definitions, not at manifest time.
- **Myrhiza app-bundle** target shape: locked-manifest + content-addressed component blobs, capability grants frozen in the bundle hash. Spin's manifest-static model is the right reference; wasmCloud's runtime-rebindable model is the anti-pattern for our determinism story.

## Implications for Myrhiza

- **WIT-based capability declaration is the correct shape.** Each Myrhiza kernel capability should be a WIT interface; the app manifest's grant list (`allowed_*`) maps cleanly to factor-prepared instance state. Reuse the pattern verbatim.
- **The trigger/component split maps to `interaction`/`behavior` profiles, not `state-apply`.** Spin's HTTP trigger is exactly the shape an `interaction` component wants — long-running listener, fresh component per event, full WASI. `state-apply` is the *opposite*: kernel calls component, no listener, stripped WASI, deterministic helpers only.
- **`inherit_configuration` per-key is the right composition primitive.** When Myrhiza apps grow sub-components, copy SIP-023's tri-modal grant (`true | false | [keys]`) — not the older boolean.
- **Locked-app + content-addressed blobs** is the right distribution shape; OCI is a transport choice we can keep open without coupling to it.
- **Manifest-static grants** must hash into the app-bundle digest. If a grant changes, the digest changes — that is what makes the deterministic verdict reproducible across peers.

## Sources

- `wit/world.wit` (verified via `gh api repos/spinframework/spin/contents/wit/world.wit`, 2026-05-09)
- `crates/trigger-http/src/lib.rs`, `crates/trigger-redis/`
- `crates/factor-key-value`, `crates/factor-outbound-networking`, `crates/factor-sqlite`, `crates/factor-llm`, `crates/factor-variables`
- SIP-008 (OCI registries) — `docs/content/sips/008-using-oci-registries.md`
- SIP-017 (service chaining) — `docs/content/sips/017-service-chaining.md`
- SIP-020 (component dependencies) and SIP-023 (fine-grained capability inheritance)
- Spin docs: `spinframework.dev/v3/http-trigger`, `spinframework.dev/v3/redis-trigger`, `spinframework.dev/v3/manifest-reference`, `spinframework.dev/v3/dependencies-tutorial`
