**Date:** 2026-05-22
**Status:** active
**Subject:** Schema evolution in distributed and local-first systems — Cambria (Ink & Switch bidirectional lenses, research-grade, stalled), the "Live & Local Schema Change" challenge problems, and the classical wire-format evolution disciplines (Protobuf, Cap'n Proto, Avro, Postcard). Load-bearing reference for Myrhiza's `state-apply` snapshot-portability problem.

## What this folder is

A small survey folder. Two clusters:

- **Cluster A — research-grade ambition:** Ink & Switch's *Project Cambria* (essay 2020-10, PaPoC paper 2021) and the Edwards/Petricek/van der Storm *Live & Local Schema Change* challenge problems (LIVE@SPLASH 2023). Asks the maximalist question — can a single bidirectional artifact translate data between heterogeneous schemas on peers running different versions, in both directions, at the same time? Cambria's answer is "yes, in principle, via lenses." Production answer is "nobody has shipped it."
- **Cluster B — production discipline:** Protobuf, Cap'n Proto, Avro, Postcard. Asks the minimalist question — given a fixed schema language, what changes are safe to apply between versions such that old + new code can read each other's bytes? The four formats give four different answers; all four work in production.

Myrhiza needs both clusters in scope. `state-apply` component upgrades are the canonical snapshot-portability problem ([`willow/open-problems.md §Snapshot portability`](../willow/open-problems.md)). Cambria is the canonical-but-stalled CRDT schema-evolution attempt ([`crdts/open-problems.md:13`](../crdts/open-problems.md)). The classical formats are how the rest of distributed-systems engineering does it today.

## Key facts

| Subject | Status as of 2026-05-22 | Source |
|---|---|---|
| **Cambria essay** | "Project Cambria — Translate your data with lenses." Geoffrey Litt, Peter van Hardenberg, Orion Henry. Ink & Switch, October 2020. Essay, not peer-reviewed. | [inkandswitch.com/cambria/](https://www.inkandswitch.com/cambria/) |
| **Cambria PaPoC paper** | "Cambria: Schema Evolution in Distributed Systems with Edit Lenses." Litt, Hardenberg, Henry. PaPoC@EuroSys 2021. DOI 10.1145/3447865.3457963, pages 8:1-8:9. | [dl.acm.org/doi/10.1145/3447865.3457963](https://dl.acm.org/doi/10.1145/3447865.3457963) |
| **`inkandswitch/cambria-project`** | TypeScript, MIT, 132 commits, 691 stars, not archived. README: *"Cambria is still immature software, and isn't yet ready for production use."* | [github.com/inkandswitch/cambria-project](https://github.com/inkandswitch/cambria-project) |
| **Geoffrey Litt** | Currently at Notion. Previously senior researcher at Ink & Switch. PhD HCI at MIT under Daniel Jackson. | [geoffreylitt.com](https://www.geoffreylitt.com/) |
| **Live & Local Schema Change** | "Live & Local Schema Change: Challenge Problems." Jonathan Edwards, Tomas Petricek, Tijs van der Storm. LIVE Programming Workshop at SPLASH 2023 (2023-10-24). arXiv 2309.11406. | [arxiv.org/abs/2309.11406](https://arxiv.org/abs/2309.11406) |
| **Foster et al. lens combinators** | Foster, Greenwald, Moore, Pierce, Schmitt. *Combinators for bidirectional tree transformations: A linguistic approach to the view-update problem.* TOPLAS 29(3):17, May 2007. (Earlier: PLAN-X 2004; POPL 2005 extended abstract.) | [cis.upenn.edu/~bcpierce/papers/](https://www.cis.upenn.edu/~bcpierce/papers/index.shtml) |
| **Edit Lenses** | Hofmann, Pierce, Wagner. *Edit Lenses.* POPL 2012. Cited by Cambria as the direct inspiration for "operate on patches, not documents." | Cambria essay, "Related Work" |
| **Protobuf evolution** | Field-number-based; explicit safe-change list. Field numbers must never be reused; deletion requires `reserved`. | [protobuf.dev/programming-guides/proto3/#updating](https://protobuf.dev/programming-guides/proto3/#updating) |
| **Cap'n Proto evolution** | Type-ID + ordinal-number based; explicit safe-change list. New members must have larger numbers than previous. Renames are safe; renumbering is not. | [capnproto.org/language.html#evolving-your-protocol](https://capnproto.org/language.html#evolving-your-protocol) |
| **Avro evolution** | Writer/reader schema resolution at decode time. Named-type aliases, default values for missing fields, int→long/float/double promotion. | [avro.apache.org/docs/](https://avro.apache.org/docs/) |
| **Confluent Schema Registry** | Compatibility modes: NONE / BACKWARD (default) / FORWARD / FULL, plus TRANSITIVE variants checking against full version history. | [docs.confluent.io/.../schema-evolution.html](https://docs.confluent.io/platform/current/schema-registry/fundamentals/schema-evolution.html) |
| **Postcard 1.1.3** | `#![no_std]` Serde serializer. Apache-2.0 / MIT. Wire format stable since v1.0.0. *Not* self-describing. **No built-in schema versioning**; explicitly out of scope. | [github.com/jamesmunns/postcard](https://github.com/jamesmunns/postcard), [postcard.jamesmunns.com](https://postcard.jamesmunns.com/wire-format) |

## Canonical reading order

1. [`cambria.md`](cambria.md) — the bidirectional-lens approach in detail; why it stalled; what it teaches.
2. [`live-and-local.md`](live-and-local.md) — the 2023 challenge-problems framing paper that re-poses the question after Cambria's stall.
3. [`traditional.md`](traditional.md) — Protobuf / Cap'n Proto / Avro / Postcard rules side-by-side.
4. [`migration-strategies.md`](migration-strategies.md) — the three strategy families Myrhiza must choose between: re-replay-from-genesis vs explicit migration vs version-and-refuse.
5. [`open-problems.md`](open-problems.md) — what schema evolution structurally doesn't solve. The semantic-vs-structural distinction.
6. [`lessons.md`](lessons.md) — validates / avoid / borrow synthesis for Myrhiza.

Skim [`glossary.md`](glossary.md) when terms get crossed (lens vs schema, structural vs semantic, writer vs reader schema).

## Cross-links to existing corpus

- [`crdts/open-problems.md §1 Schema evolution`](../crdts/open-problems.md) — the call-out that drove this folder ("Cambria — the canonical attempt at heterogeneous-schema CRDT convergence — bidirectional lenses, never reached production").
- [`crdts/open-problems.md §5 Schema migration of on-disk bytes`](../crdts/open-problems.md) — the library-version-bump variant. Distinct problem; same family.
- [`willow/open-problems.md §Snapshot portability across component upgrades`](../willow/open-problems.md) — the Myrhiza-facing version of the problem.
- [`capn-proto/capnp.md §Schema`](../capn-proto/capnp.md) — Cap'n Proto's evolution discipline in context of its broader RPC + ocap story. This folder cross-links the schema-evolution slice specifically.
- [`agoric-endo/persistence.md`](../agoric-endo/persistence.md) — vat-snapshot story, the production-scale comparator for "how do you upgrade state in flight." Different shape (single-authority vat) but same load-bearing question.
- [`agoric-endo/lessons.md §What is our baggage analog?`](../agoric-endo/lessons.md) — frames the same question Myrhiza will face.

## How to use

When a Myrhiza spec author has to make a decision about `state-apply` versioning, snapshot format, or cross-version event compatibility, this folder is the consult-this reference. Open `lessons.md` first; drop into `cambria.md` or `traditional.md` for evidence; consult `migration-strategies.md` for the three-way design choice.

**Framing disclosure.** These docs are written from a Component-Model-as-foundation, deterministic-`state-apply`, event-log-as-source-of-truth stance — most "Implications for Myrhiza" sub-sections frame the schema-evolution choices through that lens. The corpus has a structural reason to under-rate Cambria (because Cambria stalled and we are not going to ship lenses in Myrhiza v1) and to over-rate the classical formats (because they are what we will actually use). Future readers auditing whether the deterministic-`state-apply` commitment is itself the right primitive — for example, considering a CRDT-everywhere alternative where lensing would matter more — should weigh the corpus accordingly: it's a learn-from-everyone-into-deterministic-state-apply artifact, not a neutral catalog. The Cambria essay was load-bearing for local-first ambition circa 2020-2021; this corpus reads it through 2026-05 eyes after five years of stall. That bias is the bias.

## Sources

- Project Cambria essay: https://www.inkandswitch.com/cambria/
- Cambria PaPoC paper: https://dl.acm.org/doi/10.1145/3447865.3457963
- `cambria-project` repo: https://github.com/inkandswitch/cambria-project
- Geoffrey Litt's site: https://www.geoffreylitt.com/
- Live & Local Schema Change: https://arxiv.org/abs/2309.11406
- Foster et al. lens combinators (TOPLAS 2007): https://www.cis.upenn.edu/~bcpierce/papers/index.shtml
- Protobuf evolution rules: https://protobuf.dev/programming-guides/proto3/#updating
- Cap'n Proto evolution rules: https://capnproto.org/language.html#evolving-your-protocol
- Avro 1.11.1 specification: https://avro.apache.org/docs/1.11.1/specification/
- Confluent Schema Registry compatibility modes: https://docs.confluent.io/platform/current/schema-registry/fundamentals/schema-evolution.html
- Postcard repository: https://github.com/jamesmunns/postcard
- Postcard wire-format specification: https://postcard.jamesmunns.com/wire-format
- Cross-link: [`docs/prior-art/crdts/open-problems.md`](../crdts/open-problems.md)
- Cross-link: [`docs/prior-art/willow/open-problems.md`](../willow/open-problems.md)
- Cross-link: [`docs/prior-art/capn-proto/capnp.md`](../capn-proto/capnp.md)
- Cross-link: [`docs/prior-art/agoric-endo/persistence.md`](../agoric-endo/persistence.md)
