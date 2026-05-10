**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Capability model


## 7. Capability model

Three layers of gating, each at a different boundary, plus
typed resource handles for non-forgeable inter-component refs.

### 7.1 App boundary — manifest declares ambient set

Every app's `manifest.toml` declares its ambient capability set:
which host imports it may call, which UI surfaces it may bind, which
modules it depends on. The manifest is signed (per [distribution.md](distribution.md) §10) so the
declared set cannot be modified after publication.

At install time, the kernel renders a capability summary to the user
(bech32m-encoded author identity, version, declared capabilities).
The user confirms or rejects.

### 7.2 Module boundary — manifest intersection at link time

When an app declares a module dep, the module brings its own
capability declarations (what host imports it requires to function).
The kernel **intersects** the app's ambient set with the module's
required set at component instantiation:

```
M_effective = A_ambient ∩ M_required
```

A module can never exceed the calling app's grants. An app cannot
grant a module more than the module declared needing. This catches
both directions: malicious modules declaring excessive imports, and
apps trying to over-grant.

If the intersection is empty for a required capability — i.e. the
module needs something the app didn't declare — installation fails
with a precise error.

### 7.3 Per-call gating on high-value ops

Specific privileged operations are re-checked at every call against
the **calling component's** manifest:

- Clipboard write
- File picker invocation
- Top-level navigation
- Push notification registration
- AEAD seal/open with sensitive keys
- Network egress to specific origins (when interaction calls out to
  third-party services)

The list is curated; what counts as "high-value" is a child-spec
concern. The mechanism is uniform: the WIT contract for each such op
includes a per-call gate annotation; the kernel reads the calling
component's manifest at invocation time and rejects calls that
exceed its declared scope.

This catches social-engineering attacks: an interaction component
asking a UI module to write to clipboard cannot escalate beyond the
*interaction component's* clipboard grant, even if the UI module
itself has clipboard access.

### 7.4 Resource handles for non-forgeable refs

WASM Component Model resource handles (free from [abi.md](abi.md) §8's full-CM ABI
choice) are the unit of explicit capability transfer. Apps pass
scoped handles to modules to grant fine-grained access:

```wit
// (Illustrative pseudocode. The actual API for creating private
// channels is defined in the kernel WIT package; the example below
// shows the pattern, not the exact host import name.)
let channel-handle = host.create-private-channel();  // illustrative
my-module.process(channel-handle);
```

The module cannot forge equivalent handles. It can only use what
was passed.

This pattern complements §7.1–7.3: ambient grants set the bounds,
intersection scopes the module, per-call gates protect high-value
ops, and resource handles enable explicit fine-grained transfer.

### 7.5 Defense in depth

The four layers catch different attack classes:

| Attack | Caught by |
|---|---|
| Malicious app over-declares capabilities | User reviews at install (§7.1) |
| Malicious module declares more than it needs | Manifest intersection (§7.2) |
| Compromised module attempts privilege escalation | Manifest intersection + per-call gate (§7.2 + 7.3) |
| Module forges capability ref | Resource handle non-forgeability (§7.4) |
| Social engineering across components | Per-call gate (§7.3) |
| Compromised behavior signing fake non-event payloads | Structural validation in `host.author-event` (§6.1) |
| Silent capability widening on update | Per-update install flow re-runs capability summary ([distribution.md](distribution.md) §10.5) |
| Typosquatting on module names | Content-hash binding ([distribution.md](distribution.md) §10.6); name is informative only |

The cost of these layers is **moderate, not free**. Manifest
intersection requires a capability vocabulary registry + intersection
check at every component instantiation. Resource handles come with
WCM full CM (Q2-A) but require disciplined SDK design. Per-call gating
needs WIT annotation infrastructure and a manifest lookup at every
high-value-op invocation (microseconds-class per call, but real). The
benefit justifies the cost: comprehensive containment of modules,
which is essential because modules are pulled in by apps and may come
from third parties.

**What this defense does NOT cover**:
- A user who explicitly grants a malicious app full capabilities at
  install (§7.1 user review can be ignored)
- A network adversary that controls the relay infrastructure (relays
  are dumb topic bridges; metadata correlation is a separate threat
  class — see [networking.md](networking.md) §11.4)
- A malicious admitted member of a topic (group encryption protects
  against outsiders; insiders see what they were invited to see)
- An author whose private key is compromised post-install (revocation
  flow per [distribution.md](distribution.md) §10.7 mitigates but cannot fully defend)


