**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Apps, modules, distribution

# Apps, modules, and bundle distribution

## 10. Apps, modules, and bundle distribution

### 10.1 Bundle shape

Apps and modules use the same shape:

```
bundle/
├── manifest.toml          author pubkey + version + capabilities
│                          + module deps + signature
├── components/
│   ├── state-apply.wasm
│   ├── state-propose.wasm
│   ├── interaction.wasm
│   └── behavior.wasm      (optional)
├── ui-assets/             (optional; static UI assets if present)
└── signature              Ed25519 over (manifest_hash + content_hash
                           + version + author_pubkey)
```

Modules use the same shape but may not include `state-propose` or
`behavior` profiles depending on what they implement.

### 10.2 Manifest schema (v1 normative)

The manifest schema is part of the v1 master spec, not a deferred
child spec, because §7.2's intersection mechanic cannot be specified
without it.

```toml
[app]
name = "counter"
version = "0.1.0"
description = "Simple shared counter"
# Author identity. bech32m-encoded Ed25519 pubkey with HRP discriminating
# author identity class:
#   wpub-author     — third-party app/module author
#   wpub-myrhiza    — official myrhiza-* module signing root
author-pubkey = "wpub-author1q9q...xy"
author-identity-class = "third-party"   # or "myrhiza-official"

[abi]
kernel-major = 1                   # target kernel major version
kernel-minor-min = 0                # minimum kernel minor for required imports
state-digest-format = "bincode-1.3"  # the only v1 value; future opt-in

[capabilities.host-imports]
# capability-gated host imports; kernel intersects with module deps
"host.author-event" = true
"host.broadcast" = true
"host.subscribe" = true
"host.kv.get" = true
"host.kv.put" = true
"host.kv.delete" = true
"host.kv.list-prefix" = true

[capabilities.ui-surfaces]
"ui:panel" = true
"ui:button" = true

[capabilities.high-value-ops]
# per-call gated; v1-mandatory list. Apps explicitly opt-in.
"host.clipboard.write" = false
"host.file-picker.show" = false
"host.navigation.top-level" = false
"host.push.register" = false
"host.aead-seal" = []              # list of key-handle namespaces app may seal under;
                                   # per-call gated to specific keys
"host.aead-open" = []               # same shape
"host.http.request" = []           # array of RFC 6454 exact origins (scheme + host + port);
                                   # empty = denied. v1 does NOT support glob/wildcard
                                   # patterns (subdomain-injection attack class). Future
                                   # kernel minor may add suffix-wildcard support behind
                                   # an explicit opt-in.

[capabilities.deterministic-helpers]
# state-apply may bind these; always permitted for that profile, listed
# for self-documentation
"host.verify-signature" = true
"host.verify-payload-mac" = true
"host.hash" = true
"host.install-key" = true
"host.now-hlc-from-event" = true

[determinism]
# state-apply discipline. v1 lints reject violations at install.
allow-floats = false               # v1: false; future opt-in via this field

[determinism.drift-detection]
# §4.7 TUTTI-shaped drift detection. Tunes how often each peer emits
# state-digest() output for cross-peer convergence verification.
interval-events = 1024             # emit digest every N events (canonical topo-sort index modulo N)
# Wall-clock backstop is disabled at v1 (would inject peer-local non-determinism)

[modules]
# Module deps. Each entry is content-hash-pinned, not name+version.
# Name is informative; the hash is the trust binding.
[[modules.dep]]
name = "myrhiza-permission-rbac"
content-hash = "blake3:abc123..."
expected-author = "wpub-myrhiza1xyz..."
required-capabilities = ["host.kv"]   # what this module imports from kernel

[[modules.dep]]
name = "myrhiza-state-snapshot-cache"
content-hash = "blake3:def456..."
expected-author = "wpub-myrhiza1xyz..."
required-capabilities = ["host.kv", "host.broadcast"]

[components]
# WASM component artifacts in this bundle, by profile.
state-apply = "components/state-apply.wasm"
state-propose = "components/state-propose.wasm"
interaction = "components/interaction.wasm"
behavior = "components/behavior.wasm"   # optional

[signature]
# Ed25519 signature over canonical encoding of:
#   length-prefixed("myrhiza/manifest/v1") |
#   length-prefixed(BLAKE3(manifest_body_without_signature_section)) |
#   length-prefixed(BLAKE3(components_directory_canonical)) |
#   length-prefixed(version_string) |
#   length-prefixed(author_pubkey_bytes)
# Canonical encoding: each field as 4-byte LE length followed by bytes.
algorithm = "ed25519"
value = "0x..."
```

