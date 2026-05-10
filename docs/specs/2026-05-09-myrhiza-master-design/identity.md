**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Identity primitive


## 6. Identity primitive

A single kernel primitive covers user identity, multi-device
identity, behavior identity, and MLS LeafNode identity.

### 6.1 IdentityScope

```wit
// identity-handle is an opaque WIT resource — components hold it but
// cannot inspect or forge its contents. Resource lifecycle is kernel-
// managed.
resource identity-handle {
    // No methods exposed to components. Handles are passed by value to
    // host imports that consume them.
}

// peer-handle is also opaque; one per peer the kernel currently knows.
resource peer-handle {}

record identity-scope {
    long-term: borrow<identity-handle>,
    instance: option<instance-binding>,
}

record instance-binding {
    peer: borrow<peer-handle>,
    kind: instance-kind,
    name: string,
}

variant instance-kind {
    device,
    behavior,
    mls-leaf,
    custom(string),
}
```

`long-term` is the durable user/author/member identity. `instance`,
when present, is the short-lived per-(peer, instance) signing scope
nested under that long-term. `instance: none` means the operation is
performed by the long-term identity directly.

The kernel custodies all private keys. Components see only opaque
`identity-handle` resources (non-forgeable, non-inspectable per WCM
resource semantics) and scope records that borrow handles. To sign,
components call:

```wit
host.author-event(scope: identity-scope, event-payload: list<u8>) -> sig
```

The kernel verifies the calling component is authorized to use the
scope (per [capabilities.md](capabilities.md) §7), validates that `event-payload` is a structurally-valid
event under the app's WIT contract (envelope shape, deps array,
payload type), looks up the appropriate private key, signs the
canonical encoding, and returns the signature. Private keys never
enter component memory.

**Why structural validation matters**: a compromised behavior
component cannot use the kernel's signing capability to sign arbitrary
non-event bytes (e.g. a fake bundle manifest, a fake identity claim)
under the user's behavior scope. The kernel rejects malformed payloads
before signing.

**Per-(profile, payload-variant) authorization**: structural validity
checks "well-formed under the WIT type"; it does NOT check "the
calling profile/component is authorized to author this specific
variant." Apps that want fine-grained variant-level control declare
**permitted-author-set** in their manifest:

```toml
[author-policy]
# Map from profile to set of payload variants that profile may author.
# Variants not listed are forbidden (deny-by-default).
state-propose = ["UserAction", "Comment"]
behavior = ["AutoArchive", "RemindEveryone"]
```

The kernel checks `(calling-profile, payload-variant)` against this
manifest at every `host.author-event` call. Variant identification
uses WIT variant tag names.

Apps using behaviors for limited tasks (e.g. auto-moderation) should
declare a tight `behavior` variant set so a compromised behavior
cannot author admin-class events.

**v1 default for `[author-policy]`**: deny-by-default. Apps that
omit `[author-policy]` may NOT use `host.author-event` at all under
non-state-propose profiles (i.e. `behavior` profile cannot author
events without explicit policy). Apps that explicitly set
`policy = "permissive"` opt out (any profile may author any variant)
— useful for simple apps where the cost of variant enumeration
outweighs the security benefit.

This makes defense-in-depth the default and forces app authors to
*think* about which variants behaviors should be allowed to author,
rather than getting authorization-bypass-by-omission.

**State-apply re-validation**: every peer's state-apply re-checks
`(calling-profile, payload-variant)` against the manifest's author-
policy at apply time, not just at originator-side propose. Since
the manifest is content-hash-pinned via `app_bundle_hash`, every
peer materializing the topic shares the same author-policy. A
compromised originator that bypassed local pre-check still fails
remote apply, and the event is rejected from convergence.

**Cremers ETK 2025 enforcement is structural**: the kernel does not
expose any signing API that takes an algorithm parameter.
`host.author-event` always uses Ed25519 (RFC 8032 strict). Manifest
fields cannot declare alternative algorithms. ECDSA is unreachable
through the kernel surface.

### 6.2 Use cases under one primitive

| Use case | long-term | instance |
|---|---|---|
| User signing (single device) | User Ed25519 | none |
| User multi-device | User Ed25519 | `kind: device, name: "laptop"` |
| Behavior bot | Owner Ed25519 | `kind: behavior, name: "discord-bridge-1"` |
| MLS member, current epoch | Member Ed25519 | `kind: mls-leaf, name: "epoch-42"` |
| App author signing a release | Author Ed25519 | none |

**Cremers ETK 2025 constraint**: `long-term` MUST use Ed25519 (which
is SUF-CMA secure). ECDSA is EUF-CMA only and fails MLS FCGKA security.
This applies even to non-MLS scopes for forward compatibility. (For
context: Cremers et al. 2025 "End-to-end Tree-based Key agreement"
showed MLS implementations using EUF-CMA-only signatures break
Forward-Compromise Group Key Agreement security; SUF-CMA is the
stricter property Ed25519 provides. See `prior-art/mls/critiques.md`.)

### 6.3 Direction for deferred items

These items have a committed direction in the master spec; detailed
mechanics land in child specs as concrete needs emerge.

- **Device-add and device-revoke flow** — direction: an app-level
  `myrhiza-identity-multi-device` module implements device addition
  via in-band signed events from existing devices. The module wraps
  the IdentityScope primitive; the kernel does not bake device
  semantics. Device revocation is broadcast as a signed retirement
  event under the long-term identity. v2 child spec details.
- **MLS LeafNode lifecycle integration** — direction: the
  `myrhiza-crypto-mls` module composes IdentityScope with
  `instance-kind: mls-leaf` for epoch-bound signing keys. Per-epoch
  key rotation is module-internal; kernel exposes only primitive
  crypto ([crypto.md](crypto.md) §9.2). v2+ child spec details.
- **Recovery semantics when long-term key is lost** — direction:
  social recovery (M-of-N trusted peers attest to a recovery event
  re-binding the long-term identity to a new keypair) OR
  out-of-band recovery via a stored recovery seed. Both deferred to
  multi-device child spec; v1 documents this gap honestly — losing
  a single-device IdentityScope without recovery is permanent identity
  loss.
- **Cross-peer behavior continuity** — direction: apps that want
  stable bot identity across peers register an in-band mapping event
  mapping peer-side behavior keypair to an app-level role; enforced
  by the app's own pre-check. SDK macros default to making this
  binding explicit so app authors don't accidentally ship behaviors
  that lose identity on restart.
- **Behavior identity revocation** — direction: a behavior keypair
  may be revoked by the user via a kernel-side `BehaviorRevoke` event
  authored under the user's long-term IdentityScope, naming the
  (peer, kind, name) tuple. After revocation, future events signed
  under that scope are flagged in derived state as "post-revocation"
  (apps choose whether to treat them as invalid). This handles
  compromised behavior keys without requiring app-level cooperation.
  v2 child spec details mechanics; v1 documents the gap (no
  revocation path for behavior keys).
- **Quantum-safe signature migration** — direction: kernel ABI bump
  with new `instance-kind` variant for PQC scope; existing scopes
  remain Ed25519. App authors opt-in to PQC scopes when modules support
  them. Out of scope until post-quantum schemes (e.g. ML-DSA) reach
  production maturity.


