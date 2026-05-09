**Date:** 2026-05-09
**Status:** active
**Subject:** Head-to-head comparison of Automerge, Yjs, and Loro across the axes that matter for Myrhiza's `state-apply` deterministic-replay design.

This file synthesizes across `automerge.md`, `yjs.md`, and `loro.md`. Read those for per-library depth. Numbers cite published benchmarks; where benchmark methodology is contested, that is flagged.

## 1. At-a-glance

| Dim | Automerge | Yjs | Loro |
|---|---|---|---|
| Stars | 6.3K | 21.8K | 5.6K |
| Created | 2017 (Kleppmann et al., Ink & Switch) | 2014 (Kevin Jahns) | 2022 (Loro-dev / zxch3n) |
| Latest stable (May 2026) | 3.2.6 (`@automerge/automerge` JS/TS) / 0.9.0 (`automerge` Rust crate) | 13.6.30 (`yjs` stable); v14 prerelease — npm `next` 14.0.0-8, `beta` 14.0.0-16 | 1.12.1 (`loro-crdt` npm) / 1.12.0 (`loro` Rust crate) |
| License | MIT | MIT | MIT |
| Core language | Rust (`automerge` crate); TS/Wasm wrapper | JavaScript (TS); separate Rust port `yrs` | Rust (`loro`); Wasm + native bindings |
| Text algo | RGA-flavored (with columnar encoding) | YATA | Fugue (with custom rich-text + Peritext influence) |
| Stewardship | Ink & Switch + community (multi-maintainer) | Solo maintainer (Jahns) + funded community | Loro-dev startup (zxch3n + small team) |
| Bus factor | Medium (multiple committers) | Low (one critical-path maintainer) | Low-medium (small team, commercial backing) |
| WASM artifact | `automerge_wasm` raw module | `y-crdt`/yrs Wasm; pure-JS Yjs needs no Wasm | First-class Wasm (`loro-wasm`) |
| Production-shipping today | PushPin, Trellis, Pixelpusher, several startups | Evernote (RTE since 2023), Affine, JupyterLab RTC, Soulpage, Relay-for-Obsidian, hundreds more | Pre-1.0 production use rare; growing demos |

## 2. Algorithm comparison

See `crdt-theory.md` for the taxonomy. Quick reference:

| Property | Automerge | Yjs | Loro |
|---|---|---|---|
| List/text base | RGA-style (columnar history) | YATA (left/right origin) | Fugue (left/right origin, max-non-interleaving) |
| Interleaving avoidance | Susceptible in adversarial concurrent inserts | YATA reduces, does not eliminate | FugueMax: maximally non-interleaving (provably) |
| Tombstones | Kept; columnar compression amortizes | Merged when adjacent; tombstones survive logically | Kept, but eg-walker-style replay reduces steady-state memory |
| Tree move | Limited (no full-fidelity move-of-subtree CRDT) | No native move | Full move-op per Kleppmann 2021, plus fractional-index sibling order |
| Document-size growth (5K-char doc, heavy edit) | Larger pre-3.0; 3.0 cuts ~10x | Smallest historical footprint when GC enabled | Comparable to Yjs; smaller per-op metadata claimed |

## 3. Performance (published benchmarks)

The canonical CRDT benchmark suite is `dmonad/crdt-benchmarks` (Jahns) and `josephg/editing-traces` (Gentle). Loro publishes its own at `loro.dev/docs/performance`.

**Loro's published claims (loro.dev/docs/performance):** Loro reports being faster than Yjs and Automerge across the `automerge-paper` trace (260K ops from a real conference-paper edit history). Loro reports smaller encoded size and faster load.

**Methodology caveat — Jahns's rebuttal:** Kevin Jahns has publicly disputed Loro's benchmarks on the Yjs forum:

> "I have a bit of a problem with the Loro CRDT as their benchmarks are not reproducible. They don't even publish the source code for the benchmarks."
> "The size of the Loro bundle is over 1MB in size, which needs to be base64 encoded if you ship it to the browser (+30% overhead)."
> Disabling Yjs garbage collection in comparisons is "unfair, and misleading to the user" since GC is "an integral feature of Yjs and the reason why it performs well in practice."

Source: https://discuss.yjs.dev/t/yjs-vs-loro-new-crdt-lib/2567

**Honest read:** Loro is fast. Whether Loro is *faster than Yjs* depends on which trace, whether Yjs GC is on, whether bundle-size is in scope, and which version of Loro (the project shipped a backwards-compat encoding in the run-up to 1.0 that traded throughput for stability). For Myrhiza's purposes (server-side Rust, Wasm Component, deterministic replay), Loro and `automerge`-Rust are in the same league; Yjs runs in a different language.