**Capability vocabulary** is the table in §3.5 plus `ui:*` surfaces.
The v1 `ui:*` minimum vocabulary is enumerated in the kernel WIT
package at v1 ship: `ui:panel`, `ui:list`, `ui:message`, `ui:form`,
`ui:menu`, `ui:button`, `ui:input`, `ui:dialog`. Counter+poll MVP
exercises panel + button + input + form. Apps may declare any
of these; the kernel rejects unknown `ui:*` strings at install.

Apps cannot invent new capability strings outside the kernel-defined
vocabulary; the kernel rejects any unknown capability identifier at
install. Future kernel minor versions may extend the vocabulary; apps
declaring vocabulary requiring a higher kernel-minor are rejected by
older kernels (per `kernel-minor-min` field).

**ABI versioning semantics** are nuanced for state-apply imports:

- **Adding a non-deterministic import** (state-propose / interaction /
  behavior only) is a kernel **minor** version bump. State-apply
  cannot bind it; convergence is unaffected.
- **Adding a deterministic helper** that state-apply MAY bind is a
  kernel **major** version bump. Two peers running different majors
  applying the same event with the same state-apply WASM produce
  different state if the WASM imports a new helper from one major
  but not the other. This is convergence-breaking.
- **Removing or changing semantics of any import** is a kernel
  major version bump.

Apps declare `kernel-major` in manifest. Peers running incompatible
kernel-majors cannot interoperate on the same topic (§11.2 implicit:
topic IDs include `app_bundle_hash` which depends on the kernel-major
the app was built against; cross-major peers cannot subscribe to
the same topic).

**TOML canonicalization for signature**: the manifest signature
(below) is computed NOT over the TOML text itself but over a
**canonical bincode 1.3.x encoding** of the parsed manifest's
typed structure. This eliminates TOML-encoder-library drift entirely.

Canonical-encoding rules:
- Parse manifest with `toml_edit 0.22.x` (pinned at v1; bumping is
  a kernel minor version bump if-and-only-if the encoder is not
  involved in canonical signature computation; otherwise major).
- Convert to typed manifest struct (defined in `myrhiza-manifest`
  WIT package).
- Encode struct via the same bincode 1.3.x + Options chain pinned
  in §5.4.
- BLAKE3 the encoded bytes → `manifest_canonical_hash`.
- Author signs `manifest_canonical_hash + content_hash + version
  + author_pubkey`.

The TOML text is the human-readable representation; the canonical
encoding is the byte-stable signature target. This means apps may
freely re-format their TOML (whitespace, comments, key order) without
breaking the signature, as long as the parsed struct is unchanged.

`[[modules.dep]]` array in the parsed struct is sorted by
`content-hash` alphabetically before encoding (canonical order).
Strings are UTF-8 NFC-normalized at struct-construction time.
Numbers are canonical i64/u64 binary encoding via bincode.

The `[signature]` block is excluded from the body when computing
the signature (the signature signs the body, which by definition
does not contain itself).

Quoted dotted keys are required for capability identifiers containing
dots: `"host.author-event" = true` (unquoted `host.author-event = true`
is parsed as nested table `host.author-event` and conflicts with
sibling capability keys).

**Module dep content-hash discipline**: `content-hash` is the bundle's
iroh-blobs hash. Two modules with the same name but different content
hashes are different modules. Typosquatting is impossible because
the hash is the binding. The `name` field is informative for UI
display only.

