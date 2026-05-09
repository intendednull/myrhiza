**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — Pear Runtime: P2P app distribution and execution model

# Pear Runtime

Pear is the application runtime layer of the Holepunch stack. It is the only
P2P-app runtime in this prior-art set with a real, shipped consumer-mobile
deployment behind it (Keet messenger). Read this not as an API to copy, but as
the highest-volume reality check we have on what "P2P apps for end users"
actually looks like once a vendor commits to it.

## What Pear Is

Pear (`holepunchto/pear`, Apache-2.0, 241 stars, created 2024-02-03, last
updated 2026-05-06) is a P2P application runtime. An application is a JavaScript
project. Distribution is *over Hyperdrive*, addressed by a public key, not over
HTTPS via a vendor-operated app store.

You install Pear with:

```sh
npx pear
```

This downloads the platform and links a `pear` shim into `PATH`. (The npm
package `pear-cli` is stale — last published 2022-02-14 at v2.5.9 — and is
**not** the current install path. The internal `package.json` of `holepunchto/pear`
is at `"version": "2.0.0"`, but Pear self-distributes via Hyperdrive after
bootstrap, so the npm version number is not the meaningful one.) The platform
itself bootstraps from a known production Hyperdrive key
(`pear://gd4n8itmfs6x7tzioj6jtxexiu4x4ijiu3grxdjwkbtkczw5dwho`), then updates
itself in-place via Hyperdrive replication. The runtime *is* a Pear app.

You then launch any application with:

```sh
pear run pear://<key>
```

or, on a desktop with the URL handler registered, by clicking a `pear://` link.

## The "No Servers" Stance

Pear's headline pitch is no publisher infrastructure. There is no app store,
no CDN, no upload endpoint, no signing authority operated by Holepunch in the
hot path of distribution. Apps are hosted by their participants: anyone who
runs the app contributes blocks to other peers replicating the same Hyperdrive.

Compare with the npm/HTTP-based status quo:

| Concern | npm/HTTP | Pear |
|---|---|---|
| Discovery | Registry index (npmjs.com) | Out-of-band: share a key |
| Storage | Vendor-operated CDN | Peers swarming on the key |
| Authentication of artifact | TLS + signature on tarball | Public-key addressing of the drive |
| Update push | Author publishes to registry | Author appends to Hyperdrive; readers pull on next launch |
| Outage surface | Registry, CDN, DNS | DHT bootstrap, peer availability |

The honest tradeoff: Pear shifts the failure mode from "registry/CDN goes
down" to "no one is seeding this key right now." For a popular app (Keet) this
is fine — the userbase itself seeds. For a long-tail app it is a real
liveness question that the model does not answer for free.

## App Format

A Pear app is an ordinary npm-style JavaScript project. The `package.json` has
a `pear` field declaring runtime configuration. Looking at Pear's own
`package.json`:

```jsonc
{
  "name": "pear",
  "version": "2.0.0",
  "main": "./boot.js",
  "pear": {
    "name": "pear",
    "stage": { "ignore": [...] },
    "platform": { "fullSync": 0, "runtimes": "pear://0.3278.gd4n8itmfs..." }
  },
  "subsystems": ["./subsystems/sidecar/index.js"]
}
```

The `pear` field carries staging/build hints and runtime metadata.
Capability-style declarations (filesystem scope, GUI vs terminal, etc.) live
inside this field — see <https://docs.pears.com/reference/configuration.html>
for the canonical list.

The build pipeline:

1. `pear init` — scaffold a project
2. `pear stage <channel> <dir>` — produce a hash-addressed staged build,
   writing it into a Hyperdrive
3. `pear release <channel>` — mark a staged build as the released version
4. `pear seed <channel>` — keep a node online seeding the drive

Once staged, the app *is* a Hyperdrive snapshot. The "address" of the app is
the public key of that drive, prefixed `pear://`.

## Update Mechanism

