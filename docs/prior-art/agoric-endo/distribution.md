**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric — release engineering, monorepos, and the practical pin policy

# Distribution and packaging

Two monorepos, two release cadences, two npm scopes. Pinning policy matters because cross-package compatibility is *not* automatic — `@agoric/*` and `@endo/*` are coupled but not lock-step.

For the chain context see [chain.md](./chain.md), for what's in the packages see [contracts.md](./contracts.md), and for who consumes them in production see [apps.md](./apps.md).

## The two monorepos

### `endojs/endo`

The lower-stratum library tree. Pure TypeScript / JavaScript, no Cosmos SDK, no Go, no chain code. **This is what MetaMask actually ships.**

- Repo: [github.com/endojs/endo](https://github.com/endojs/endo)
- npm scope: `@endo/*` plus the un-scoped `ses`
- Released package-by-package; each release is a git tag of the form `@endo/<pkg>@<version>` or `ses@<version>`
- Coordinated by Endo's own version tooling (lerna-style), not lock-stepped to agoric-sdk

The packages most relevant to a downstream consumer like us:

| Package | Latest (verified 2026-05-09) | Published | Role |
|---|---|---|---|
| `ses` | **2.0.0** | 2026-04-17 | The `lockdown()` + `Compartment` substrate. Just-shipped major version. |
| `@endo/lockdown` | **1.0.19** | 2026-04-16 | Re-export wrapper around `ses` that calls `lockdown()` at import. |
| `@endo/init` | **1.1.13** | 2026-02-26 | Convenience entry point: `import '@endo/init';` boots SES with sensible defaults. |
| `@endo/marshal` | **1.9.1** | 2026-04-17 | Pass-by-copy / pass-by-presence serialization. Underpins CapTP and vstorage. |
| `@endo/captp` | **4.5.0** | 2026-02-26 | Capability transfer protocol — distributed object messaging over a transport. |
| `@endo/eventual-send` | **1.5.0** | 2026-04-16 | `E(remote).method(...)` syntax — promise-pipelined sends. |
| `@endo/far` | **1.1.14** | 2025-07-12 | `Far(label, methods)` — make a JS object remotable. |
| `@endo/exo` | **1.7.0** | 2026-04-17 | Durable exo-class objects (the upgrade-safe object framework). |
| `@endo/pass-style` | **1.8.0** | 2026-04-16 | The pass-style classification (copyable vs remotable vs promise). |
| `@endo/patterns` | **1.9.0** | 2026-04-16 | Structural pattern matching — what Zoe uses for offer-shape validation. |
| `@endo/ses-ava` | **1.4.1** | 2026-04-16 | AVA test runner integration that boots SES first. |
| `@endo/compartment-mapper` | **2.1.0** | 2026-04-16 | Resolves a node_modules tree into a Compartment-shaped graph. |
| `@endo/bundle-source` | **4.3.0** | 2026-04-17 | Single-file bundler used to package contracts and snaps. |
| `@endo/promise-kit` | **1.2.1** | 2026-04-16 | `makePromiseKit()` — explicit-resolver promises. |
| `@endo/ocapn` | **1.0.0** | 2026-04-16 | OCapN protocol — interop with Spritely Goblins. *Just hit 1.0.* |

Sources: per-package npm registry queries via `https://registry.npmjs.org/<pkg>` and the GitHub releases for the endo monorepo.

### `Agoric/agoric-sdk`

The chain-and-contract-framework monorepo. Yarn workspaces. Mixed JS + Go (the `cosmic-swingset` and `agoric-sdk` Go modules live here).

- Repo: [github.com/Agoric/agoric-sdk](https://github.com/Agoric/agoric-sdk)
- npm scope: `@agoric/*`
- Releases tagged either `agoric-upgrade-N[-rcM]` (the chain consensus releases) or `<pkg>@<version>` (npm publishes), or `ymax-v<...>` / `v0.3.YYMM-itest<N>` for the orchestration-product release lines

Recent chain-upgrade tags (verified via `gh api repos/Agoric/agoric-sdk/releases`):

| Tag | Published | Notes |
|---|---|---|
| `agoric-upgrade-23-rc1` | 2026-05-06 | In-flight |
| `agoric-upgrade-22b` | 2025-10-20 | Current mainnet |
| `agoric-upgrade-22` | 2025-09-17 | |
| `agoric-upgrade-21` | 2025-07-07 | |
| `agoric-upgrade-20` | 2025-05-15 | |
| `agoric-upgrade-19` | 2025-03-20 | |

Cadence: roughly **one chain upgrade every 1–2 months** through 2025–2026. Each upgrade is a coordinated halt + binary swap on validators. SwingSet kernel state survives across upgrades (the "orthogonal persistence" property).

Recent npm releases of the key `@agoric/*` packages (verified May 2026):

| Package | Latest | Published |
|---|---|---|
| `@agoric/swingset-vat` | **0.33.0** | 2026-04-08 |
| `@agoric/zoe` | **0.27.0** | 2026-04-08 |
| `@agoric/ertp` | **0.17.0** | 2026-04-08 |
| `@agoric/cosmic-swingset` | **0.42.0** | 2026-04-08 |
| `@agoric/vats` | **0.16.0** | 2026-04-08 |
| `@agoric/casting` | **0.5.0** | 2026-04-08 |
| `@agoric/orchestration` | **0.2.0** | 2025-07-12 |
| `agoric` (CLI) | **0.22.0** | 2026-04-08 |

Note the **synchronized publish dates** across @agoric packages (2026-04-08) — that's deliberate. agoric-sdk releases its npm packages in lock-step bursts so that intra-monorepo cross-references resolve cleanly. This is *not* true within `@endo/*`, where individual packages release on their own cadence.

## Versioning discipline

### Pre-1.0 vs 1.0+

Endo has a mixed spread:

- The "core ABI" packages have hit 1.0 (`@endo/marshal` 1.9.1, `@endo/eventual-send` 1.5.0, `@endo/far` 1.1.14, `@endo/lockdown` 1.0.19, `@endo/exo` 1.7.0, `@endo/ocapn` 1.0.0, `@endo/captp` 4.5.0, `ses` 2.0.0)
- The tool packages have moved past 1.0 too (`@endo/compartment-mapper` 2.1.0, `@endo/bundle-source` 4.3.0 — both have been 1.0+ since at least 2024)

Within @agoric, **almost everything is still pre-1.0**:

- `@agoric/swingset-vat` is at **0.33.0** — that's a lot of zeros for a thing running on mainnet.
- `@agoric/zoe` is at **0.27.0**.
- The `agoric` CLI is at **0.22.0**.

Reading their semver: **minor bumps frequently include breaking changes**. The pre-1.0 status is genuine — they treat it as license to break things. This is an intentional choice that lets the chain evolve faster than a strict semver-1.0 commitment would allow, but it makes downstream pinning harder.

The escape hatch in @agoric land is the **`agoric-upgrade-N` tag**: that's the chain-side coordinated release, and the `@agoric/*` package versions you see at any given mainnet upgrade form a coherent set. The chain upgrade tag is the stable snapshot, not any individual npm version.

### Cross-package compatibility

`@agoric/*` packages depend on `@endo/*` packages. The `peerDependencies` and dependency declarations in `@agoric/swingset-vat` etc. tend to use caret ranges on @endo packages — but in practice the agoric-sdk repo pins to specific Endo versions in its yarn.lock and tests against that combination. **Mixing-and-matching @endo versions outside the tested combination is unsupported.**

There is no published compatibility matrix. The de facto authority is "whatever yarn.lock in `agoric-sdk@<tag>` says." This is operationally fine for Agoric (their CI tests one combination) but a real problem for downstream consumers who want a pin policy.

## Release cadence and breaking-change tolerance

Endo: **rolling releases**. Multiple times per month for active packages. The endo monorepo's RELEASES.md and CONTRIBUTING.md call out that breaking changes are signaled in changesets and folded into the next major bump of the affected package only.

Agoric: **chain-upgrade-driven releases**. The npm releases are batched and timed to chain upgrades. Off-cycle patch releases happen but are discouraged.

The honest summary: **Endo is a normal npm library project; agoric-sdk is a chain project that happens to publish npm packages.** They have different release philosophies and you should not expect agoric-sdk packages to feel "library-shaped."

## Practical pin policy for Myrhiza

If we end up depending on `@endo/lockdown` and/or `@endo/marshal` (the two most likely candidates):

1. **Pin `ses` exactly, not by caret.** `ses` is the substrate; subtle behavioral changes in `lockdown()` or `Compartment` affect us deterministically. Note: ses 2.0.0 just shipped (April 2026) — there will likely be a follow-up patch wave; consider waiting one minor before adopting.
2. **Pin `@endo/lockdown` to a known-compatible-with-ses version.** Currently 1.0.19 + ses 2.0.0 is the matched pair (lockdown 2026-04-16, ses 2026-04-17 — published one day apart in the same release wave). Match the publish dates as a heuristic for compatibility.
3. **Pin `@endo/marshal` exactly.** Wire-format compatibility matters. If two peers are exchanging marshalled values, they need byte-compatible marshal versions. `@endo/marshal` has had wire-format bumps in the past — 1.x is a stable wire format but minor versions have added shapes.
4. **Don't depend on `@agoric/*` at all unless you need Zoe / SwingSet semantics.** The chain-coupling and pre-1.0 churn aren't worth it for runtime utilities. Anything we need from agoric-sdk that isn't fundamentally chain-shaped is also available, more cleanly versioned, in Endo.
5. **Track `@endo/captp` carefully if we pursue P2P object messaging.** It's at 4.5.0 — older than the rest but more battle-tested. Used by both Agoric and (via OCapN) by Spritely. The CapTP wire format has been stable since 3.x.
6. **Re-pin every 6 months at minimum.** Endo is moving. ses 2.0 → 3.0 timing is unknown; ocapn 1.0 just landed. Static pins will stagnate; we want a quarterly review cycle.

A concrete starting pin (May 2026):

```
ses == 1.15.0      # known-stable, used by MetaMask Snaps
@endo/lockdown == 1.0.x       # match ses
@endo/marshal == 1.9.1        # current
@endo/captp == 4.5.0          # current
```

Wait one minor on ses 2.x → 2.1+ before adopting, unless we have a specific reason to bleed-edge. The MetaMask precedent here is informative — they are still on `ses ^1.15.0` as of May 2026 despite ses 2.0 being available. That's a strong signal that 1.15 is the conservative production pin.

## Cross-package compatibility — Myrhiza-specific concerns

If we use `@endo/marshal` and `@endo/captp` to talk to other ocap systems (e.g., Spritely Goblins via OCapN), we have a **wire-format dependency on more than just our own pin**. The other side's marshal version matters. OCapN is supposed to be the stable interop format, with `@endo/ocapn@1.0.0` being the just-released stake in the ground. If we want OCapN interop, our pin policy is dominated by what the OCapN spec stabilizes on.

For purely-internal use (peer A talking to peer B, both running Myrhiza), we can pin tighter and upgrade lock-step.

## Implications for Myrhiza

1. **Endo is consumable as a library; agoric-sdk is not.** If we want runtime primitives (Compartment, marshal, captp, eventual-send), use Endo packages directly, not via @agoric. They have cleaner semver, faster fixes, and aren't tied to a Cosmos chain release schedule.
2. **Watch ses 2.0 stabilization.** ses 2.0 (April 2026) is the substrate version everything else will eventually re-pin to. We probably want to adopt ses 2.x within 6 months, but not before MetaMask has — they will catch the regressions first because they have the user volume.
3. **Wire-format pinning is the trap.** `@endo/marshal` and `@endo/captp` have wire formats that two peers must agree on. This is *not* a normal npm dependency where you can update unilaterally. Anything we expose over the wire needs an explicit version-negotiation story.
4. **Don't try to vendor @agoric/swingset-vat.** It's 0.33.0 for a reason — it's evolving with the chain. If we want SwingSet semantics, we have to track agoric-sdk's chain-upgrade cadence, and that's a heavy ongoing tax. Either use Endo's smaller pieces or re-implement the kernel-level pieces against our own (WASM-Component) substrate.
5. **Quarterly pin review minimum.** Endo's release pace is fast enough that a year-old pin will be 5–10 minor versions behind. For a runtime project we need a recurring "bump and validate" rhythm.

## Sources

- npm registry queries (verified 2026-05-09 via `https://registry.npmjs.org/<pkg>`):
  - [`ses`](https://registry.npmjs.org/ses) — 2.0.0
  - [`@endo/lockdown`](https://registry.npmjs.org/@endo/lockdown) — 1.0.19
  - [`@endo/init`](https://registry.npmjs.org/@endo/init) — 1.1.13
  - [`@endo/marshal`](https://registry.npmjs.org/@endo/marshal) — 1.9.1
  - [`@endo/captp`](https://registry.npmjs.org/@endo/captp) — 4.5.0
  - [`@endo/eventual-send`](https://registry.npmjs.org/@endo/eventual-send) — 1.5.0
  - [`@endo/far`](https://registry.npmjs.org/@endo/far) — 1.1.14
  - [`@endo/exo`](https://registry.npmjs.org/@endo/exo) — 1.7.0
  - [`@endo/ses-ava`](https://registry.npmjs.org/@endo/ses-ava) — 1.4.1
  - [`@endo/compartment-mapper`](https://registry.npmjs.org/@endo/compartment-mapper) — 2.1.0
  - [`@endo/bundle-source`](https://registry.npmjs.org/@endo/bundle-source) — 4.3.0
  - [`@endo/ocapn`](https://registry.npmjs.org/@endo/ocapn) — 1.0.0
  - [`@agoric/swingset-vat`](https://registry.npmjs.org/@agoric/swingset-vat) — 0.33.0
  - [`@agoric/zoe`](https://registry.npmjs.org/@agoric/zoe) — 0.27.0
  - [`@agoric/ertp`](https://registry.npmjs.org/@agoric/ertp) — 0.17.0
  - [`@agoric/cosmic-swingset`](https://registry.npmjs.org/@agoric/cosmic-swingset) — 0.42.0
  - [`agoric`](https://registry.npmjs.org/agoric) — 0.22.0
- [agoric-sdk releases (verified via `gh api repos/Agoric/agoric-sdk/releases`)](https://github.com/Agoric/agoric-sdk/releases)
- [endo releases](https://github.com/endojs/endo/releases)
- [endojs/endo monorepo](https://github.com/endojs/endo)
- [Agoric/agoric-sdk monorepo](https://github.com/Agoric/agoric-sdk)
- [MetaMask Snaps `package.json` (ses ^1.15.0 pin)](https://github.com/MetaMask/snaps/blob/main/packages/snaps-execution-environments/package.json)