**`expected-author` field**: the signing pubkey the kernel expects on
the module's bundle signature. If the module's actual signature is
under a different pubkey, install fails. This catches a compromised
hash-replacement attack.

**`author-identity-class`**: distinguishes third-party apps from
official myrhiza-* modules. The kernel maintains a small built-in
allowlist of `wpub-myrhiza` pubkeys (initially the project's signing
root); any module declaring `myrhiza-official` whose pubkey is not
on this list is rejected at install. This is a soft trust-root
signal — users may trust myrhiza-official modules differently than
third-party.

**Schema evolution**: adding a capability or module field is additive
(new kernel minor version). Removing or changing semantics of a field
is breaking (new kernel major version). The manifest schema version
is implicit in the kernel's `kernel-major` requirement.

### 10.3 Distribution

Bundles distributed via iroh-blobs by content hash. No central
registry. Discovery is out-of-band at v1: hashes shared via links,
QR codes, in-app share. Future-direction (deferred to child spec):
in-band catalog gossip for app/module discovery.

### 10.4 Signing

Author Ed25519 signs `(manifest_hash + content_hash + version +
author_pubkey)`. The signature is part of the bundle. The author
public key is embedded in the manifest.

Author identity reuses the IdentityScope primitive (§6). App
authors are users; user signing keys can sign app releases.
Production-grade authors typically use a separate IdentityScope
long-term identity for releases (separation of concerns).

### 10.5 Install flow

1. User receives bundle hash via out-of-band channel.
2. Kernel fetches bundle via iroh-blobs by hash.
3. Kernel verifies Ed25519 signature against author pubkey embedded
   in manifest. Cremers ETK 2025 enforcement: kernel structurally
   rejects any non-Ed25519 signature algorithm — there is no
   manifest field to declare alternative algorithms.
4. Kernel resolves module deps recursively. For each module dep,
   kernel fetches by content hash, verifies signature against
   `expected-author`, and recursively resolves transitive deps.
   Failures (hash mismatch, signature failure, capability excess)
   abort install with precise error.
5. Kernel intersects capability declarations across the dep tree:
   - Each module's required capabilities are intersected with the
     calling app's ambient set (§7.2).
   - Transitive module deps follow the same rule recursively. A
     module's required capabilities cannot exceed its calling
     module/app's grants.