Updates do not go through any registry. The author pushes a new version to
the same Hyperdrive key, which is just an append to the underlying Hypercore.
Running peers detect the new length on next launch (or via the long-running
sidecar process if it's already running), pull the diff, and restart against
the new version.

Versioning is **by Hypercore length**, not semver. The on-wire identifier of a
release is `pear://<length>.<key>` — e.g.
`pear://0.3278.gd4n8itmfs6x7tzioj6jtxexiu4x4ijiu3grxdjwkbtkczw5dwho` is "the
state of this Hyperdrive at length 3278." This makes "version" a strict-monotone
property of the publisher's append-only log. There is no branching, no
"v1.2.3 vs v1.2.3-rc1," no separate channel that diverges and re-merges. To
ship a divergent build, you make a new key.

Tradeoff: this gives you cryptographic clarity (one log, append-only, public-key
identity) at the cost of any model of branching, alternative builds, or
collaborative authorship beyond "co-sign the next append." Holepunch addresses
multi-author publishing via `hyper-multisig`, not via any branching primitive.

## Security Model

Pear runs each application in its own **OS process**, on the Bare runtime (see
[`bare-runtime.md`](bare-runtime.md)). Cross-app isolation is at the process
boundary — the same isolation Chromium gets between renderer processes. There
is no in-runtime capability sandbox in the sense of WASI / WebAssembly Component
Model: a Pear app, once launched, has the privileges of the Bare process it
runs in.

What is mediated:

- **Filesystem scope.** Each app has a per-app data directory. The `pear`
  package field declares filesystem permissions; the platform enforces them
  by configuring the Bare process accordingly.
- **Network.** Apps talk to peers via Hyperswarm/HyperDHT, all of which is
  brokered by Holepunch's own libraries. There is no host-mediated capability
  *type* for "open this socket" — it's library-level convention.
- **Encryption keys.** Apps generally use `hypercore-crypto` and friends. Key
  storage is per-app, in the per-app directory.

Honest critique: this is *process-level* isolation, not *capability-level* in
the OCAP sense. Compare to wasmtime's component-model imports, where the host
decides exactly which functions the guest can call and the guest cannot
syscall around them. Pear's guest is a full Bare process; if the OS lets it
syscall, the runtime cannot stop it. The Pear team is candid about this in the
recommended-practices docs: the security story leans on Bare being a small
runtime with a small surface, not on a guest sandbox.

## Relationship to Bare

Pear apps run on Bare. Bare is the JS runtime; Pear is the
deploy/discover/update wrapper, plus a long-running "sidecar" process that
manages running apps, drive replication, and updates. The sidecar is itself a
Bare process — see `boot.js` in `holepunchto/pear`:

```js
const BOOT_SIDECAR = 1
const BOOT_CLI = 2
switch (getBootType()) {
  case BOOT_SIDECAR: { require('./sidecar.js'); break }
  case BOOT_CLI:     { require('./cli.js');     break }
}
```

A typical desktop install has one sidecar daemon and zero-to-many app processes
spawned from it. The sidecar is the long-lived peer in the swarm; CLI/app
processes are short-lived clients that talk to it via `pear-ipc`.

Stack picture:

```
+--------------------------------------------------+
|  application (your JS)                           |
+--------------------------------------------------+
|  pear-* libraries (pear-api, pear-ipc, pear-run) |
|  hyper* libraries (hyperdrive, hyperswarm, ...)  |
+--------------------------------------------------+
|  Bare (JS runtime: V8 via libjs + libuv)         |
+--------------------------------------------------+
|  OS (Linux / macOS / Windows / iOS / Android)    |
+--------------------------------------------------+
```

## Keet as the Flagship

Keet is the production application validating the Pear stack. Keet desktop is
**closed-source** — there is no public `keet-desktop` repo under
`holepunchto`. We can't read the source, so claims about "Keet runs on Pear"
come from Holepunch marketing and Pear-docs cross-references rather than direct
inspection. The Keet website (keet.io) markets the chat product but does not
spell out the Pear/Bare runtime relationship in obvious page copy.
[paraphrased: Keet is presented as a peer-to-peer chat with no servers and
unlimited file sharing.]

What we can say with confidence:

- Bare ships on iOS and Android (Tier 1 in Bare's platform support matrix —
  see `bare-runtime.md`), and Holepunch publishes `bare-android` and `bare-ios`
  example projects. Keet's mobile apps almost certainly use this path, since
  it's the only path Holepunch maintains for embedding their JS stack in mobile
  binaries.
- Hyperswarm, Hyperdrive, and Hypercore are the data layer Keet is documented
  to use for chat, calls, and file transfer.

What we cannot directly verify without source: whether Keet desktop is
literally launched as a `pear://` app on end-user machines, or whether Keet
ships as a more conventional Electron-style packaged binary that *embeds* the
Holepunch stack rather than running as a Pear-managed app. Treat any specific
claim about Keet's deployment shape as marketing-confirmed-only until a Keet
team member confirms otherwise.

## Implications for Myrhiza

Direct lessons (things to copy):

- **Hash-addressed application identity.** Pear's `pear://<length>.<key>` is
  the cleanest existing solution to "what does an app's identity even *mean*
  in a P2P world." Myrhiza apps should be addressed by content-hash of the
  WASM-component bundle plus a publisher key, not by registry-issued name.
- **Self-distribution via the data layer.** Pear ships Pear over Hyperdrive.
  Myrhiza's runtime distributing itself the same way the apps do is a strong
  forcing function on substrate completeness — if your runtime can't carry
  itself, it probably can't carry an app either.
- **Append-only versioning.** Hypercore-length-as-version eliminates a whole
  class of "which 1.2.3 do you mean" bugs. Myrhiza should resist semver as the
  primary version channel; use signed-append history.
- **Sidecar pattern for long-lived peer presence.** App processes come and go;
  the swarm presence lives in a daemon. Myrhiza will need the same, otherwise
  apps cannot accept incoming connections when their UI is closed.

Anti-patterns Myrhiza explicitly skips:

- **JS as the guest language.** Pear pays a steep cost for "guest is a full
  Node-flavor JS runtime": no real sandbox, no per-call capability mediation,
  no determinism, large attack surface. Myrhiza picks WASM Component Model
  precisely to avoid this; see [`../wasm-component-model/`](../wasm-component-model/).
- **OS-process as the only isolation.** Pear leans on the OS for all
  isolation. Myrhiza's `state-apply` profile demands stricter sandboxing than
  any OS process can give — the guest must be a pure function of `(prior state,
  event)` plus a deterministic helper set, full stop. See `## Component
  Profiles` in `CLAUDE.md`.
- **Capabilities as JSON conventions.** Pear's `pear` field declares
  capabilities, but enforcement is library-level. Myrhiza enforces capabilities
  at the WASM import boundary, where the guest *cannot* syscall around the
  host. Compare [`../wasmcloud/capability-model.md`](../wasmcloud/capability-model.md).
- **No determinism story.** Pear is fine with this because it isn't trying to
  do convergent state replication across peers. Myrhiza is. This single
  property — `state-apply` must be deterministic — divides the two stacks more
  than any other.

UX reality checks (things to take seriously even if we don't copy them):

- **Updates *will* happen mid-session.** Pear's experience is shaped by users
  not noticing they got a new version because the sidecar pulled it. Myrhiza
  will have the same expectation; design the runtime swap as a first-class
  operation, not an afterthought.
- **Mobile is not optional.** Keet's volume is on mobile. Any P2P runtime
  that punts on iOS/Android at v1 is choosing the desktop niche by default.
  See [`bare-runtime.md`](bare-runtime.md) for what Holepunch had to build to
  make this work.
- **No-app-store distribution is *legally* fine, but *socially* hard.**
  "Click this `pear://` link" is friction. Pear's own docs spend significant
  effort on the bootstrap step. Myrhiza will need the same humility.

## Cross-References

- [`bare-runtime.md`](bare-runtime.md) — the JS runtime Pear apps run on
- [`hypercore-stack.md`](hypercore-stack.md) — the data layer apps use
- [`hyperswarm.md`](hyperswarm.md) — peer discovery / DHT
- [`keet-and-apps.md`](keet-and-apps.md) — flagship app, ecosystem
- [`governance.md`](governance.md) — Holepunch Inc, license posture
- [`history.md`](history.md) — Dat → Hypercore → Pear evolution
- [`commercial.md`](commercial.md) — funding model, business viability
- [`comparisons.md`](comparisons.md) — Pear vs Iroh vs WASM Component Model
- [`critiques.md`](critiques.md) — known weaknesses
- [`open-problems.md`](open-problems.md) — what Pear hasn't solved
- [`lessons.md`](lessons.md) — distilled takeaways for Myrhiza spec authors
- Prior-art neighbors: [`../iroh/`](../iroh/) (transport comparison),
  [`../wasm-component-model/`](../wasm-component-model/) (substrate
  comparison), [`../wasmcloud/`](../wasmcloud/),
  [`../holochain/`](../holochain/)

## Sources

- Pear repo and README: <https://github.com/holepunchto/pear>
- Pear `package.json`: <https://github.com/holepunchto/pear/blob/main/package.json>
- Pear `boot.js`: <https://github.com/holepunchto/pear/blob/main/boot.js>
- Pear documentation index: <https://docs.pears.com/>
- Pear application configuration reference:
  <https://docs.pears.com/reference/configuration.html>
- Pear CLI reference: <https://docs.pears.com/reference/cli.html>
- Bare repo (runtime substrate): <https://github.com/holepunchto/bare>
- Hyperdrive: <https://github.com/holepunchto/hyperdrive>
- Hyperswarm: <https://github.com/holepunchto/hyperswarm>
- `pear-cli` (stale, do not use): <https://www.npmjs.com/package/pear-cli>
- Keet (closed-source flagship): <https://keet.io>
