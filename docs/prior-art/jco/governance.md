**Date:** 2026-05-22
**Status:** active
**Subject:** jco governance — maintainers, bus factor, funding, release cadence.

## 1. Project structure

jco lives under the **Bytecode Alliance** (BA) GitHub organization, a 501(c)(6) industry consortium founded 2019 to standardize WebAssembly outside the browser. BA hosts (among other things) Wasmtime, the Component Model spec, WASI, and the toolchain projects including jco, ComponentizeJS, StarlingMonkey, and cargo-component.

License: Apache-2.0 WITH LLVM-exception (verified per LICENSE files in both jco and ComponentizeJS repos). The LLVM-exception is BA standard — same as Wasmtime, the component-model spec, and the WASI standard.

## 2. The current maintainer set

**Codeowners** (per `.github/CODEOWNERS` on jco main, 2026-05):

- **`@vados-cosmonic`** — Victor Adossi. Cosmonic-employed (now Akamai-stewarded post-2025-12). Dominant recent committer; observation of `/commits/main` in May 2026 shows the large majority of recent commits are his (approximate; verify on the live commit graph before quoting in a spec). The single highest-bus-factor identity in this codebase.
- **`@andreiltd`** — Andrei Stanciu. Active on PR review and on preview3-shim work.

**npm maintainers** (a slightly different set — historical):

- `tschneidereit` — Till Schneidereit, CEO of Cosmonic (now Akamai-stewarded). Has push access; not active in recent commits.
- `cfallin` — Chris Fallin, principal engineer at Cosmonic. Wasmtime + Cranelift, historical reviewer.
- `guybedford` — Guy Bedford. **Original author** (first npm publish 2023-02-17). Historical Fastly affiliation; primary author of the transpile pipeline architecture. No recent commits to jco main as of 2026-05.
- `vados` — Same Victor Adossi as `@vados-cosmonic` above.

## 3. Bus factor: honest assessment

**The bus factor is low.** Concretely:

- One active person (vados-cosmonic) handling ~90% of recent main-branch commits.
- One backup (andreiltd) doing PR review + targeted feature work.
- Original author (Guy Bedford) is the architecture authority but is not in the recent committer set; if vados-cosmonic stepped away, Guy Bedford's involvement would need to re-engage.
- Cosmonic was acquired by Akamai (2025-12-01, per [`prior-art/spin/`](../spin/) verification). The current jco maintainer pair are both Cosmonic-employed; their employment continuity depends on Akamai's strategic interest in jco continuing. Akamai's stated commitment is to Spin and wasmCloud; jco's position is adjacent, not central, to that commitment.

For a Myrhiza-shaped dependency that is **load-bearing** (browser peer profile), a low bus factor on the dependency is a real risk to flag. The Bytecode Alliance steward status partially mitigates (the BA could hand jco to another maintainer if Cosmonic stepped away), but BA-wide engineering capacity for jco-specific work is limited.

**Lesson for Myrhiza:** the Myrhiza-side build pipeline should be capable of producing browser bundles without jco's CLI on the build path, if jco maintenance lapsed. That means: the `js-component-bindgen` Rust crate (the actual binding-emitter; the part of jco that does real work) should be a Myrhiza-build-time direct dependency, and the jco CLI should be a *convenience layer* on top, not a critical path. If jco-the-CLI got abandoned, Myrhiza could keep using `js-component-bindgen` directly from its workspace. This is a Myrhiza-spec design choice.

## 4. Release cadence

| Package | Cadence | Recent example |
|---|---|---|
| `@bytecodealliance/jco` (CLI) | ~biweekly to monthly minor releases; patches when needed | 1.17.9 (2026-04-17) → 1.18.0 (2026-04-18) → 1.18.1 (2026-04-20) → 1.19.0 (2026-04-22) |
| `@bytecodealliance/componentize-js` | Slower; major release every 1–3 months | 0.19.3 (2025-10-27) → 0.20.0 (2026-04-14) → 0.21.0 (2026-05-20) |
| `@bytecodealliance/preview2-shim` | Tracks jco minor releases | 0.17.9 (2026-04) co-released with jco 1.18-19 |
| `js-component-bindgen` (Rust crate) | Versioned independently in the jco workspace | 1.19.0 (2026-05-18); rc cycle visible: rc.0 (2026-05-13) → rc.7 (2026-05-16) → 1.19.0 (2026-05-18) |
| `StarlingMonkey` | Major bumps tied to SpiderMonkey version bumps; ~quarterly | 0.2.0 (mid-2025) is the version embedded by componentize-js 0.19.0+ |

Release tagging convention in the jco monorepo: `jco-v<version>` for the CLI, `js-component-bindgen-v<version>` for the Rust crate, `jco-std-v<version>` for the experimental std package. The tag namespace is per-package.

There is no published "RELEASE.md" describing the formal process; the cadence is observed from the tag history rather than documented. Flagged as a documentation gap in [`open-problems.md` §5](open-problems.md).

## 5. Decision-making

Decisions on jco architecture and roadmap are made in:

- **PRs on `bytecodealliance/jco`** — code-level decisions; ~1–7 day review SLA per recent observations.
- **BA SIG-Components meetings** — design-level decisions involving CM-spec changes that affect jco.
- **BA Zulip `#jco` and `#component-model` channels** — informal discussion.

There is no formal jco-specific RFC process. Issues + PRs serve that function. For a Myrhiza-shaped dependency, that means: changes to the transpile contract (the shape of emitted bindings, the import surface, etc.) happen *in PRs* and propagate to releases without a separable RFC trail. To track contract drift, Myrhiza spec authors should watch PR titles on the jco repo and pin to specific jco minor versions in the build pipeline.

## 6. Funding context

- **Cosmonic** (now Akamai-stewarded post-2025-12): primary jco maintainer employment.
- **Bytecode Alliance** itself: 501(c)(6) consortium; does not employ engineers directly. Member companies (Fastly, Microsoft, Mozilla, Intel, Akamai/Cosmonic, Fermyon/Akamai, et al.) employ the engineers.
- **Fastly's historical involvement** (via Guy Bedford's original Fastly tenure + StarlingMonkey's Fastly production use): real, ongoing, less visible in commits than in StarlingMonkey-side work.

There is no jco-direct funding mechanism (no GitHub Sponsors, no Open Collective on the jco repo). Funding flows through the BA member companies. This is the standard BA pattern; it works for Wasmtime, which has 5+ companies invested. It works *thinner* for jco, which has 1–2 companies materially invested.

## Sources

- jco `.github/CODEOWNERS`: <https://github.com/bytecodealliance/jco/blob/main/.github/CODEOWNERS>
- jco recent commits: <https://github.com/bytecodealliance/jco/commits/main>
- npm `@bytecodealliance/jco` maintainers field
- BA org: <https://bytecodealliance.org/>
- BA Zulip: <https://bytecodealliance.zulipchat.com>
- Cosmonic/Akamai acquisition context: [`prior-art/spin/`](../spin/), [`prior-art/wasmcloud/`](../wasmcloud/)
- Myrhiza cross-refs: [`prior-art/wasm-component-model/governance.md`](../wasm-component-model/governance.md)
