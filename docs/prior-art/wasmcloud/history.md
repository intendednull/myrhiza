**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — chronological reference from 2020 inception to v2.1.0

# History & milestones

A dated reference for wasmCloud's evolution. Every entry has a verifiable source. Where a date could not be verified to the day, the entry says so.

## 2020 — origins and the OTP era

**2020-10-15** — `wasmCloud/wasmCloud` repository created.

```bash
$ gh api repos/wasmCloud/wasmCloud --jq '.created_at'
"2020-10-15T17:41:17Z"
```

The project was created by **Liam Randall** and **Kevin Hoffman** during their time at "a top 10 US bank" (Capital One, per public talks). Source: [CNCF incubation announcement](https://www.cncf.io/blog/2024/11/12/cncf-welcomes-wasmcloud-to-the-cncf-incubator/), which states verbatim: *"The project was created by Liam Randall and Kevin Hoffman during their time at a top 10 US bank."*

Bailey Hayes joined later as project lead and is now the project's day-to-day technical lead (and Cosmonic CTO).

**2021-03-11** — `Cosmonic` GitHub organization created. Source: `gh api orgs/Cosmonic --jq '.created_at'` → `2021-03-11T02:39:06Z`. Cosmonic Inc. is the original commercial entity, founded by Liam Randall and the early wasmCloud team. The exact incorporation date is not on public sources (the GitHub org creation is the verifiable proxy).

## 2021 — the Elixir/OTP host

**2021-05-28** — `wasmCloud/wasmcloud-otp` repository created. Source: `gh api repos/wasmCloud/wasmcloud-otp` → `created: 2021-05-28T20:34:53Z`. Description: *"wasmCloud host runtime that leverages Elixir/OTP and Rust to provide simple, secure, distributed application development using the actor model."*

This is the era when wasmCloud's host was an **Elixir/OTP** application that supervised WebAssembly modules ("actors") as if they were Erlang/OTP processes. The actor terminology, lifecycle hooks, and lattice topology owe their shape to OTP. Many wasmCloud design choices that survive into v2 (the "anything talks to anything over the lattice" property, the linkdef abstraction) date from this period and are recognizably OTP-shaped.

The repo is **not archived** as of 2026-05-09 (`archived: false`), but is no longer the host; it remains for historical reference and for users on the Elixir host.

**2021-07-13** — wasmCloud accepted into the **CNCF Sandbox**. Source: [CNCF project page](https://www.cncf.io/projects/wasmcloud/), verbatim: *"wasmCloud was accepted to CNCF on July 13, 2021."*

Per the [TechCrunch profile](https://techcrunch.com/2023/04/17/cosmonic-launches-its-webassembly-paas-into-open-beta/) of 2023-04-17: *"Cosmonic's PaaS is enabled by the wasmCloud application runtime, which Cosmonic donated to the CNCF in 2021."* The donation timing roughly coincides with Sandbox acceptance.

## 2022 — Sandbox-era development

Most of 2022 is spent on the OTP host, hardening the lattice (NATS-backed control and RPC plane), and growing the capability-provider ecosystem (HTTP, key-value, blob, messaging providers).

The CNCF announcement of November 2024 retrospectively notes: *"In the last 12 months, the wasmCloud community has grown significantly. wasmCloud now has over 100 regular contributors, with overall contributions rising by 300% since 2021."* The 2021 baseline was small; 2022–2024 was the growth phase.

## 2023 — the Rust host and the Component Model rewrite

The story of 2023 is a wholesale rewrite from Elixir/OTP to a **Rust-based host** built around the WebAssembly Component Model and Wasmtime. The terminology shift "actors → components" tracks this rewrite. Pre-existing actors (`wasmcloud:actor` modules using a custom interface scheme based on smithy) had to be ported to Component-Model components implementing WIT interfaces.

The earliest tagged release of the Rust host visible in `gh api repos/wasmCloud/wasmCloud/releases` is `v0.80.0` on **2023-11-02**. Tags before this lived in the OTP repo. So the Rust host's release line begins in late 2023.

| Tag | Date |
|---|---|
| `v0.80.0` | 2023-11-02 |
| `v0.81.0` | 2023-12-28 |
| `v0.82.0` | 2024-02-14 |
| `v1.0.0-alpha.1` | 2024-03-13 |

This 4-month cadence from `v0.80` to `v1.0.0-alpha.1` is the period when the Component Model rewrite was being landed and stabilized.

## 2024 — v1.0, wRPC, and Incubation

**2024-01-12** — `bytecodealliance/wrpc` repository created. Source: `gh api repos/bytecodealliance/wrpc --jq '.created_at'` → `2024-01-12T20:27:36Z`. Description: *"Wasm component-native RPC framework."*

wRPC was developed inside wasmCloud as the Component-Model-aware replacement for the project's previous smithy-based RPC, then upstreamed to the Bytecode Alliance so other Wasm runtimes could share it. The 322-star repo is now the substrate; wasmCloud is one production consumer. (See [wrpc.md](./wrpc.md) for the framework itself and [governance.md](./governance.md) for the BA relationship.)

**2024-03-13** — `v1.0.0-alpha.1` released. The first 1.0-track release of the Rust host.

**Early 2024** — `v1.0.0` GA. The CNCF November announcement says: *"wasmCloud 1.0 was released in early 2024 as a stable, production-ready platform."* Exact GA date is not in the release JSON (releases JSON paginates and the exact `v1.0.0` final tag did not surface in the first 100 entries of the recent-first listing); the announcement language pins it to early 2024.

**2024-08-23** — CNCF blog: ["wasmCloud on the factory floor"](https://www.cncf.io/blog/2024/08/23/wasmcloud-on-the-factory-floor-efficient-and-secure-processing-of-high-velocity-machine-data/) — the MachineMetrics case study cited in the incubation testimonial bundle.

**2024-11-08** — CNCF TOC votes to move wasmCloud from Sandbox to **Incubating**. Source: [CNCF project page](https://www.cncf.io/projects/wasmcloud/) verbatim: *"moved to the Incubating maturity level on November 8, 2024."*

**2024-11-12** — Public announcement: ["CNCF welcomes wasmCloud to the CNCF Incubator"](https://www.cncf.io/blog/2024/11/12/cncf-welcomes-wasmcloud-to-the-cncf-incubator/). The post collects testimonials from Adobe (Colin Murphy), Cosmonic (Brooks Townsend, Taylor Thomas), and project leadership. Production users named: Adobe, Orange, MachineMetrics, TM Forum CSPs, Akamai. Earlier interest from Capital One, Volvo, BMW, and Intel is documented in the [TechCrunch 2023 article](https://techcrunch.com/2023/04/17/cosmonic-launches-its-webassembly-paas-into-open-beta/) but not all of those persisted into the 2024 testimonial set.

**2024-12-23** — CNCF blog: ["Navigating platform engineering pitfalls with WebAssembly components"](https://www.cncf.io/blog/2024/12/23/navigating-platform-engineering-pitfalls-with-webassembly-components/) — post-incubation thought-leadership piece positioning wasmCloud against pure-Kubernetes platform engineering.

## 2025 — v1.x maturation

The 1.x line stabilizes through 2025 with a steady release cadence. Notable from `gh api repos/wasmCloud/wasmCloud/releases` is the `v1.x` series running through into 2026, with the v2.0 work landing as `wash-v2.0.0-rc.*` tags starting in March 2026.

**2025-03-26** — Microsoft Open Source Blog: ["Hyperlight Wasm: Fast, secure, and OS-free"](https://opensource.microsoft.com/blog/2025/03/26/hyperlight-wasm-fast-secure-and-os-free/). Microsoft's Hyperlight VMM project announces a Wasm-component runtime mode. The post lists wasmCloud explicitly as a peer Wasm runtime in the same sentence as Spin and Nginx Unit. **This is parallel ecosystem positioning, not a wasmCloud + Microsoft collaboration**: Microsoft's Hyperlight-Wasm is a separate runtime that targets the same component artifacts. See [commercial.md](./commercial.md) for the commercial-layer reading of this.

## 2026 — v2.x

**2026-03-22** — `v2.0.0` released. Source: `gh api repos/wasmCloud/wasmCloud/releases` → `published_at: 2026-03-22T18:51:54Z`.

**2026-04-02 → 2026-05-05** — Patch releases `v2.0.1` through `v2.0.7` ship at roughly weekly cadence. This is a fast-iteration period stabilizing v2.x.

**2026-05-07** — `v2.1.0` released. Latest as of folder write time. Source: `gh api repos/wasmCloud/wasmCloud/releases` → tag `v2.1.0`, `published_at: 2026-05-07T17:04:14Z`.

| Tag | Date |
|---|---|
| `v2.0.0` | 2026-03-22 |
| `v2.0.1` | 2026-03-22 |
| `v2.0.2` | 2026-04-02 |
| `v2.0.3` | 2026-04-14 |
| `v2.0.4` | 2026-04-21 |
| `v2.0.5` | 2026-04-24 |
| `v2.0.6` | 2026-05-01 |
| `v2.0.7` | 2026-05-05 |
| `v2.1.0` | 2026-05-07 |

The cadence (a 2.0 GA followed by 7 patch releases and a feature minor in under 7 weeks) signals an active stabilization-and-add-features rhythm, not a project on life support.

## What changes between major versions (not a full changelog)

This section is a navigational pointer, not a changelog. For features, see [architecture.md](./architecture.md) and the per-release notes at [github.com/wasmCloud/wasmCloud/releases](https://github.com/wasmCloud/wasmCloud/releases).

- **0.x → 1.0** (Mar 2024): the OTP-to-Rust rewrite is complete; the Component Model is the only supported component shape; wRPC replaces the smithy-based RPC; `wash` reaches feature parity with the OTP-era CLI.
- **1.x → 2.0** (Mar 2026): substrate-level reset, not a feature drop. The lattice abstraction, NATS-as-data-plane, capability-providers-as-runtime-construct, link definitions, and wadm-as-scheduler are all retired. The K8s `runtime-operator` reconciles `Host` and `Workload` CRDs in `runtime.wasmcloud.dev/v1alpha1`; in-process host plugins replace external capability providers; component-to-component is in-process by default. NATS is demoted from data plane to control plane only. See [`architecture.md`](./architecture.md) for the full delta.

## Implications for Myrhiza

Three things worth internalizing from the chronology:

- **The OTP era was a learning era, and the rewrite cost was real.** wasmCloud spent ~2 years on Elixir/OTP before rewriting in Rust around the Component Model. The actor-model muscle-memory survived the rewrite (lattice, linkdefs); the host implementation did not. If Myrhiza needs to swap a runtime later, plan for a multi-quarter rewrite — the wasmCloud precedent is honest about how long that takes.
- **Sandbox-to-Incubation took 3 years and 4 months.** Plan timelines accordingly if Myrhiza ever pursues CNCF status. Production end-user testimonials are the gating constraint.
- **Major versions correspond to substrate shifts, not feature drops.** `v1.0` was "the rewrite landed." `v2.0` was "the K8s-native pivot landed; lattice + wadm + capability providers retired." Reserve major version bumps for actual architectural shifts; don't bump for marketing.

## See also

- [governance.md](./governance.md) — the structures the dates above flow through
- [commercial.md](./commercial.md) — Cosmonic's commercial trajectory aligned to these dates
- [architecture.md](./architecture.md) — the technical shape that emerged from the 2023 rewrite
- [wrpc.md](./wrpc.md) — the framework upstreamed in 2024