6. Kernel renders capability summary to user:
   - bech32m-encoded author identity (with HRP class indicator —
     `wpub-myrhiza-...` highlighted as official)
   - version + bundle hash (truncated)
   - capability summary (host imports, high-value ops, ui surfaces)
   - module dep tree (each module's name + content hash + author)
   - high-value-op list separately highlighted
7. User confirms or rejects. **Kernel-controlled UI surface** (chrome
   the app cannot draw over) renders the prompt; high-value-op
   prompts must use the same surface (§7.3).
8. Kernel instantiates the app's components.

**Per-update consent**: when the app or any module dep updates,
step 7 re-runs. Users approve each update individually. Silent
in-place updates are forbidden — capability widening on update is
the attack class this defense closes.

**Per-module-update consent (separate from per-app-update consent)**:
when an app version bump changes ONLY a module dep (no app code
changes, no capability changes), the install flow surfaces the
module update specifically — "App X updated module M from hash Hold
to hash Hnew (capabilities unchanged)" — rather than rolling it
into the app-update prompt. Users may approve the module change
without approving an associated app capability change. This prevents
authors from hiding module substitutions inside larger app updates.

**`on-completion` UI rendering**: high-value-op approval prompts
(per §7.3) MUST render via the kernel-controlled UI surface defined
in §13.2.1 (kernel-rendered chrome that the UI app cannot draw
over). UI app cannot intercept or fake these prompts. Non-privileged
prompts (`host.user-prompt` for general intent) MAY render via the
UI app's own surface, with the understanding that the UI app is in
the TCB for those prompts.

**Capability summary fatigue mitigation** (skeptic finding):
- Default deny for capabilities not explicitly highlighted by the
  user as understood ("auto-approve trivial caps after N installs"
  is rejected).
- High-value-op prompts have a 2-second minimum render time before
  the Approve button enables (anti-clickthrough).
- Bech32m author identity rendered with visual hash icon (e.g.
  4×4 colored grid derived from pubkey) to ease author recognition
  across installs.
- New author identities highlighted as "first time installing from
  this author"; subsequent installs from same author show the same
  hash icon.

### 10.6 Versioning

Semver for human-readable version strings. Bundle hash (content-
addressed iroh-blobs hash) is the **trust binding** — semver is
informative only.

Module deps pin **content hashes**, not semver ranges. An app that
wants to allow semver-compatible upgrades publishes a new app version
referencing the new module hash; users approve the app update
(which surfaces the new module hash in the install flow's capability
summary at step 6).

This makes silent module updates impossible. An app cannot say "I
depend on `^1.0.0` of module X" and have the kernel auto-pull a
patched version; every module-version-bump is an app-version-bump
with explicit user consent.

### 10.7 Revocation

Author retracts a bad version by publishing a **revocation event**
signed under the same author IdentityScope. The revocation event
declares:

```toml
[revocation]
revoked-bundle-hash = "blake3:..."
reason = "string describing why"
revoked-at = "2026-05-09T12:34:56Z"
```

**Distribution mechanism (v1 commitment):**

- Revocations propagate via iroh-gossip on a **per-author
  revocation topic** computed as
  `topic_id = BLAKE3("myrhiza/revocations/v1" | author_pubkey)`.
- Every peer that has ever installed an app or module signed by
  this author auto-subscribes to the author's revocation topic on
  install.
- When a revocation arrives, the kernel surfaces it to the user
  for any installed bundle matching `revoked-bundle-hash`. User
  is prompted to uninstall (default action) or pin (explicit
  opt-in).
- Revocations are append-only and signed; previous revocations
  cannot be retracted.

**Threat model coverage:**

- **Author key compromise**: if the author's key is leaked, an
  attacker can forge revocations or new releases. The kernel cannot
  distinguish; users must out-of-band verify if a sudden revocation
  storm appears suspicious. Future direction: key transparency
  log + petname registry (deferred to identity-binding child spec).
- **Stale-network attack**: an adversary may withhold revocation
  events from a target peer. Mitigation: revocation topic is part
  of the auto-subscribed set; peers run a HeadsSummary-shape sync
  on the revocation topic at start to backfill missed revocations.
  Peers without a fresh sync within 24 hours surface a "potentially
  stale" warning before installing a new version.
- **Mass revocation by malicious author / flooded revocation topic**:
  revocation events MUST carry a monotonically-increasing
  `revocation-seq: u64` per author. The kernel tracks the highest
  observed `revocation-seq` per author and rejects revocations with
  lower or equal seq. Single-key compromise can therefore at-most
  publish one revocation per (author, seq); a flood of fake
  revocations under the same seq is structurally impossible.
  **Maximum seq jump**: the kernel rejects any revocation whose seq
  exceeds `last_observed_seq + MAX_REVOCATION_JUMP` (default 1024
  per author per 24-hour window). This prevents a compromised key
  from publishing seq=`u64::MAX` and bricking the author's
  revocation channel. If the legitimate author needs to revoke many
  bundles fast, they may publish at most 1024 revocations per
  24-hour window. Users may pin a specific bundle hash (decline
  revocation); the kernel surfaces pinning prominently when an
  author's revocation sequence jumps abnormally fast.

**Subscription enumeration risk**: a relay observing revocation
topic subscriptions can enumerate which peers ever installed software
from author A (subscription is sticky after install). This is part
of the §11.4 metadata-correlation surface; mitigation requires
relay rotation + topic-subscription cover (out of scope for v1;
named in §19).

**Out of scope at v1**: certificate-transparency-style log;
post-revocation re-keying; revocation forwarding via third-party
attestations.

### 10.8 No central registry

No Myrhiza-operated registry. No sigstore dependency. No reliance on
any centralized service for app distribution. P2P-native distribution
is non-negotiable; matches iroh-blobs commitment and the project's
no-deployed-infrastructure framing.

### 10.9 Myrhiza-official signing root

The kernel maintains a small built-in allowlist of bech32m-encoded
Ed25519 pubkeys with HRP `wpub-myrhiza` recognized as the official
project signing root. Modules signed by these pubkeys may declare
`author-identity-class = "myrhiza-official"` in their manifest.

The allowlist is hard-coded in the kernel binary (a `const` in
`crates/kernel/src/identity/official_root.rs`). Updating the allowlist
requires a kernel binary update — i.e. users must re-install the
kernel to trust new official pubkeys.

This provides a soft trust-root signal — modules signed by listed
pubkeys may be treated differently in install UX (e.g. less prominent
warnings) but the kernel does not block third-party modules.

**Initial allowlist members** (provisional; pinned at v1 ship time):
- The Myrhiza project's primary release-signing pubkey.
- Three backup pubkeys held offline by separate maintainers, used
  for **community-attested rotation** of the primary key. Rotation
  procedure: the new primary pubkey is announced via three separate
  channels (project website / community forums / signed posts under
  maintainer identities), and an emergency kernel binary update
  carries the new allowlist. The backups are not used as a
  cryptographic threshold signature in v1 — proper threshold-Ed25519
  schemes (e.g. FROST-Ed25519 IETF draft) are not yet RFC-stable
  and adding their verification logic to the kernel TCB at v1 is
  premature. Future kernel majors may adopt FROST-Ed25519 once it
  reaches RFC.

**v1 rotation is policy + emergency-update, not cryptographic
threshold.** This is honest about what the maintainer ceremony
actually provides. The threat model assumes that compromising the
primary key requires also compromising the kernel-update channel
(§10.10) for an attacker to land malicious modules — defense in
depth via separate trust roots, not via threshold cryptography.

### 10.10 Kernel binary distribution and authentication

The Myrhiza-official allowlist (§10.9) is the trust root for module
signing. **The kernel binary itself is the trust root for the
allowlist.** Distribution and authentication of the kernel binary
matter as much as any in-spec security mechanism.

**v1 kernel distribution channels** (operator chooses):

1. **OS package managers** (homebrew, apt, dnf, MSI). The package
   manager's signing infrastructure verifies the binary; users trust
   the package manager root. This is the recommended path for desktop.
2. **GitHub release artifacts** with reproducible builds. The Myrhiza
   project signs each kernel release with a separate offline-key
   "kernel signing root" (distinct from the module-signing allowlist).
   The kernel-signing-root pubkey is published in the project README,
   on the project website, and via the `wpub-myrhiza-kernel` HRP. Users
   verify by checking the kernel binary's signature against this root.
3. **Reproducible build verification**: kernel source is open; users
   may build from source and compare against published checksums.

**Kernel-signing-root rotation**: if the kernel-signing-root key is
compromised, the project announces rotation via:
- Out-of-band channels (project website / community forums / signed
  social posts under maintainer identities).
- A signed advisory pushed via the `myrhiza-revocation` topic
  (§10.7-shape) under the offline backup keypair.
- Distribution channels (OS package managers, GitHub) are updated to
  the new signing root.

**v1 acknowledged risk**: a sophisticated adversary controlling both
the project's release infrastructure AND the OS package manager could
distribute a compromised kernel. Mitigation is reproducible builds +
multi-channel announcements; v1 does not commit a transparency log
or third-party attestation. Future direction (v2+): kernel-binary
transparency log + community attestation.

**Out-of-band trust still required for first-install**: a user
downloading Myrhiza for the first time must trust the publication
channel. Users who care can verify the binary against the
kernel-signing-root pubkey published on the project website (HTTPS
+ DNSSEC) and via community-mirror posts. v1 does not hide this
gap; it is the standard "trust the OS package manager" model used
by every desktop runtime.


