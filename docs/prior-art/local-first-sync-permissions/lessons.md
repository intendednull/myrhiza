**Date:** 2026-05-29
**Status:** active
**Subject:** Lessons for Myrhiza — driving the myrhiza-permission-* module ABI and the state-apply authority verdict from shipped local-first authority designs

# Lessons for Myrhiza

The decision file. Other files are evidence; this is what we take away for the
two named decision surfaces: the **`myrhiza-permission-*` module ABI** and the
**`state-apply` authority verdict** ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.4). This is the **product-layer** answer to the **library-layer** gap that
[../crdts/open-problems.md](../crdts/open-problems.md) §2–§3 names: "CRDTs
converge, then violate."

## Validates

Shipped products **confirm** these Myrhiza bets:

- **Per-transaction peer-verified authority is implementable and shipping.**
  cojson's `determineValidTransactions` re-derives, on every peer, whether an
  Ed25519-signed transaction was made by a member whose role-at-the-time
  permitted it ([jazz-cojson.md](jazz-cojson.md)). This is exactly the
  `state-apply` Accept/Reject verdict run identically per peer. Myrhiza is not
  inventing an untried shape — it is adopting cojson's, with the verdict relocated
  into a WASM Component.
- **Authority belongs to the data, evaluated at apply — not to the merge, and
  not to the transport.** Every product here attaches authorization to the write
  and checks it before the write counts. None put it in the CRDT merge function.
  This validates Myrhiza's "validate authority *before* admitting to the
  convergent log" ([../crdts/lessons.md](../crdts/lessons.md) Avoid).
- **Role-at-the-time, not role-now.** cojson evaluates each transaction against
  the group state *as of the transaction's timestamp*. Myrhiza's deps-anchored
  pre-check is the same principle: the verdict is relative to a defined causal
  point, not a peer's "now." Bake "authority is evaluated against deps-closure
  state" into the `myrhiza-permission-*` ABI explicitly.
- **Reads enforced by encryption, not by withholding.** cojson seals the read key
  to authorized members; a non-reader holds ciphertext it cannot decrypt. This
  matches Myrhiza's relay-is-dumb posture — a Myrhiza relay must never be the
  thing deciding who can read. Confidentiality is a crypto property, not a server
  ACL.
- **Self-sovereign keys beat trusted issuers.** PowerSync/Triplit/Zero all anchor
  identity in a JWT from a trusted issuer; cojson and Myrhiza anchor it in a
  keypair. The JWT model is a single point of forgery; the keypair model has none.
  Validates [identity.md](../../specs/2026-05-09-myrhiza-master-design/identity.md).

## Avoid

Specific pitfalls these products expose, with Myrhiza mitigation:

- **Don't let the verdict drift toward a trusted middlebox under DX pressure.**
  Jazz 2.0 is *reintroducing* a trusted-server tier
  ([maturity-and-the-2.0-pivot.md](maturity-and-the-2.0-pivot.md)) because pure
  peer-verification made migration, optimistic-update DX, and global invariants
  hard. *Mitigation*: Myrhiza must answer those three up front —
  pre-check-then-sign for optimistic DX (§4.4), schema-version ABI for migration
  ([../schema-evolution/](../schema-evolution/)), reservation/escrow patterns for
  invariants — or it will feel the same pull.
- **Don't expect role-checks to enforce global invariants.** Per-author role
  checks answer "who may write," not "is the result legal" ([open-problems.md](open-problems.md)
  §1). The bank-account problem survives the product layer. *Mitigation*: keep
  true global bounds in escrow/reservation CRDT modeling inside `state-apply`;
  document that `myrhiza-permission-*` gates *authorship*, not *result legality*.
- **Don't over-invest in a fully declarative permission DSL.** Rocicorp
  **deprecated** declarative RLS rules for arbitrary server functions
  ([zero-rocicorp.md](zero-rocicorp.md)); declarative cross-object authorization
  hits expressiveness walls ([open-problems.md](open-problems.md) §6).
  *Mitigation*: let `myrhiza-permission-rbac` cover the common role/claim cases
  declaratively, but keep the `state-apply` code path as the escape hatch for
  authorization too complex to declare — don't try to make the module language
  Turing-complete.
