**Date:** 2026-05-09
**Status:** active
**Subject:** Master-spec review round 1 — synthesis

# Round 1 review of myrhiza-master-design

Five independent reviewers given different lenses (architect, security, determinism, implementation feasibility, skeptic). Each reviewed the master spec at commit `6f3c90a` (1,391 lines) without seeing the writing process.

## Verdicts

| Reviewer | Verdict |
|---|---|
| Architect | fix-then-ship |
| Security | fix-then-ship |
| Determinism | fix-then-ship |
| Implementation feasibility | fix-then-ship with scope-reduction-needed |
| Skeptic | fix-then-ship |

Strong consensus. No reviewer requested rework or fundamental redesign.

## Top-tier load-bearing issues (3+ reviewers)

1. **Manifest schema + capability vocabulary deferred but blocks v1.** §7.2 intersection (`A_ambient ∩ M_required`) cannot be specified without vocabulary. Fix: promote manifest schema + capability vocabulary + high-value-op list from "child spec" to v1 master-spec annex.

2. **`state-digest` format deferred but blocks convergence proof.** Acceptance criterion #2 (multi-peer convergence) requires committed format. Fix: pin `bincode 1.3.x` with default config at master-spec level.

3. **Wasmtime version pin missing.** Cross-peer fuel determinism requires Wasmtime version pin. Fix: commit Wasmtime LTS major (target v48 or earliest LTS-eligible at v1 ship) at master-spec level.

4. **Per-call gating list deferred but mechanism is load-bearing security.** Six candidates named in §7.3 ("examples") — promote to v1-mandatory; child spec is additive only.

5. **Pre-check claim overstated.** "Structurally impossible" divergence (§4.4 line 288) is true only for identical prior-state inputs. Cross-peer rejection from differing prior-states is normal eventual consistency. Fix: weaken claim with explicit prior-state caveat.

6. **Schedule realism.** ~16-20 weeks dual-stack is optimistic 30-50%. Realistic 24-32 weeks. Fix options: accept range OR reduce v1 scope (defer jco to v1.5; cut behavior to v1.1; cut per-call gating to v1.1).

## Mid-tier improvements (2 reviewers)

7. **`host.hash` algorithm pin missing.** BLAKE3 named "provisional" but state-digest depends on it. Fix: pin BLAKE3 in §5.1.

8. **`host.sign-via-scope(scope, msg: list<u8>)` too broad.** Compromised behavior can sign fake non-event payloads. Fix: replace with `host.author-event(scope, event-payload)` where kernel enforces structural validity.

9. **Module supply-chain attacks.** Module deps by name+version not content hash; no signing root distinction; no module-update user confirmation. Fix: module deps use content hashes; per-update confirmation; "myrhiza-official" signing root with HRP convention.

10. **Helper-set divergence between §3 / §5 / §6 / §9.** `host.verify-signature` vs `host.verify-sig` — same op different names; `state-propose` not granted `host.sign-via-scope`. Fix: add §3.5 normative table — rows are host imports, columns are profiles.

11. **Topic identity formula incomplete.** §11.2 cites `topic-app-state` undefined; topic creation/lifecycle not specified. Fix: add §4.x on topic identity formula + lifecycle.

## Lower-tier polish (single reviewer)

12. **Cremers enforcement structural** (security) — kernel must reject non-Ed25519 structurally; not advisory lint
13. **HLC role in state-apply** (determinism) — clarify HLC IS available via `host.now-hlc-from-event`
14. **PendingBuffer eviction local-only** (determinism) — call out; doesn't affect convergence
15. **Bundle revocation direction** (security) — pick a shape even if mechanics deferred
16. **Author equivocation** (determinism) — name resolution (first-seen-wins, flagged in derived state)
17. **`identity-scope` should be `resource` not `record`** (security)
18. **AEAD nonce reuse** (security) — kernel-managed nonces, not app-chosen
19. **§14.5 ~2-5% overhead figure** (security, skeptic) — steady-state; cold-instantiation higher
20. **§15.5 typo `coexistence.cs`** (architect) — should be `.rs`
21. **Migration story §16 too brief** (architect)
22. **Implementation outline order** (implementation) — manifest before Wasmtime backend
23. **Capability summary fatigue at install** (skeptic) — MetaMask Snaps pattern; mitigate
24. **Browser kernel as JS shim engineering project** (implementation) — own scope
25. **Resource-handle persistence + Agoric `baggage`** (architect, skeptic) — component upgrade

## Prior art the spec doesn't fully absorb (skeptic)

- **Holochain Borrow** — source chain (already aligned with per-author DAG), DHT op decomposition (informs v2 sharding), warrants (bad-author signal), countersigning (multi-author events) — none currently in spec
- **Croquet TUTTI** snapshot-equality voting — acknowledged in brainstorming, dropped from spec
- **Agoric `baggage`** upgrade convention — not absorbed
- **Agoric `bringOutYourDead`** distributed GC — not absorbed
- **Willow `timestamp_hint_ms` split-semantics review-trap** — inherited verbatim

## Strengths consensus (all 5 reviewers)

- Three-tier architecture cleanly stated
- Pre-check unification structural correctness (modulo issue #5)
- IdentityScope primitive unification good
- Capability gating four-layer model
- Tradeoff matrix §18 with named runners-up
- Future-direction list §17 honest
- WASM-on-every-backend principled
- Decision rationale visibility unusually traceable

## Recommended fix strategy

**Comprehensive fix pass** on master spec — address top + mid + lower-tier inline; absorb missing prior-art via additive sections. New sections needed:

- §3.5 Normative host import table
- §4.x Topic identity formula + lifecycle
- §10A Manifest schema annex (capability vocabulary, high-value-op list, supply-chain rules)
- §10.7+ Bundle revocation direction
- §17 expanded with Holochain Borrow §1-4, Croquet TUTTI, Agoric `baggage`/`bringOutYourDead` items
- §22 Observability + drift detection (TUTTI-shaped runtime convergence verification)

After comprehensive fix, dispatch round-2 reviewers (fresh agents). Iterate until no major issues found.

## Files in this folder

- `README.md` (this file) — synthesis
- `architect.md` — full architect review
- `security.md` — full security review
- `determinism.md` — full determinism review
- `feasibility.md` — full implementation feasibility review
- `skeptic.md` — full skeptical review

## Sources

- Master spec under review: `docs/specs/2026-05-09-myrhiza-master-design.md` at commit `6f3c90a`
- Brainstorming decision log: `docs/reports/2026-05-09-myrhiza-design-space/brainstorming-decisions.md`
- Prior-art corpus: `docs/prior-art/{12 folders}/`
