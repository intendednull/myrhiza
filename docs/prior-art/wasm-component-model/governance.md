**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — steward, foundation, member companies, and the CG/WG/BA relationship

# Governance & funding

A reference snapshot of who runs the WebAssembly Component Model substrate, how the standards process actually works, and where the money comes from. Current as of May 2026.

## Three bodies, not one

The substrate Myrhiza would adopt sits at the intersection of three distinct organizations. Conflating them is the most common error in this space.

| Body | Form | Role |
|---|---|---|
| **W3C WebAssembly Community Group** ("the CG") | W3C Community Group, open to anyone with a W3C account | Drafts proposals, including the Component Model. CG output is non-normative until the WG promotes it. |
| **W3C WebAssembly Working Group** ("the WG") | W3C Working Group, member-only | Promotes CG-stable proposals to W3C Recommendation. Tied to the W3C Process. |
| **Bytecode Alliance** ("BA") | 501(c)(6) industry consortium | Stewards the dominant reference implementations (Wasmtime, Cranelift, WAMR, wit-bindgen, jco, cargo-component). Not a standards body. |

The Component Model spec repository (`WebAssembly/component-model`, created 2021-09-22) explicitly lives under the W3C `WebAssembly` GitHub org and states: *"All Component Model work is done as part of the W3C WebAssembly Community Group."* ([repo README](https://github.com/WebAssembly/component-model/blob/main/README.md))

The repo also notes the spec is **incrementally stabilized** through the WASI Preview 2 milestone (with Preview 3 planned for async/threads). There is no separate W3C Recommendation track for the Component Model itself today; what exists is a CG proposal whose binary-format and Canonical ABI live in `design/mvp/`.

## The Bytecode Alliance

### Founding

Publicly announced **2019-11-12** by Mozilla, Fastly, Intel, and Red Hat as a four-org consortium. The GitHub organization `bytecodealliance` was created **2019-08-12T17:48:21Z** (three months before the public launch). ([gh api orgs/bytecodealliance](https://api.github.com/orgs/bytecodealliance), [Mozilla 2019 announcement](https://hacks.mozilla.org/2019/11/announcing-the-bytecode-alliance/))

### The 501(c)(6) reorganization

In 2021 the loose alliance reorganized into the **Bytecode Alliance Foundation, Inc.**, a 501(c)(6) trade association under US tax law. (501(c)(6) is the same structure used by the Linux Foundation, OpenSSF, and the CNCF parent. Member dues are tax-deductible as a business expense rather than a charitable donation.) The bylaws and IP Policy are public PDFs at [bytecodealliance.org/assets/bylaws.pdf](https://bytecodealliance.org/assets/bylaws.pdf) and [/assets/ip-policy.pdf](https://bytecodealliance.org/assets/ip-policy.pdf).

The exact incorporation date is not on the public site footer (which reads "Copyright © 2019-2023 the Bytecode Alliance contributors"). What we can verify: the foundation was operational and accepting members by mid-2021.

### Current member roster (May 2026)

Scraped from [bytecodealliance.org](https://bytecodealliance.org/) — the on-page member list:

Amazon, Anaconda, Copia Wealth Studios, Cosmonic, Endor, Fastly, Fermyon, Futurewei, Igalia, imec, Intel, DFINITY (Internet Computer), JAF Labs, Microsoft, Midokura, Mimic, Mozilla, NGINX, Shopify, StackBlitz, Stellar Development Foundation, UC San Diego, University of Luxembourg (SnT).

Notable absences: **Google** (the V8 team contributes to core WASM at the W3C but is not a BA member), **Apple**, **AWS** parent (Amazon listed but not as the AWS division specifically), and **Cloudflare** (which has its own `workerd` runtime, separate from BA-stewarded ones).

### Board of Directors (May 2026)

From [bytecodealliance.org/about](https://bytecodealliance.org/about):

- **Bobby Holley** (Mozilla) — Member Director, **Board Chair**
- **Bailey Hayes** — At-Large Director (also TSC Elected Delegate)
- **Pat Hickey** — At-Large Director
- **Tyler McMullen** (Fastly) — Member Director
- **Till Schneidereit** — TSC Director
- **Oscar Spencer** (F5) — Member Director (also TSC Chair)
- **Ralph Squillace** (Microsoft) — Member Director, **Treasurer**
- **Deian Stefan** (UCSD) — Member Director
- **David Bryant** — Consulting Executive Director (non-voting; supports board, oversees operations)

Two-year terms, staggered, elections every December. At-Large directors are elected by the Recognized Contributor program — the BA's vehicle for individual (non-member-company) contributors. Bylaws section 5.5.

### Technical Steering Committee

Per [bytecodealliance.org/about](https://bytecodealliance.org/about) and the [TSC charter](https://github.com/bytecodealliance/governance/blob/main/TSC/charter.md):

- **Oscar Spencer** — TSC Chair
- **Bailey Hayes** — Elected Delegate
- **Till Schneidereit** — Appointed Delegate (also TSC Director on the Board)
- **Christof Petig** — Elected Delegate

The TSC governs hosted projects and SIGs and runs the Recognized Contributor program. Project-level technical decisions are explicitly *delegated* to project maintainers — the TSC mediates only on cross-project deadlocks. (Bylaws value: *"Localized governance wherever possible."*)

### Hosted projects

107 public repos under `bytecodealliance` as of May 2026. The load-bearing ones for the Component Model substrate:

- **Wasmtime** — production runtime (Apache-2.0, 17,977 stars, 1,697 forks).
- **Cranelift** — code generator backend (lives in the wasmtime monorepo; also used by Firefox SpiderMonkey and rustc_codegen_cranelift).
- **wasm-tools** — CLI for inspecting/transforming components.
- **wit-bindgen** — generates language bindings from WIT.
- **cargo-component** — Cargo subcommand for building Rust components (latest: v0.21.1, 2025-04-07; pre-1.0).
- **jco** — JavaScript-side component tooling and bindings (latest: jco-v1.19.0, 2026-04-22; **post-1.0**).
- **WAMR** (`wasm-micro-runtime`) — embedded/IoT runtime (Apache-2.0, 5,916 stars).

## Funding model

**Industry-funded membership dues + member-employed engineers.** This is the operative fact for Myrhiza spec authors:

- **No token.** No ICO, no airdrop, no governance token. (Compare [Holochain's HOT](../holochain/governance.md) or [Iroh's commercial layer](../iroh/governance.md).)
- **No external raise.** The Bytecode Alliance Foundation does not take VC money, does not issue equity. It is a non-profit trade association.
- **Member dues** fund foundation operations: legal, infrastructure, events, the Consulting Executive Director.
- **Member companies fund their own engineers.** Mozilla pays Cranelift contributors. Fastly pays Wasmtime contributors. Microsoft pays the Hyperlight and Spin-adjacent teams. Cosmonic and Fermyon pay the wasmCloud and Spin contributors. The foundation does not employ the engineers; it provides the legal vehicle, IP policy, and coordination.
- **No paid-tier features.** All hosted projects are Apache-2.0 (Wasmtime, WAMR, wit-bindgen) or MIT/Apache-2.0 dual. There is no enterprise edition under the BA umbrella; commercial offerings (Fermyon Cloud, Cosmonic Connect, Fastly Compute@Edge) live at member companies, not at the foundation.

The bylaws explicitly enshrine *"Influence through effort"*: **"We grant influence and decision-making authority through ongoing efforts towards our alliance's vision and goals, not through monetary contributions."** Members pay dues but do not buy votes; technical authority flows through the TSC and project maintainers, not the Board.

## How CG → WG → BA actually works

In contrast to most W3C standards, where browsers vendor competing implementations and the WG mediates, WebAssembly's server-side story is **dominated by a single reference implementation: Wasmtime**. The Component Model has the following pipeline:

1. **CG proposal** lives in `WebAssembly/component-model`. Anyone with a W3C account can attend CG meetings (biweekly Component Model subgroup).
2. **Reference implementation** is built into Wasmtime (and into JavaScript via jco) at BA. Implementation experience feeds back into spec changes.
3. **WASI Preview milestones** (`WebAssembly/WASI/releases`) act as the de-facto stabilization checkpoints — `wasip2` 0.2.0 (Jan 2024) and the in-flight `wasip3` work.
4. **W3C WG promotion** has not happened for the Component Model itself. Core WebAssembly 1.0 reached W3C Recommendation in 2019; 2.0 was promoted to Recommendation in 2025. The Component Model is **not on the WG track** today.

This means: a Myrhiza component bundle's binary format is governed by a CG proposal whose stability guarantee is "Wasmtime ships it." Not "the W3C has Recommended it."

## What "BA-stewarded" means for outside contributors

The BA's process bylaws say the right things — *"What we develop is Free and Open Source, and available for everyone, not just our members"* and *"We accept all contributors who are willing and able to collaborate."* Empirically, this is mostly true: outside PRs land in Wasmtime regularly, the Recognized Contributor program gives non-members board representation, and the spec discussion on the CG is open.

But the load-bearing technical decisions — Canonical ABI shape, async semantics, the resource-handle redesign — are made by engineers paid by member companies (primarily Fastly, Mozilla, Microsoft, Fermyon, Cosmonic). A non-member adopter has no formal seat at that table. The leverage available to outside projects is: file issues, attend CG meetings, run a fork.

For Myrhiza this is acceptable on its face — the BA is more open than most foundation-led infrastructure (compare the Linux Foundation or CNCF), the licenses are permissive, and the technical direction has so far been compatible with capability-based runtimes. The risks worth tracking:

- **Async-Component model** is a primary axis where BA priorities (high-throughput cloud serverless) may diverge from Myrhiza's priorities (deterministic event apply on heterogeneous peers).
- **Determinism guarantees** are not first-class in the Canonical ABI today. Floats, NaN canonicalization, and host-call ordering are not specified at the level Myrhiza's `state-apply` profile requires; we will need to enforce determinism at the host (Wasmtime config) layer, not at the CM layer.
- **CG proposal status** means the binary format can still change. We pin to a specific Wasmtime major version and treat that as ABI-stable; we do not chase nightly CM changes.

## Cross-references

- [WASI Preview history & milestones](history.md)
- [Alternative runtime landscape](ecosystem.md)
- [Holochain's governance for comparison](../holochain/governance.md) — token-funded, foundation-led
- [Iroh as transport substrate](../iroh/governance.md) — VC-backed, single-company-led

## Sources

- https://bytecodealliance.org/about
- https://bytecodealliance.org/membership
- https://bytecodealliance.org/assets/bylaws.pdf
- https://bytecodealliance.org/assets/ip-policy.pdf
- https://github.com/bytecodealliance/governance/blob/main/TSC/charter.md
- https://github.com/WebAssembly/component-model
- https://github.com/WebAssembly/component-model/blob/main/README.md
- https://github.com/bytecodealliance/wasmtime
- https://api.github.com/orgs/bytecodealliance
- https://hacks.mozilla.org/2019/11/announcing-the-bytecode-alliance/
- https://www.w3.org/community/webassembly/
