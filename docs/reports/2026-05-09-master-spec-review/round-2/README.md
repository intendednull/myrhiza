**Date:** 2026-05-09
**Status:** active
**Subject:** Master-spec review round 2 — synthesis

# Round 2 review of myrhiza-master-design (post round-1 fixes)

Five independent reviewers given fresh-eyes lenses. Spec at commit
`c34657e` (2,327 lines).

## Verdicts

| Reviewer | Verdict |
|---|---|
| Architect | fix-then-ship |
| Security | fix-then-ship |
| Determinism | fix-then-ship |
| Implementation feasibility | fix-then-ship |
| Skeptic | fix-then-ship |

All converge on fix-then-ship. Round-1 fixes mostly landed substantively
(22-25 of 25 round-1 items fully landed; 3 partially). Round-2 found
new issues, mostly arising from round-1 fix propagation gaps + previously-
unsurfaced details.

## Critical issues to fix in round-3

### Cross-section consistency (multiple reviewers)
1. **AEAD nonce model contradiction**: §3.5 has `nonce-handle`; §9.2 has raw `nonce: list<u8>`. Pick one (kernel-managed handle is correct).
2. **`host.verify-signature` (§3.5/§5.1) vs `host.verify-sig` (§9.2)** — same op, different names.
3. **Topic-identity formula not propagated**: §4.6 has formula, §11.2 still has old undefined `topic-app-state` wording.
4. **`host.broadcast` (§3.5) vs `host.network.broadcast-submit` (§8.5)** — pick one.
5. **`host.create-private-channel` (§7.4) undefined** — mark as pseudocode or remove example.
6. **`IdentityScope.long_term` (§16) vs `long-term`** — kebab-case slip.
7. **Per-call high-value-ops list misalignment**: §3.5 four entries; §7.3 six; §10.2 five — align.

### Substantive gaps
8. **Acceptance criterion #6 ambiguity** (§15.1 v1 / §15.5 v1.1 / §15.2 "optional" — pick one).
9. **§4.3 still says format "open and deferred"** — contradicts §5.4 firm pin.
10. **§4.7 drift-detection anchor selection undefined** — cannot tell drift from sync-lag.
11. **§4.7 drift detection cost claim wrong** — `state-digest()` walks full app state, not "one BLAKE3 hash."
12. **§4.7 manifest field referenced but absent from §10.2 schema**.
13. **§4.4.1 equivocation × convergence claim**: equivocating authors break acceptance criterion #2 unless convergence guarantee scoped explicitly.
14. **bincode 1.3.x pin imprecise**: bincode 1.3 has multiple Options builder paths producing different bytes. Pin the exact `Options` chain + serde version.
15. **ABI-versioning semantics**: spec treats deterministic-helper-set additions as kernel minor; this is convergence-breaking and should be major.
16. **Cherry-picked precedents only acknowledged in §19** — should be in §1 vision section.
17. **`host.user-prompt` rendering conflict**: §10.5 says kernel-controlled UI surface; §13.2 says UI app is in TCB for chrome. Reconcile.
18. **Capability vocabulary `ui:*` set** referenced but deferred to child spec — block manifest validator engineering.
19. **Manifest TOML correctness**: quoted dotted keys requirement undocumented; `[[modules.dep]]` array canonicalization for signature undocumented.
20. **Author-event payload-variant authorization**: structural validity ≠ authorization to author specific variant.
21. **Behavior identity revocation**: compromised behavior keypair has no revocation path.
22. **Kernel binary distribution + auth**: §10.9 hard-codes allowlist but doesn't specify how user verifies kernel binary is authentic. Bootstrap of trust unaddressed.
23. **Genesis event semantics + topic-id-in-invitation**: spec references genesis event without defining; out-of-band invitations should carry topic_id.
24. **`host.http.request` origin pattern format** unspecified.

### Polish
25. **§16 typo `IdentityScope.long_term` → `long-term`**
26. **Clipboard READ + sensor APIs** not in §3.5 denied list
27. **Holochain Borrow §5** "validation as pure WASM callback" should be in already-aligned (since state-apply IS this)
28. **`host.kv` decomposition** (get/put/delete/list-prefix)
29. **`on-completion` WIT shape** illustrative-only
30. **Snapshot lifecycle**: v1 specifies "no snapshots, full replay" or commits a snapshot model
31. **`host.install-key` kernel-side bookkeeping spec** missing
32. **`host.now-hlc-from-event` canonical event-envelope encoding** required

## Defer to v3 or child specs (named in round-2)

- **Wasmtime LTS kernel-version-skew handling** — accepted risk, document in §19
- **Revocation topic DDoS** — sequence number revocations + rate-limit
- **Module update UX** — per-module-update consent vs per-app-update
- **Side-channel scope** — already in §19, small tightening
- **Pre-check fuel rate limits** — name as accepted risk in §19
- **Resource-handle cleanup discipline** — paragraph in §19
- **Performance figures committed-to-measure** — phrasing tighten
- **Critical path item count discrepancy** §15.5 vs §20 — note alignment

## Strengths consensus (post round-1 fixes)

- §3.5 normative table is the most useful round-1 addition
- §4.4 deps-monotonicity caveat is the cleanest round-1 fix
- §4.6 topic identity formula concrete
- §10.2 manifest schema concrete
- §14.5 honest overhead bands
- §15.5 24-32 week range with v1.5 fallback
- §17 prior-art absorbed (named, not implemented)
- §6.1 structural Cremers ETK 2025 enforcement
- Round-1 fixes mostly landed substantively, not cosmetically

## Files in this folder

- `README.md` (this file) — synthesis
- per-reviewer review content preserved in agent transcripts