## 4. Document size growth

For the `automerge-paper` trace (~260K ops, ~104K final characters):

| Lib | Encoded doc size | In-memory state | Notes |
|---|---|---|---|
| Automerge 2.x | ~120 KB | ~GBs in worst case | Pre-3.0 history was the bloat source |
| Automerge 3.0 | ~30% overhead, "less than 1 byte per character" | 10x reduction over 2.x | Per Automerge 3.0 blog post |
| Yjs (GC on) | ~159 KB encoded; 19.7 MB peak parse | "10,971 Item objects" for 260K insertions | Per Jahns's blog `are-crdts-suitable-for-shared-editing` |
| Loro (1.x) | Comparable to Yjs claim per Loro docs | Comparable | Disputed; methodology contested by Jahns |

**Cross-cut:** All three converge on roughly equivalent steady-state sizes for *typical* edit traces. The differences appear at the tails: huge histories, adversarial right-to-left typing, deep concurrent fork/merge.

## 5. Sync protocol

| Lib | Mechanism | Round trips per sync | Bytes on wire |
|---|---|---|---|
| Automerge | Bloom filter over commit-graph hashes | 2+ (false positives may force more) | ~10 bits/entry filter + missing-changes payload. Source: `rust/automerge/src/sync/bloom.rs` (10 bits, 7 probes) |
| Yjs | State vector → diff encode | 2 (SyncStep1 / SyncStep2 / Update) | varInt-encoded state vector + delta. Source: `y-protocols/PROTOCOL.md` |
| Loro | Version vector → `export({mode:"update", from: vv})` | 2 | VV-keyed update chunk. Source: `loro.dev/docs/tutorial/sync` |

**For Myrhiza:** version-vector-based protocols (Yjs, Loro) are simpler to reason about and have predictable byte-cost. Bloom-filter (Automerge) tolerates dropped peers better but has false-positive amplification. Pick by topology: gossip with offline peers → bloom; hub-and-spoke → state vector.

## 6. WASM compilability

| Target | Automerge | Yjs (yrs) | Loro |
|---|---|---|---|
| `wasm32-unknown-unknown` | Yes (`automerge_wasm` ships) | Yes (yrs ships `y-crdt`) | Yes (first-class) |
| `wasm32-wasi` | Yes via Rust toolchain (untested in CI) | Same caveat | Yes |
| WASM Component Model (`.wasm` with WIT) | **No.** Ships raw module + bindgen | **No.** Same | **No.** Same |

Verified by inspecting `Cargo.toml [lib] crate-type` in each repo: all three are `cdylib` + `rlib`. None ship a `.wit` interface.

**Implication for Myrhiza:** all three require Myrhiza to wrap them with WIT and produce a Component Model artifact. None drop in. Loro and Automerge-Rust are easier to wrap because the host crate is already Rust; Yjs requires going through `yrs`.

## 7. Maturity gradient

| Axis | Yjs | Automerge | Loro |
|---|---|---|---|
| Years in production | 10+ | ~7 | ~3 |
| API stability | Stable since ~v13 (2020) | Major rewrites: 0.x → 1.x → 2.0 (2023) → 3.0 (late 2025) | Stable claim from 1.0 (2024); pre-1.0 churn was high |
| Wire-format stability | Long-stable; v2 format additive | Reformatted at 2.0 and again at 3.0 | "1.0 lock-in" promised; encoding intentionally extensible |
| Editor binding ecosystem | Dominant (see §8) | Smaller | Smallest |
| Spec/docs of binary format | Informal, in `INTERNALS.md` | Yes: `automerge-binary-format-spec` | Yes: `loro.dev/docs` |

"Mature" in this context = wire format stable across versions so peers running different lib versions interoperate. **Yjs leads here.** Automerge has broken wire format twice. Loro is too young to know.

## 8. Editor binding ecosystem

| Editor | Yjs | Automerge | Loro |
|---|---|---|---|
| ProseMirror | y-prosemirror (canonical) | community port | community port |
| TipTap | y-tiptap (first-class) | none | none |
| CodeMirror 6 | y-codemirror.next (canonical) | none | none |
| Monaco | y-monaco | none | none |
| Slate | slate-yjs | none | none |
| Quill | y-quill | none | none |
| Lexical | LexicalCollab uses Yjs | none | none |

