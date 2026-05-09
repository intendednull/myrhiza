**Date:** 2026-05-09
**Status:** active
**Subject:** CRDT library ecosystems — adoption, integrations, language ports, commercial offerings, WASM Component Model fit

# Ecosystem comparison: Automerge / Yjs / Loro

The TL;DR for spec authors: Yjs is the dominant CRDT library by every adoption metric (downloads, named production users, editor bindings, commercial ecosystem). Automerge has a smaller but more deliberate set of named users built around Ink & Switch's research orbit. Loro has effectively no named production adoption yet — its own README links to no production case studies, and its docs site historically advised against production use.

## 1. Automerge — production users

Verified named users (from Ink & Switch / Automerge community pages and sponsorship lists):

- **Ink & Switch's own prototype lineage**: PushPin (collaborative corkboard, Electron + React + Automerge + hypermerge), Patchwork (the active 2024–2026 research project on universal version control, built on Automerge), and earlier Trellis/Pixelpusher (now archived research demos). PushPin is the canonical reference app; Trellis is explicitly marked defunct in its own README.
- **GoodNotes** — funds Automerge via support contracts and feature development. Whether GoodNotes' shipped iPad sync currently routes through Automerge in production (vs. iCloud) is unclear from public sources; the relationship is documented as funding + collaboration, not a confirmed production deployment.
- **Bowtie** — uses Automerge for distributed private-network resilience (CTO quote on automerge.org).
- **Fly.io, Prisma** — listed as open-source sponsors on Automerge 2.0 announcement, not necessarily production users.

`@automerge/automerge-repo` (the batteries-included sync layer) is a separate adoption story — 682 stars, actively maintained, used in MeetingNotes (Apple-platforms SwiftUI showcase) and various Ink & Switch internal tools. It's the recommended on-ramp for new apps.

Mac/iOS/Android claim: `automerge-swift` is real and active (316 stars, last updated 2026-05-06, supports iOS / macOS / Mac Catalyst / tvOS / watchOS / visionOS). Android coverage via Kotlin is *not* a first-party effort — only via JVM/JNI to the Rust core (community efforts, unverified maturity).

## 2. Yjs — production users

Verified from the Yjs README (companies the project itself lists) and primary engineering posts:

- **JupyterLab Real-Time Collaboration** — confirmed via Jupyter blog post by Kevin Jahns ("How we made Jupyter Notebooks collaborative with Yjs") and the `jupyter-collaboration` extension. RTC ships in JupyterLab 4.x via `jupyter_collaboration`.
- **Linear** — listed in Yjs README. Linear's public sync-engine talks (Tuomas Artman) describe a custom sync engine and don't explicitly name Yjs in the publicly-available material I could verify, so Yjs's role inside Linear is plausible but underspecified outside the README listing.
- **Proton Docs** — confirmed. Proton's launch blog post (July 2024) and follow-up coverage describe Yjs as the CRDT under their E2E-encrypted collaborative editor.
- **AFFiNE, GitBook, Evernote, Lessonspace, Dynaboard, Nimbus Note, modyfi, Sana, AWS SageMaker, NextCloud, Eclipse Theia, Synthesia, Cargo** — all listed in Yjs README. Treat README listings as self-reported adoption; depth of integration varies.
- **Notion** ≠ Yjs user. Common misattribution. Notion is an Ink & Switch *research partner* on Peritext (rich-text CRDT collaboration), not a Yjs production user. No primary source places Yjs in Notion's stack.
- **GitHub Copilot Workspace** — no primary source found tying it to Yjs (unverified).

Editor binding ecosystem (where Yjs is genuinely without peer): ProseMirror, Quill, CodeMirror, Monaco, Ace, Slate, BlockSuite, Lexical, BlockNote, Tiptap, Milkdown, Superdoc — all maintained or community-maintained against Yjs.

## 3. Loro — adoption (honest scale check)

The Yjs Discourse thread "Yjs vs Loro" and the Loro homepage gather *developer testimonials* but no named, shipped, production-scale users. The closest thing to a named user is "[a] developer mentioned using Loro as the document representation for their web-based computational notebook software" — i.e., anonymous testimonial.

