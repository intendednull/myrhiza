**Date:** 2026-05-09
**Status:** active
**Subject:** Master-spec review round 3 — synthesis

# Round 3 review of myrhiza-master-design (post round-2 fixes)

Spec at commit `8d3249e` (2,734 lines).

## Verdicts

| Reviewer | Verdict |
|---|---|
| Architect | minor-polish |
| Security | fix-then-ship |
| Determinism | **ship-as-is** with minor-polish nits |
| Feasibility | fix-then-ship |
| Skeptic | fix-then-ship |

Determinism reviewer says ship-as-is. Other four say fix-then-ship or
minor-polish. Round-2 critical fixes mostly landed (~22/25 fully
landed across reviewers' tallies). Round-3 finds remaining issues are
either propagation gaps or load-bearing details still unspecified.

## Block-ship fixes for round 4

1. **`host.http.request` origin pattern format** unspecified — pin at RFC 6454 exact-origin match at v1
2. **Fuel-cost defaults** not pinned at v1 (convergence-load-bearing)
3. **Threshold-signature scheme** undefined — name FROST-Ed25519 or downgrade to community attestation
4. **Revocation-seq=u64::MAX** can brick revocation channel — cap max-jump
5. **TOML canonical encoder** library not pinned — pin `toml_edit`
6. **§4.2 vs §19 snapshot inconsistency** — remove snapshot-bootstrap from §4.2
7. **§14.2 Wasmtime LTS bump** version-class contradicts §10.2
8. **Per-call gating list mismatch** §3.5/§7.3/§10.2 (AEAD seal/open per-call?)

## Significant fixes

9. Browser kernel UI: cross-origin iframe (not z-index)
10. HeadsSummary protocol normative description
11. Genesis event seed-injection flow definition
12. `[author-policy]` default to deny-by-default
13. Drift-message signing scope
14. `iroh-gossip` not `iroh-blobs` in §10.7
15. `host.kv` granularity in §10.2
16. `host.create-private-channel` mark as illustrative

## Polish

17. Cremers ETK 2025 one-line gloss
18. §15.2 "Optional behavior" → "v1.1 behavior"
19. Native kernel-UI engineering cost in §15.5
20. §17 prior-art reframe ("named for v2+" not "absorbed")

## Persistent honest-deferral items (not block-ship)

- deps-monotonicity not mechanically enforced (§4.4 acknowledges)
- Behavior key revocation absent v1 (§6.3 documents direction)
- Capability summary fatigue mitigations weak long-term (§19 acknowledges)
- v1 commits-to-measure for performance overhead, MLS perf, browser CM
  on Safari iOS (§14.5, §19)

## Strengths consensus (post round-2)

- Three-tier architecture coherent
- Convergence proof three legs airtight at v1 scope (determinism reviewer
  ships-as-is)
- Pre-check unification + deps-monotonicity invariant honest
- Drift detection anchored properly (TUTTI + equivocation interaction)
- bincode 1.3.x explicit Options chain pin
- IdentityScope unified primitive
- §10.10 kernel-binary trust root explicit
- §13.2.1 kernel-controlled UI surface specified (with browser caveat)
- §6.1 [author-policy] for variant authorization
- §10.7 revocation seq monotonicity (with seq=MAX caveat)
- Honest commit-to-measure framing where measurement isn't yet possible
