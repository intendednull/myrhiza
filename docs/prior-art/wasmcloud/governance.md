**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — CNCF stewardship, maintainer governance, and the Cosmonic relationship

# Governance & stewardship

A reference snapshot of who runs wasmCloud, how decisions are made, and the load-bearing relationships to CNCF, Cosmonic, and the Bytecode Alliance. Current as of May 2026.

## CNCF stewardship

wasmCloud is a CNCF project. The CNCF page ([cncf.io/projects/wasmcloud](https://www.cncf.io/projects/wasmcloud/)) states the project history in one sentence:

> "wasmCloud was accepted to CNCF on July 13, 2021 and moved to the Incubating maturity level on November 8, 2024."

Two distinct events, three years apart:

| Event | Date | Source |
|---|---|---|
| CNCF Sandbox acceptance | 2021-07-13 | CNCF project page |
| CNCF Incubation (TOC vote) | 2024-11-08 | CNCF project page; vote |
| Public incubation announcement | 2024-11-12 | [CNCF blog](https://www.cncf.io/blog/2024/11/12/cncf-welcomes-wasmcloud-to-the-cncf-incubator/) |

The Sandbox-to-Incubation gap is in the median range for CNCF projects. The TOC's published criteria require, in summary: production deployments by multiple end users, healthy contributor diversity, a documented governance and code of conduct, regular releases, and end-user testimonies presented at a TOC meeting. The November 2024 announcement names Adobe, Orange, MachineMetrics, TM Forum CSPs, and Akamai as production users — the testimonial layer that satisfied the "small number of end users in production" bar that distinguishes Incubating from Sandbox.

### IP and trademark assignment

Per CNCF's standard charter, all CNCF-hosted projects assign their trademark and domain names to **The Linux Foundation** (the CNCF's parent 501(c)(6)). Source code remains Apache-2.0 licensed under the project's own copyright line ("Copyright The wasmCloud Authors"); the LF holds the marks. This is verifiable from the wasmCloud `wash` package metadata in `microsoft/winget-pkgs`, which lists `Copyright: Copyright The wasmCloud Authors` rather than a Cosmonic copyright.

The project is **single-licensed Apache-2.0** with a DCO (Developer Certificate of Origin) sign-off requirement on commits. There is no CLA. There is no dual-license. This is the standard CNCF posture and matches BA project practice.

### What CNCF Incubation actually buys

For Myrhiza-relevant purposes, three things:

1. **Trademark and domain ownership lives outside any single company.** If Cosmonic disappears tomorrow, the `wasmCloud` mark and `wasmcloud.com` domain stay with LF and the project continues under whichever maintainers remain. This is the structural antidote to single-vendor capture.
2. **A neutral escalation path for governance disputes.** The CNCF TOC and CNCF Code of Conduct apply.
3. **Reputational signaling for enterprise adoption.** The Adobe / Orange / Akamai users explicitly cited "CNCF status" as a procurement gate.

What CNCF Incubation does *not* buy: full-time engineering staff, paid security audits (must be applied for separately), or hosting costs. The project still relies on contributor companies for paid engineering time.

## Maintainer governance

The governance structure is documented in [`wasmCloud/wasmCloud/GOVERNANCE.md`](https://github.com/wasmCloud/wasmCloud/blob/main/GOVERNANCE.md). Two layers:

| Layer | Role | Size |
|---|---|---|
| **Org maintainers** | Project-wide direction, brand, security disclosures, budget | 2–9 people |
| **Project maintainers** | Per-codebase or per-area review and merge | Variable per area (`wash`, `go`, `typescript`, `wadm`, `wasmcloud-operator`, `wasmcloud.com`) |

### Org maintainers (current — `wasmCloud/wasmCloud/MAINTAINERS.md`, verified 2026-05-09)

| Name | GitHub | Organization |
|---|---|---|
| Bailey Hayes | `@ricochet` | Cosmonic |
| Brooks Townsend | `@brooksmtownsend` | Capital One |
| Colin Murphy | `@cdmurph32` | Adobe |
| Jordan Rash | `@jordan-rash` | Synadia |
| Liam Randall | `@LiamRandall` | Cosmonic |
| Aditya Salunkhe | `@Aditya1404Sal` | Betty Blocks |
| Victor Adossi | `@vados-cosmonic` | Cosmonic |

3 of 7 org maintainers (43%) work for Cosmonic. The remaining 4 are split across Capital One, Adobe, Synadia, and Betty Blocks. **No company holds a majority.** Compare to the BA TSC (Member Directors are 5 of 5 from member companies, but no single company holds a majority — same shape).

The top-15 contributors by commit count (`gh api repos/wasmCloud/wasmCloud/contributors`) are dominated by Cosmonic engineers (`brooksmtownsend` 385 — now Capital One; `vados-cosmonic` 219; `ricochet` 164; `lxfontes` 114; `connorsmith256` 107; `thomastaylor312` 88) but include Adobe, Synadia, T-Bank, and Betty Blocks contributors in the top tier. Engineering velocity is concentrated; org-level decision-making is not.

### Decision-making

Two scopes:

- **Org-level decisions** (governance changes, brand, removing a maintainer, budget): super-majority vote of org maintainers (two-thirds).
- **Project-level decisions** (code merges, feature direction): lazy consensus among project maintainers — 7 days without objection equals approval. Standard CNCF / open-source posture.

Maintainer turnover rules: an org maintainer who is unresponsive for 3 months loses maintainership unless a super-majority extends. This is unusually explicit — most CNCF projects leave this informal.

## The wasmCloud-Cosmonic relationship

Cosmonic Inc. is the original commercial sponsor and remains the primary contributor. The TechCrunch profile from 2023 captures the donation moment plainly:

> "Cosmonic's PaaS is enabled by the wasmCloud application runtime, which Cosmonic donated to the CNCF in 2021." ([TechCrunch, 2023-04-17](https://techcrunch.com/2023/04/17/cosmonic-launches-its-webassembly-paas-into-open-beta/))

The CNCF's own incubation post quotes Liam Randall (Cosmonic CEO) as "wasmCloud co-founder" and Bailey Hayes as "Cosmonic CTO and Bytecode Alliance Technical Steering Committee member" — *and* the project lead. The personnel overlap is total. The organizational separation, post-2021, is real but soft: Cosmonic pays many of the engineers, but the IP, the marks, the domain, and the governance structure are CNCF/LF property.

The "single-vendor risk" framing applies less harshly than at, say, HashiCorp pre-IBM or MongoDB. Two structural buffers:

1. **Trademark and code are out of Cosmonic's hands.** A Cosmonic acquisition or shutdown does not give an acquirer leverage to relicense or rename.
2. **A real diaspora of contributing companies.** Capital One's Brooks Townsend (former Cosmonic) is now an org maintainer at Capital One. Adobe, Synadia, T-Bank, and Betty Blocks employ contributors. The bus factor is not 1.

What is not buffered: **engineering velocity**. If Cosmonic stopped employing wasmCloud engineers tomorrow, a substantial fraction of the commit volume would evaporate. CNCF status preserves the project's ability to *exist*; it does not preserve the pace of v2.x development.

## Bytecode Alliance relationship

wasmCloud is **not** a Bytecode Alliance project. It is a CNCF project. The relationship runs through three channels:

1. **Cosmonic is a BA member** ([bytecodealliance.org member list](https://bytecodealliance.org/)). Bailey Hayes sits on the BA TSC as the at-large director — wearing the Cosmonic hat at BA, the wasmCloud hat at CNCF.
2. **wasmCloud consumes BA substrate.** The host embeds Wasmtime as its component runtime. The Component Model and WASI interfaces are upstream, not forks.
3. **wRPC was upstreamed.** wasmCloud's component-native RPC framework, originally developed in-tree, was donated to the BA at `bytecodealliance/wrpc` (created 2024-01-12, currently 322 stars). This is the cleanest example of the "BA owns the substrate, wasmCloud is one production-runtime adopter" pattern. wasmCloud iterated wRPC under its own maintainers, then handed it to BA when it stabilized as a substrate that other Wasm runtimes (Spin, etc.) could share.

See [BA governance](../wasm-component-model/governance.md) for the BA structure that sits underneath this.

## Working groups & sub-teams

Inside the project, organizational substructure is by **codebase**, not by SIG/WG. The MAINTAINERS file lists per-area teams:

- `@wasmCloud/org-maintainers` (the org level above)
- `@wasmCloud/wash-maintainers` (the `wash` CLI)
- `@wasmCloud/go-maintainers` (Go SDK and Go-based tooling)
- `@wasmCloud/typescript-maintainers` (TypeScript SDKs, in `wasmCloud/typescript`)
- Per-repo maintainer files in `wadm`, `wasmcloud-operator`, `wasmcloud.com`

There is no "capability providers SIG" as a formal body — capability providers are reviewed by whichever project maintainer team owns the relevant codebase. This is a flat structure compared to Kubernetes' SIG-heavy architecture, and appropriate to the project's scale (~100 regular contributors per the CNCF announcement).

## Implications for Myrhiza

Myrhiza currently has no commercial entity, no foundation, single founder. The wasmCloud trajectory offers three transferable lessons and one warning:

- **Donate early, donate clean.** Cosmonic donated to CNCF in 2021 — within ~9 months of the repo being created (2020-10-15). The donation happened *before* significant commercial product was built around it, which made the IP-assignment story uncomplicated. If Myrhiza ever needs neutral stewardship, doing it earlier costs less than later.
- **Marks and domain matter as much as code.** The CNCF/LF holding of `wasmCloud` and `wasmcloud.com` is what makes the single-vendor-disappearance question survivable. Apache-2.0 alone is not enough.
- **Diversify maintainers before you need to.** wasmCloud's 7-person org-maintainer council across 5 companies is the structural property that distinguishes it from a "Cosmonic project hosted at CNCF." Reaching that diversity *after* the project is associated with a single company is hard.
- **Warning: governance does not preserve velocity.** CNCF Incubation protects the project's existence; it does not pay engineers. A Myrhiza foundation move would protect the codebase from capture, not from stagnation. Plan the funding model separately from the governance model.

## See also

- [history.md](./history.md) — chronological reference for the dates above
- [commercial.md](./commercial.md) — Cosmonic, Cosmonic Control, and the OSS-vs-commercial boundary
- [BA governance](../wasm-component-model/governance.md) — the substrate steward
- [Agoric governance](../agoric-endo/governance.md) — comparison: single-vendor steward without a foundation
- [Holochain governance](../holochain/governance.md) — comparison: foundation + commercial entity from day one