Loro's own historical positioning ("API and encoding schema remain experimental, not production-ready") softened with the **Loro 1.0** release announced in the README, which signals stable APIs going forward. But there are still no flagship case studies — no "Linear of Loro," no Proton-Docs-level deployment.

For Myrhiza spec authors: assume no production validation pressure on Loro yet. Pick it for its technical merits (movable tree, time travel, rich-text), not because anyone has stress-tested it at scale.

## 4. Integration surface — head-to-head

**Editor bindings** (rich-text + code editors):
- Yjs: 12+ first-class bindings (see above).
- Automerge: thinner. CodeMirror, ProseMirror, Slate bindings exist in community space but with less coverage; the Automerge community leans on `automerge-repo` patterns rather than tight editor coupling.
- Loro: ProseMirror (`loro-prosemirror`, 143 stars), CodeMirror (`loro-codemirror`, 42 stars), and demo apps (`loro-tldraw`, `loro-excalidraw`). Smaller surface, but actively maintained from inside loro-dev.

**Network providers** (sync transports):
- Yjs: `y-websocket`, `y-webrtc`, Hocuspocus, Liveblocks, PartyKit, Velt — diverse and mature.
- Automerge: `automerge-repo` ships its own websocket adapter and storage adapters (filesystem, IndexedDB, NodeFSStorage). Network architecture is opinionated; you're expected to use the repo abstraction.
- Loro: `iroh-loro` (P2P over iroh) demo exists. Otherwise transport is left to the embedder. No equivalent of Hocuspocus.

**Persistence adapters**:
- Yjs: `y-indexeddb`, `y-leveldb`, `y-redis`, plus everything Hocuspocus / Liveblocks / Y-Sweet bake in.
- Automerge: filesystem, IndexedDB, S3-via-`automerge-repo`-adapters; community Postgres adapters exist.
- Loro: import/export of binary snapshots and updates is the primary persistence story; durable-storage adapters are thinner.

## 5. Language ports / bindings

**Yjs**:
- `yrs` (Rust port, separate `y-crdt` org) — actively maintained, 1.36M crate downloads, last release ~4 months ago. Treats Yjs as the source of truth and tracks binary protocol compatibility.
- `pycrdt` (Python, on top of yrs) — actively maintained, last release 2026-03-16, used in JupyterLab.
- `ypy` — older Python binding; pycrdt is preferred now.
- `yrb` (Ruby), `ydotnet` (.NET), Swift, Java/Kotlin via yrs FFI — exist; activity varies.

**Automerge**:
- Rust core (`automerge` crate, 0.9.0).
- JS/WASM via `@automerge/automerge` 3.2.6 (wasm-bindgen, not Component Model).
- C FFI in tree.
- Swift via `automerge-swift` (active, 316 stars, 28 releases).
- Android: community work over the Rust core; not a first-party effort.

**Loro**:
- Rust core (`loro` 1.12.0 on crates.io).
- JS/WASM (`loro-crdt` 1.12.1 on npm).
- Swift via `loro-swift` (37 stars).
- Python via `loro-py` (26 stars).
- React Native via `loro-react-native`.
- All bindings are first-party from the loro-dev org and use `loro-ffi` as the FFI substrate.

## 6. Commercial offerings around CRDTs

- **Liveblocks** — managed Yjs-as-a-service. Server-side AGPL-3.0; client SDKs Apache-2.0. Edge-region storage, WebSocket transport. Notable Yjs sponsor.
- **Hocuspocus / Tiptap Cloud** (ueberdosis) — commercial managed offering on top of Hocuspocus, the open-source Yjs WebSocket backend.
- **Y-Sweet by Jamsocket** — open-source Yjs server backed by S3; Jamsocket sells the managed version.
- **PartyKit** — Cloudflare Durable Objects + `y-partykit` for Yjs-on-the-edge.
- **Velt** — managed Yjs collaboration provider.
- **Automerge** — *no commercial managed-service equivalent*. Ink & Switch is a research lab, not a SaaS vendor. Production users self-host `automerge-repo`. This is a real gap for teams that want a hosted backend.
- **Loro** — no commercial offering known. The loro-dev co-founder (Leon Zhao) is also building a separate product, "lody.ai" — unclear whether that becomes a commercial Loro host.