- **Don't treat revocation as instantaneous.** cojson revocation is forward-only
  and racy across partitions ([open-problems.md](open-problems.md) §3).
  *Mitigation*: Myrhiza's revocation-seq freshness
  ([distribution.md](../../specs/2026-05-09-myrhiza-master-design/distribution.md) §10.7) must specify what
  happens to concurrent writes by an about-to-be-revoked author — role-at-tx-time
  semantics mean some are valid.
- **Don't model entitlement as "what the server sends."** PowerSync buckets /
  Electric shapes / Zero queries conflate *partial replication* with
  *authorization* ([powersync.md](powersync.md),
  [electricsql-comparator.md](electricsql-comparator.md)). That only works with a
  trusted enforcer. *Mitigation*: Myrhiza v1 keeps "every peer holds everything"
  ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.5); when partial
  replication arrives, authorization must remain a separate on-DAG verdict, never
  "the subset you received is the subset you're allowed."
- **Don't claim cojson is battle-tested.** No named at-scale deployment verified;
  unfunded; 2.0 in alpha. Cite it as architectural proof, not maturity proof.

## Borrow

Primitives worth studying for the `myrhiza-permission-*` ABI:

- **cojson role ladder** (`reader < writer < manager < admin`, plus `writeOnly`)
  and the `isHigherRole` rule that *you cannot assign a role above your own*
  ([jazz-cojson.md](jazz-cojson.md)). A compact, audited RBAC vocabulary for
  `myrhiza-permission-rbac`. `writeOnly` (write but read only your own) is a
  genuinely useful fifth role for drop-box / invite-request flows.
- **`determineValidTransactions` structure** — reconstruct authority state at the
  write's causal point, look up effective role, mark valid/invalid — is a direct
  template for the permission-module half of the `state-apply` verdict.
- **Triplit's per-operation granularity** (read / insert / update / **postUpdate**
  / delete; [triplit.md](triplit.md)). `postUpdate` (check the *resulting* row) is
  the pre-check-the-hypothetical-post-state idea (§4.4) generalized — adopt the
  vocabulary.
- **PowerSync's auth-params-trusted / client-params-untrusted split**
  ([powersync.md](powersync.md)). Any Myrhiza grant API must mark which inputs are
  signed-and-trusted vs caller-supplied. Clean, hard-won distinction.
- **PowerSync bucket / Electric shape** as the v2 partial-replication design when
  Myrhiza hits the §4.5 scaling ceiling — but keep authority a separate on-DAG
  verdict (see Avoid above).
- **Electric gatekeeper** (token claim = exact resource definition) as a
  capability-token shape ([../capability-tokens/](../capability-tokens/)) — useful
  framing for resource-handle grants ([capabilities.md](../../specs/2026-05-09-myrhiza-master-design/capabilities.md)
  §7.4), minus the trusted-online-checker.

## Recommended posture for the spec

1. **Adopt the cojson shape explicitly and cite it.** The `state-apply` authority
   verdict and the `myrhiza-permission-*` modules are the cojson model relocated
   into WASM Components. Say so; name cojson as the prior art.
2. **Split the ABI: authorship-gate vs result-legality.** `myrhiza-permission-*`
   gates *who may author* (role/claim check, declarative, cheap). Global-invariant
   legality stays in app `state-apply` logic (escrow/reservation). Don't conflate.
3. **Specify role-at-deps-closure semantics.** State that the verdict is evaluated
   against the authority state implied by the event's `deps`, and define
   concurrent-revocation behavior — the gap cojson hits in production.
4. **Keep the declarative module thin; keep the code escape hatch.** Learn from
   Rocicorp's deprecation: declarative covers the 80%, code covers the rest.
5. **Treat read-confidentiality as a crypto module, not a permission verdict.**
   Reader-role enforcement = key sealing ([crypto.md](../../specs/2026-05-09-myrhiza-master-design/crypto.md)),
   distinct from the authorship verdict.

## Sources

- Synthesizes the per-product files in this folder.
- [../crdts/open-problems.md](../crdts/open-problems.md), [../crdts/lessons.md](../crdts/lessons.md).
- Myrhiza spec: [convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4–4.5,
  [capabilities.md](../../specs/2026-05-09-myrhiza-master-design/capabilities.md) §7,
  [distribution.md](../../specs/2026-05-09-myrhiza-master-design/distribution.md) §10.7,
  [identity.md](../../specs/2026-05-09-myrhiza-master-design/identity.md).
