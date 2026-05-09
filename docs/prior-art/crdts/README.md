**Date:** 2026-05-09
**Status:** active
**Subject:** CRDT survey — Automerge + Yjs + Loro; the three production-grade open-source libraries Myrhiza could build state-apply convergence on top of

# CRDT prior-art survey

Multi-library reference folder. Three libraries — Automerge, Yjs, Loro — sized up against each other and the academic CRDT lineage that sits behind them. Folder exists so future Myrhiza spec authors deciding how `state-apply` components converge across peers have a curated reading rather than starting from scratch.

This is a *survey* folder, not a single-system folder. Each library has its own deep-dive file; cross-cutting files (theory, history, ecosystem, governance, comparisons, open-problems, critiques) sit alongside.

## Key facts at a glance

| Library | Stable | Stars | License | Core lang | Text algo | Stewardship | Bus factor |
|---|---|---|---|---|---|---|---|
| **Automerge** | `@automerge/automerge` 3.2.6 (2026-04-22), crate `automerge` 0.9.0 | 6,258 | MIT | Rust core + JS/WASM/C FFI | RGA + Peritext marks | Ink & Switch (Alex Good, Orion Henry full-time; Martin Kleppmann advisor) | 3+ |
| **Yjs** | `yjs` 13.6.30 stable; v14 in prerelease (npm `next` 14.0.0-8, `beta` 14.0.0-16) | 21,791 | MIT | Pure JS (port: `yrs` 0.26.0 Rust) | YATA | Kevin Jahns (`@dmonad`) solo, GitHub Sponsors | **1** |
| **Loro** | `loro` 1.12.0 crate, `loro-crdt` 1.12.1 npm (2026-04-29) | 5,594 | MIT | Rust-native + WASM/Swift via `loro-ffi` | Fugue (text), Peritext (marks), Moveable Tree | `loro-dev` org; Zixuan Chen (`@zxch3n`) ~81% authorship | **1** |

All three are MIT. None ship as a WASM Component Model artifact — Myrhiza will need to wrap whichever it picks.

## How to use

Read in this order:

1. **[crdt-theory.md](crdt-theory.md)** — what a CRDT is, the text-CRDT family tree (Treedoc → Logoot → WOOT → RGA → YATA → Fugue → Eg-walker), the interleaving anomaly, why text is harder than counters.
2. **[history.md](history.md)** — chronological lineage 2006-2026; INRIA/RWTH/Cambridge/Ink & Switch/Loro tracks running parallel; reading order for Myrhiza-flavored study.
3. **[automerge.md](automerge.md)**, **[yjs.md](yjs.md)**, **[loro.md](loro.md)** — per-library deep dives. Architecture, algorithms, sync protocol, performance, determinism analysis, Myrhiza implications.
4. **[comparisons.md](comparisons.md)** — head-to-head: algorithm, performance, document size, sync protocol, WASM compilability, determinism. Recommendation table.
5. **[ecosystem.md](ecosystem.md)** — verified production users (Yjs: Proton Docs, JupyterLab, AFFiNE; Automerge: GoodNotes sponsorship, NLnet grant, Bowtie; Loro: no named at-scale users). Notion-uses-Yjs is debunked here.
6. **[governance.md](governance.md)** — bus-factor analysis. Yjs bus-factor 1 (Kevin Jahns alone). Loro bus-factor 1 (Zixuan Chen ~81% commits). Automerge healthier (3+ active full-time + Kleppmann + Ink & Switch staff).
7. **[critiques.md](critiques.md)** — third-party voices. Kleppmann's own Eg-walker pivot. Joseph Gentle's "academic CRDTs are crazy slow." Marijn Haverbeke on OT vs CRDTs. Aaron Boodman / Rocicorp on server-authority dichotomy. Per-library criticism.
8. **[open-problems.md](open-problems.md)** — what NO CRDT solves: schema evolution, authority/Byzantine resistance, invariant validation (bank-account problem), tombstone GC, on-disk schema migration, cross-library interop, WASM Component Model wrapping.
9. **[lessons.md](lessons.md)** — *the decision file*. Validates / avoid / borrow synthesis for Myrhiza `state-apply` design.
10. **[glossary.md](glossary.md)** — terms across all three libraries.

If you only have time for two files: read **lessons.md** + **comparisons.md**.

## Why this folder exists

Myrhiza `state-apply` components must be **deterministic pure functions** of `(prior state, event)`. CRDTs are one mechanism for achieving cross-peer convergence; this corpus exists to make the choice with eyes open.

The three libraries represent three coherent design points:

- **Automerge** — uniform RGA-flavored algorithm, Rust core + FFI, healthy stewardship, ~7 years of production hardening, Peritext for rich text. The "boring choice" for production.
- **Yjs** — pure-JS YATA implementation, the de-facto rich-text editor standard (largest binding ecosystem of the three by an order of magnitude), single-maintainer risk. The Rust port `yrs` is what Myrhiza would actually consume.
- **Loro** — Rust-native, per-container algorithm choice (Fugue + Peritext + Moveable Tree), youngest, no published at-scale production users. The "technical-merit bet" with maturity gap.

## Honest scale disclosure

- **Yjs** is shipping at scale: Linear lists Yjs in their stack (commercially); JupyterLab and Proton Docs ship Yjs as dependency; Tiptap-via-Hocuspocus sits in front of many enterprise editors.
- **Automerge** is shipping but smaller: Ink & Switch demos (PushPin, Pixelpusher), GoodNotes sponsorship of `automerge-swift`, Bowtie commercial use. Less editor-ecosystem volume than Yjs.
- **Loro** has zero named at-scale production users that we could verify. Treat as "shipping research-grade." Don't soft-pedal this if recommending Loro.

## Framing disclosure

These docs are written from the **Myrhiza-as-deterministic-state-apply-runtime** stance — the "Implications for Myrhiza" sub-sections in each per-library file frame each library's choices through that lens. A reader auditing whether deterministic-state-apply is itself the right primitive should weigh the corpus accordingly: it is a learn-from-CRDTs-into-Myrhiza-state-apply artifact, not a neutral catalog. CRDT-skeptics like Aaron Boodman (Rocicorp/Replicache, server-authority architecture) and Kleppmann's own Eg-walker pivot are quoted in [critiques.md](critiques.md) and [open-problems.md](open-problems.md) precisely so the bias is visible.

The corpus also reads through the **WASM Component Model substrate** lens (see [`../wasm-component-model/`](../wasm-component-model/)) — none of the three libraries ship as Component Model artifacts today, and that gap shapes the lessons.

## Sources

Per-file `## Sources` sections list URLs cited in that file. The aggregate top-level sources:

- Automerge: <https://github.com/automerge/automerge>, <https://automerge.org/>
- Yjs: <https://github.com/yjs/yjs>, <https://docs.yjs.dev>
- Loro: <https://github.com/loro-dev/loro>, <https://loro.dev>
- y-crdt (Rust port of Yjs): <https://github.com/y-crdt/y-crdt>
- Shapiro et al. 2011 INRIA tech report (CRDT foundations)
- Martin Kleppmann's papers index: <https://martin.kleppmann.com/papers/>
- Ink & Switch local-first essay: <https://www.inkandswitch.com/local-first/>