## 7. WASM Component Model considerations (for Myrhiza)

This is the load-bearing question for a runtime that runs apps as WASM components.

- **Automerge JS package** uses `wasm-bindgen` (not the Component Model). Compiling the Rust core to a `wasm32-unknown-unknown` core module works, but there's no published Component Model artifact and no WIT interface definition. To use Automerge inside a Myrhiza component, you'd embed the Rust crate directly and expose CRDT operations through your component's own WIT interface.
- **Loro** — same story. Pure-Rust core, JS package via wasm-bindgen, no Component Model artifact published. Embedding the `loro` crate directly inside a Rust component is the path; should compile cleanly to `wasm32-unknown-unknown` with no platform-specific dependencies (verify with `cargo component build` in our workspace before committing).
- **Yjs** — JavaScript-first. The Rust port `yrs` is the only realistic embedding target for a WASM-component runtime. yrs does compile to `wasm32-unknown-unknown` (JS bindings already do), but again: no Component Model artifact. You'd embed `yrs` and expose your own WIT.

For Myrhiza, none of the three offers a drop-in Component Model artifact. The decision reduces to: which Rust crate is most embeddable in a `state-apply`-style component?
- Automerge: most mature; deterministic merge is the design center.
- yrs: protocol-compatible with the JS ecosystem, useful if Myrhiza apps want to interop with browser Yjs clients.
- Loro: most modern API, but lower production validation.

A separate spec should evaluate determinism guarantees of each crate when embedded in `state-apply` (which Myrhiza requires to be a pure function). All three are "deterministic" in the CRDT convergence sense, but `state-apply` requires byte-level determinism — that's a tighter property and warrants empirical verification.

## Sources

- Automerge community page — https://automerge.org/community/
- Automerge 2.0 announcement — https://automerge.org/blog/automerge-2/
- Automerge GitHub — https://github.com/automerge/automerge
- automerge-swift — https://github.com/automerge/automerge-swift
- automerge-repo — https://github.com/automerge/automerge-repo
- PushPin — https://automerge.org/pushpin/ ; https://github.com/automerge/pushpin
- Trellis (archived) — https://github.com/automerge/trellis
- Patchwork — https://www.inkandswitch.com/project/patchwork/
- Ink & Switch supporters — https://www.inkandswitch.com/supporters/
- Yjs GitHub — https://github.com/yjs/yjs
- Yjs README named users (Linear, Evernote, GitBook, AFFiNE, Proton, AWS SageMaker, etc.) — https://github.com/yjs/yjs#who-is-using-yjs
- JupyterLab RTC — https://github.com/jupyterlab/jupyter-collaboration ; https://blog.jupyter.org/how-we-made-jupyter-notebooks-collaborative-with-yjs-b8dff6a9d8af
- Proton Docs launch — https://proton.me/blog/docs-proton-drive
- Liveblocks Yjs — https://liveblocks.io/technology/hosting-platform-for-yjs
- Hocuspocus — https://github.com/ueberdosis/hocuspocus ; https://tiptap.dev/docs/hocuspocus/
- Y-Sweet — https://github.com/jamsocket/y-sweet ; https://jamsocket.com/y-sweet
- PartyKit y-partykit — https://docs.partykit.io/reference/y-partykit-api/
- y-crdt (yrs) — https://github.com/y-crdt/y-crdt ; https://crates.io/crates/yrs
- pycrdt — https://github.com/y-crdt/pycrdt ; https://y-crdt.github.io/pycrdt/
- Loro GitHub — https://github.com/loro-dev/loro
- Loro org repos — https://github.com/loro-dev (loro-prosemirror, loro-codemirror, loro-swift, loro-py, loro-react-native, loro-tldraw, loro-excalidraw, iroh-loro)
- Loro vs Yjs discussion — https://discuss.yjs.dev/t/yjs-vs-loro-new-crdt-lib/2567
- WebAssembly Component Model context — https://github.com/bytecodealliance/wit-bindgen ; https://bytecodealliance.org/articles/component-model-tooling-compatibility
