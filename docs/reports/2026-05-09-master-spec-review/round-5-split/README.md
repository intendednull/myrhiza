**Date:** 2026-05-09
**Status:** active
**Subject:** Master-spec split-consistency review (round 5)

# Round 5: split-consistency review

After 4 review-fix rounds converged the master spec at 2,936 lines
single-file, user requested split into folder. Split landed at commit
`ef52faf` — 20 child files under
`docs/specs/2026-05-09-myrhiza-master-design/`.

## Verdict
**minor-cleanup-needed** (post-review fixes applied; now ship)

## Round-5 findings

1. **Content preservation: 21/21 sections present, all subsections
   intact.** No drift, no duplication, no truncation. Tables and
   code blocks (manifest TOML schema, normative host import table,
   tradeoff matrix, workspace shape) survived split byte-equivalent.

2. **Cross-file refs**: ~80 bare `§N.M` refs to sections in different
   files. Not broken (every target exists), but bypassed README's
   own `[file.md](file.md) §N.M` convention. **Fixed via sweep
   script** — all cross-file refs now linkified; intra-file bare
   refs preserved.

3. **Double-headings**: every child file had `# Subject` + `## N.
   Title` pair. **Fixed via sweep script** — `# Subject` line
   removed; `## N. Title` retained as the canonical section header
   (preserves number-keeping for cross-file refs).

4. **Frontmatter**: uniform across all 19 child files. README
   frontmatter shorter (no Parent / Subject — appropriate as the
   parent itself).

5. **Reading order in README**: sensible, mirrors §3-§21 numerical
   order which is also dependency order.

6. **Internal subsection numbering**: faithfully preserved
   (§4.4.1, §13.2.1 nested sub-subsections survived).

## Strengths

- Section coverage exact and exhaustive
- File-to-section mapping intuitive (filename = handle for content)
- Frontmatter uniform
- Reading order matches dependency order
- Cross-section ref convention documented in README (and now enforced)
- Distribution of file sizes reasonable (largest 487 lines
  distribution.md / 423 convergence.md; smallest 24-27 lines for
  tradeoffs/sources)

## Files in this folder

- `README.md` (this file) — synthesis
- per-reviewer review content preserved in agent transcripts