**Yjs dominates.** Almost every collaborative-editor demo on the web uses Yjs. Automerge has Peritext-style rich-text but limited editor adapters. Loro is building its own; few third-party adapters.

## 9. Determinism reality

For Myrhiza `state-apply`: given identical sequences of operations, does library X produce *byte-identical* document state across replicas?

- **Logically deterministic:** all three. Convergence is the defining CRDT property; given the same set of ops, all converge to the same logical state.
- **Byte-identical state:** **not guaranteed by any of the three.** None of the libraries promise identical *internal representation* across runs — only identical observable document state. Internal struct ordering, GC timings, and columnar block boundaries can differ.
- **Byte-identical encoded export:** depends on insertion order. Automerge 3.0 columnar export is order-sensitive; Yjs `encodeStateAsUpdate` is order-sensitive; Loro `export({mode:"snapshot"})` is order-sensitive.

**Consequence for Myrhiza:** if `state-apply` needs byte-identical state hashes across peers (e.g. for proof-of-state or content-addressing), wrapping a CRDT lib is insufficient. Either (a) use the CRDT for *logical* state and hash a canonicalized projection, or (b) implement state-apply over a pure deterministic data model and use the CRDT only for op ordering. (b) is closer to eg-walker's stance.

Verification source: Automerge sync docs, Yjs `INTERNALS.md`, Loro `version_deep_dive`. None of the three claim byte-identical internal representation.

## 10. Recommendation

| If Myrhiza wants... | Choose | Why |
|---|---|---|
| Rust + WASM + Fugue (modern text algo) + tree move | Loro | Only lib with all four |
| 7+ years of production hardening, multi-org governance | Automerge | Ink & Switch + community |
| Largest editor-binding ecosystem (UI surface) | Yjs (or yrs in Rust contexts) | 10+ years, dominant |
| Smallest sync delta for known peer set | Yjs / Loro (state-vector / version-vector) | Bloom filter only wins with unknown peers |
| Sync across unknown / churning peer set | Automerge (bloom filter) | Tolerates membership drift |
| Movable tree (concurrent move of subtree) with proof | Loro | Per Kleppmann 2021 |
| Stable wire format across versions | Yjs | Others have rewritten twice |
| Determinism for `state-apply` in WASM Component | None drop in | All require canonicalization layer |

For Myrhiza's `state-apply`: **none of the three is a complete solution.** A CRDT lib gives you convergence; Myrhiza needs convergence *plus* authority validation *plus* deterministic byte-state. See `open-problems.md` for the gap list.

## Sources

- Automerge sync: https://automerge.org/automerge/automerge/sync/index.html
- Automerge bloom filter source: https://automerge.org/automerge/src/automerge/sync/bloom.rs.html
- Automerge 3.0 release: https://automerge.org/blog/automerge-3/
- Automerge binary format spec: https://automerge.org/automerge-binary-format-spec/
- Yjs `INTERNALS.md`: https://github.com/yjs/yjs/blob/main/INTERNALS.md
- Yjs sync protocol: https://github.com/yjs/y-protocols/blob/master/PROTOCOL.md
- Yjs document updates: https://docs.yjs.dev/api/document-updates
- Jahns response on CRDTs for shared editing: https://blog.kevinjahns.de/are-crdts-suitable-for-shared-editing
- Loro performance docs: https://loro.dev/docs/performance
- Loro sync docs: https://www.loro.dev/docs/tutorial/sync
- Loro version-vector deep dive: https://loro.dev/docs/advanced/version_deep_dive
- Loro vs Yjs forum thread (Jahns's rebuttal): https://discuss.yjs.dev/t/yjs-vs-loro-new-crdt-lib/2567
- Loro tree move blog: https://loro.dev/blog/movable-tree
- Loro rich-text Fugue blog: https://loro.dev/blog/loro-richtext
- Fugue paper (Weidner & Kleppmann): https://arxiv.org/abs/2305.00583
- Eg-walker paper: https://arxiv.org/abs/2409.14252
- Tree-move CRDT paper (Kleppmann et al. 2021): https://martin.kleppmann.com/papers/move-op.pdf
- Kleppmann bloom-filter sync: https://martin.kleppmann.com/2020/12/02/bloom-filter-hash-graph-sync.html
- Evernote case study: https://evernote.com/blog/future-proofing-evernotes-foundations
- `dmonad/crdt-benchmarks`: https://github.com/dmonad/crdt-benchmarks
- `josephg/editing-traces`: https://github.com/josephg/crdt-benchmarks
