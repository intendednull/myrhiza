**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — production CM runtime, CNCF Incubating, mid-pivot from lattice-on-NATS (v1) to Kubernetes-native (v2)

# wasmCloud

A production WebAssembly Component Model runtime, started in 2020, CNCF Sandbox project since 2021-07-13, **CNCF Incubating since 2024-11-08**. Wasmtime is the embedded WASM engine; the value-add is a host process + control-plane + capability-provider story for operating fleets of components.

**The dominant fact about wasmCloud as of 2026-05-09: v2 is a pivot, not an iteration.** v2.0.0 shipped 2026-03-22 (~7 weeks before this folder's date). It removed wadm-as-orchestrator, removed the lattice abstraction, removed capability providers as a separate runtime construct, removed `wash up`/`wash down`, and replaced all of that with Kubernetes-native CRDs (`Host`, `Workload`) reconciled by a Go-based `runtime-operator`, plus in-process host plugins. Components-talk-to-each-other-over-NATS-by-default is gone — v2 components are in-process by default, with explicit opt-in wRPC for cross-host calls. The "actor" terminology is gone; both v1 and v2 say "component" now.

This folder reflects both eras because both are relevant to Myrhiza:

- **v1 era (~2020–early-2026)** — the lattice + NATS-as-control-plane + capability providers + link definitions + wadm orchestration model. Several techniques here are direct precedent for Myrhiza's kernel-mediated capability model. **It is the v1 architecture, not v2, that most resembles what Myrhiza wants.**
- **v2 era (2026-03-22 →)** — the K8s-native pivot. Mostly an anti-pattern from Myrhiza's perspective (Myrhiza is peer-symmetric P2P, no central orchestrator), but the shift itself is a useful data point on what production wasmCloud users asked for and what the project responded with.

## Key facts

| Fact | Value |
|---|---|
| Steward | wasmCloud project (CNCF Incubating); `wasmCloud` GitHub org; primary contributor company is Cosmonic Inc |
| Founders | Liam Randall, Kevin Hoffman (verified from CNCF announcement; Bailey Hayes is current tech lead, also Cosmonic co-founder, but not a wasmCloud project co-founder) |
| Repo | `wasmCloud/wasmCloud`, created **2020-10-15**, **Apache-2.0**, **2,301 stars** as of 2026-05-09 |
| CNCF Sandbox | 2021-07-13 |
| CNCF Incubating | TOC vote **2024-11-08**, public announcement **2024-11-12** |
| Maintainers | 7 org-level maintainers from 5 companies; 3-of-7 Cosmonic; no single-company majority. Trademarks held by LF Projects, LLC per CNCF policy |
| Current stable | `v2.1.0` published **2026-05-07**; `v2.0.7` patch 2026-05-05 |
| v2.0 release | **2026-03-22** — major architectural reset (see narrative above) |
| Last v1 release | `v1.x` line maintained on patch branch; v2 is the mainline |
| First Rust-host release | `v0.80.0` 2023-11-02 (the Rust rewrite that displaced the Elixir/OTP host) |
| Elixir-era repo | `wasmcloud-otp`, last push **2024-10-12** (deprecated; v2 is pure Rust) |
| `wash` CLI | v1 line `0.43.0` (2026-02-04, crates.io `wash-cli`); v2 line in-monorepo, `wash-v2.0.0-rc.7` (2026-02-19); the v2 binary is renamed `wash-runtime` |
| `wadm` | `v0.21.1` 2026-01-29; functionally **subsumed by the K8s `runtime-operator` and CRDs in `runtime.wasmcloud.dev/v1alpha1`** |
| wRPC | `bytecodealliance/wrpc` (BA-stewarded since 2024-01-12), Apache-2.0 WITH LLVM-exception, crate `wrpc 0.16.0` 2025-11-18; transports `wrpc-transport-nats 0.30.0`, `-quic 0.5.0`, `-web 0.2.0`; wire protocol const `wrpc.0.0.1` |
| Substrate | Wasmtime + WASM Component Model; see [`../wasm-component-model/`](../wasm-component-model/) for substrate-level reference |
| WASI packages used | `wasi:keyvalue` (Phase 2, **stalled** at `v0.2.0-draft` from 2024), `wasi:blobstore`, `wasi:config`, `wasi:logging`, `wasi:http`, plus `wasmcloud:bus@1.0.0`, `wasmcloud:secrets@0.1.0-draft`, `wasmcloud:messaging@0.2.0` |
| Largest production deployments | Cosmonic Control (K8s control-plane, Technical Preview since **2025-07-07**, not yet GA); no CNCF case studies as of 2026-05-09; specific company adoption is anecdotal at maintainer level |
| Browser support | None — RFC `#27` was closed without shipping |
| Cosmonic Inc | Alive and shipping as of 2026-05-09 (latest blog 2026-05-06); pivoted from "Cosmonic Connect" hosted PaaS (2023) to **Cosmonic Control** K8s-native control plane (2025-07 →); not acquired, not shut down |

(All version numbers, dates, and status verified via `gh api` / `crates.io` / `bytecodealliance/wrpc` repo / `cosmonic.com` site fetch on 2026-05-09. Any other-runtime comparison numbers — Spin 6,407 stars, Extism 5,601, Wasmer 20,654, Holochain 1,374, Iroh 8,494 — verified the same day.)

## Contents

15 files, ~2,260 lines. Each file independently skimmable. The split between v1 and v2 architecture is treated within each file rather than as separate folders, since most subsystems carried over (Wasmtime + CM + Apache-2.0 license + CNCF stewardship) and only the orchestration layer was rewritten.

**Architecture**
- [**Architecture**](architecture.md) — host process, the v1 lattice + NATS model, the v2 K8s-CRD model, the `runtime-operator`, the in-process-by-default v2 component story.
- [**Capability model**](capability-model.md) — components-as-import-only, capability providers (v1) vs in-process host plugins (v2), the WIT-typed boundary, the `wasi:keyvalue` / `wasi:blobstore` / `wasi:config` / `wasmcloud:bus` interface set.

**Inter-component contracts**
- [**wRPC**](wrpc.md) — Bytecode-Alliance-stewarded WIT-derived RPC. Wire protocol, transport plug-ability (NATS / QUIC / Web). v1 routed everything via wRPC implicitly; v2 makes wRPC explicit-opt-in. The Spin claim is corrected: Spin does **not** use wRPC in mainline.
- [**Interfaces**](interfaces.md) — three WIT package families (`wasi:*`, `wasmcloud:*`, `wrpc:*`), per-package versions, the `wasmcloud:secrets@0.1.0-draft` retirement story, the `wasi:keyvalue` Phase-2-stalled status.

**Operations & ecosystem**
- [**Tooling**](tooling.md) — v1 `wash` (`up/down/app/claims/keys/reg/spy/ctl/ctx/call/drain/inspect`) vs v2 `wash-runtime` (`build/config/completion/dev/host/new/oci/update/wit`). wadm OAM manifest pattern subsumed by `kubectl apply`.
- [**Ecosystem**](ecosystem.md) — production deployments (no CNCF case studies as of 2026-05-09), comparative GitHub-star counts, npm download stats, the "no canonical at-scale public adopter" reality.

**Project lens**
- [**Governance**](governance.md) — CNCF stewardship, TOC vote 2024-11-08, full org-maintainer table, CNCF-LF trademark assignment, BA relationship (wRPC was upstreamed to BA).
- [**History**](history.md) — 2020-10 repo created, Cosmonic 2021-03, Sandbox 2021-07, Elixir/OTP era 2021–2023, Rust-host rewrite 2023-11, v1.0 GA 2024, Incubation 2024-11, v2.0 reset 2026-03-22, current `v2.1.0` 2026-05-07.
- [**Commercial**](commercial.md) — Cosmonic Inc trajectory: 2023 Cosmonic Connect (hosted PaaS) → 2025-07 Cosmonic Control (K8s control-plane Technical Preview); alive and shipping; OSS/commercial boundary is clean.

**Synthesis**
- [**Comparisons**](comparisons.md) — vs Spin, Extism, raw Wasmtime, Holochain, Spritely OCapN, Agoric SwingSet, Iroh. Headline finding: Myrhiza ≈ "wasmCloud's component model on top of Holochain's network shape, transported by Iroh, with capability discipline closer to OCapN." Eight comparison tables.
- [**Critiques**](critiques.md) — third-party + insider critiques. The v2-reset friction (issue `#5020`), the v1 maintenance gap (wash + wadm both 3+ months stale on the v1 branch), Microsoft Hyperlight is **not** a wasmCloud production user (correction; Hyperlight is a separate CNCF Sandbox runtime).
- [**Open problems**](open-problems.md) — 12 unresolved questions: cache RAM (`#4940`), string-comparison resource bug (`#4953`), secrets (`#5016`), benchmarking (`#5052`), browser path RFC closed (`#27`).

**Reference**
- [**Lessons for Myrhiza**](lessons.md) — validates / avoid / borrow — **the consult-this-when-designing file.**
- [**Glossary**](glossary.md) — host, lattice, link def, capability provider (v1) vs host plugin (v2), wadm, wash, wRPC, OAM-vs-CRD, etc.

## Recommended reading order

For a Myrhiza spec author working on **the kernel's capability-mediation model**: [**lessons.md**](lessons.md), then [**capability-model.md**](capability-model.md), then [**architecture.md**](architecture.md). Most of what wasmCloud-v1 calls "capability providers + link definitions" is a direct working analog of what Myrhiza calls "kernel-mediated host imports." The v2 K8s pivot is the path Myrhiza explicitly is **not** taking; the architecture file makes both visible.

For a spec author working on **cross-peer component RPC**: [**wrpc.md**](wrpc.md), then [**interfaces.md**](interfaces.md), then `[../spritely-ocapn/captp-and-ocapn.md](../spritely-ocapn/captp-and-ocapn.md)` for the CapTP comparison. wRPC is interface-typed but not capability-typed in the ocap sense; that gap is the design space for Myrhiza's cross-peer story.

For a spec author working on **app-bundle distribution**: [**tooling.md**](tooling.md) (wash + wadm + OCI registry conventions), [**interfaces.md**](interfaces.md), then `[../wasm-component-model/tooling.md](../wasm-component-model/tooling.md)`.

For anyone evaluating "should we adopt wRPC for cross-peer Myrhiza calls": [**wrpc.md**](wrpc.md), [**critiques.md**](critiques.md) (the explicit-opt-in shift in v2), [**comparisons.md**](comparisons.md) §wRPC-vs-CapTP.

## How to use this prior-art doc

This corpus is reference for future Myrhiza spec writing. Pin numbers and dates accurate as of the **Date:** in this README; bump the date when meaningful churn happens upstream (next major release, next CNCF graduation step, Cosmonic exit, etc.).

**Framing disclosure.** These docs are written from a P2P, peer-symmetric, capability-mediated-host-imports stance — most "Implications for Myrhiza" sub-sections frame wasmCloud's choices through that lens. wasmCloud chose lattice/NATS (v1) and now Kubernetes (v2) as orchestration substrates; we choose neither. Future readers auditing whether *peer-symmetric* is itself the right primitive should weigh the corpus accordingly: it is a learn-from-wasmCloud-into-Myrhiza artifact, not a neutral catalog. The Spritely / Agoric / Holochain / WASM-CM folders carry the same disclosure for the same reason.

**v1-vs-v2 disclosure.** wasmCloud is mid-pivot. The folder treats both eras as data: v1 architecture is the closer Myrhiza analogue, v2 architecture is the production-running incumbent. Be explicit about which era a given lesson came from.

**Not a tutorial.** Upstream documentation (`wasmcloud.com/docs`) is the right source for hands-on use. This folder is the curated, version-pinned, Myrhiza-perspective synthesis those docs do not provide.

## Sources

- wasmCloud project: https://github.com/wasmCloud/wasmCloud
- wasmCloud documentation: https://wasmcloud.com/docs
- CNCF project page: https://www.cncf.io/projects/wasmcloud/
- wRPC: https://github.com/bytecodealliance/wrpc
- wadm: https://github.com/wasmCloud/wadm
- wasmCloud-otp (deprecated): https://github.com/wasmCloud/wasmcloud-otp
- Cosmonic Inc: https://cosmonic.com/
- Cosmonic blog index: https://blog.cosmonic.com/
- v2.0 release tag: https://github.com/wasmCloud/wasmCloud/releases/tag/v2.0.0
- CNCF Incubation announcement (2024-11-12): https://www.cncf.io/blog/2024/11/12/cncf-welcomes-wasmcloud-to-the-cncf-incubator/
