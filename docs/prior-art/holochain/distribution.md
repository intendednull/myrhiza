# Distribution & versioning

## hApp bundle format

A **hApp bundle** is a YAML manifest plus packed WASMs ([build/dnas](https://developer.holochain.org/build/dnas/), [hc CLI](https://blog.holochain.org/hc-cli-test-run-and-package-your-happ/)):

```
my-app.happ
 ├── happ.yaml                  (manifest_version, list of role -> dna)
 └── dnas/
     └── my-dna.dna
         ├── dna.yaml           (integrity + coordinator zome lists)
         └── *.wasm             (one per zome, can be bundled or fetched by URL)
```

DNAs reference WASMs by `bundled:` (embedded), `path:` (filesystem), or `url:` (fetched at install). DNA hash is a function of the integrity zomes' code + modifiers — change integrity, fork the network. Manifest version stayed at `'1'` through the 0.x line and reset to `'0'` (unstabilized) in 0.6 ([upgrade-holochain-0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)).

The role layer in the manifest (one DNA can be installed under multiple symbolic roles within the same hApp) is genuinely useful. Composition primitive worth borrowing.

## The version churn is the uncomfortable truth

Every minor release has been a breaking release for hApps:

| Upgrade | Breakage examples |
|---|---|
| [0.1 → 0.2](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.2/) | New entry definition macros, manifest format changes |
| [0.2 → 0.3](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.3/) | Conductor config restructure; mobile groundwork |
| [0.3 → 0.4](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.4) | More HDK signature changes |
| [0.4 → 0.5](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.5/) | Kitsune2 wire-incompatible; DPKI gated; bootstrap servers changed |
| [0.5 → 0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6) | DPKI removed entirely; default transport tx5 → iroh; `hash_blake2b` etc. removed; ChainFilter restructure; manifest format reset |

## No ABI stability story

HDK major numbers track core ([compatibility tables](https://developer.holochain.org/resources/compatibility/)); HDI is core+1. Apps are generally rebuilt for each release.

This is the cost of evolving the host without a typed ABI boundary — every change ripples through every guest. The HDK macros are tightly bound to internal data shapes; refactoring the host means breaking the macro contract; breaking the macro contract means every app re-compiles (and often re-codes).

Component Model with WIT gives Myrhiza the option of:

- **Versioned interfaces.** Multiple versions of the same interface coexist in one runtime. Apps written against `state-apply@1.0` keep working when the host adds `state-apply@1.1`.
- **Graceful coexistence.** Apps can declare which interface versions they support; the host links the latest compatible version.
- **Independent evolution of the host vs. the interface.** Host implementation changes don't force ABI changes.

## Implications for Myrhiza

- **Lock onto WIT-typed interfaces from day one.** Treat WIT files like a public API: semver, deprecation cycles, no silent breakages.
- **Stabilize the manifest format earlier than feels comfortable.** Holochain's manifest churned through every minor release. Apps + tooling depend on the manifest; treat it as a product surface from day 0, not a config detail.
- **Never break a wire format silently.** Kitsune2 broke wire compatibility with kitsune1 — that's a defensible decision but it forked all networks. Myrhiza's component identity (hash) plus an explicit interface-version dimension keeps wire breaks observable.

## Manifest schemas in detail

### `happ.yaml` (full schema)

Top-level fields ([build/happs](https://developer.holochain.org/build/happs/)):

| Field | Type | Default | Meaning |
|---|---|---|---|
| `manifest_version` | string | `'0'` (0.6+), was `'1'` through 0.5 | Schema version. Reset to `'0'` at 0.6 to mark unstabilized. |
| `name` | string | required | Human-readable, used in conductor admin panel. |
| `description` | string | `null` | Long description shown to operators. |
| `roles` | array | required | List of role objects (see below). |
| `allow_deferred_memproofs` | bool | `false` | Lets the hApp be installed without a membrane proof, supplied later. |

Each `roles[*]` entry:

| Field | Default | Meaning |
|---|---|---|
| `name` | required | The symbolic role name code refers to (`role_name` in client calls). |
| `provisioning.strategy` | `create` | Currently the only option; reserved for future "use existing cell" semantics. |
| `provisioning.deferred` | `false` | Currently the only option. |
| `dna.path` / `dna.bundled` / `dna.url` | one required | Where to fetch the DNA at install/package time. |
| `dna.modifiers` | optional | `network_seed`, `properties` (override at install). |
| `dna.installed_hash` | optional | If set, install fails on hash mismatch — pinning. |
| `dna.clone_limit` | `0` (u32) | Max clone cells creatable from this role. |

### `dna.yaml` (full schema)

| Field | Default | Meaning |
|---|---|---|
| `manifest_version` | `'0'` | Schema version. |
| `name` | required | Conductor-visible DNA name. |
| `integrity.network_seed` | `null` | Arbitrary string; salts the DNA hash to fork networks ([build/dnas](https://developer.holochain.org/build/dnas/)). |
| `integrity.properties` | `{}` | Application-specific constants readable by zomes. **Hashed into DNA hash** because validation logic can branch on them ([build/cloning](https://developer.holochain.org/build/cloning/)). |
| `integrity.zomes[*]` | required | `name`, `bundled`/`path`, optional `hash` for verification. |
| `coordinator.zomes[*]` | required | Same shape; can include `dependencies` (names of integrity zomes). |

`origin_time` and `quantum_time` were DNA modifiers in 0.4 — they parameterized the [quantised gossip](https://blog.holochain.org/quantised-gossip-optional-countersigners/) windows. **Removed in 0.5** (Kitsune2 dropped them).

## Modifiers as a forking primitive

The DNA hash is `H(integrity_zome_wasms || network_seed || properties)`. Three implications:

- **Network seed.** Same code, different network. Used by [Relay](https://blog.holochain.org/happs-spotlight-relay/) to spawn one private chat per group: each group gets a fresh UUIDv4 seed at clone-creation time. Pattern in the docs: `client.createCloneCell({ modifiers: { network_seed: crypto.randomUUID() }, role_name: "chat" })`.
- **Properties.** Compile-time-ish constants injected into integrity zomes — change one and you've forked the network and invalidated all prior data, because validators on the new network reject anything written under the old `properties`. Documented gotcha: "you will find yourself in the same network as any others who happened to create a clone with the same properties" — properties are *not* a privacy mechanism.
- **Cloning.** Combines both. `clone_limit` in `happ.yaml` caps how many parallel networks one role can spawn from a single hApp install.

## Bundle signing & source verification

Holochain has **no native bundle signing**. The `.happ` and `.webhapp` formats are unsigned tarballs. What exists:

- **Per-platform OS code-signing** is the developer's responsibility for desktop wrappers ([distribute-your-app](https://developer.holochain.org/get-started/distribute-your-app/)) — Apple Developer ID for macOS, Authenticode for Windows. The kangaroo-electron template has CI scaffolding for this.
- **Optional `installed_hash` pinning** in `happ.yaml`'s role definitions — the conductor refuses to install if the bundled DNA's hash doesn't match. This prevents a swapped DNA at install time, but doesn't bind the bundle to an author identity.
- **Launcher 'Verified' badge.** The (now-deprecated) Holochain Launcher hand-curated a list of "verified" hApps; the team vetted submissions manually with no programmatic chain-of-custody. [Source verification](https://hackmd.io/@hololtd/S1c3sipEq) was acknowledged as an unsolved problem.
- **Lair keystore** signs zome calls (chain entries are signed by the agent), but that's runtime signing of source-chain actions, not bundle provenance.

This is a real gap. Implication for Myrhiza: **a typed component identity (component-hash + signed manifest by author key) must be wired in from day one** — retrofitting authenticity onto a tarball-and-trust-the-source model is what Holochain has put off for seven years.

## DevHub / app-store status

DevHub was a hApp-as-package-manager: a DNA storing manifests and binaries on its own DHT, with a peer-host model so first-time installers didn't have to sync the whole catalog ([cell-cloning-milestone](https://blog.holochain.org/cell-cloning-milestone/)). It shipped with the Launcher and powered the in-app store.

**Current state: DevHub is effectively shelved.** The Holochain Launcher is no longer actively maintained as of v0.400.0 (March 2025), and the team [explicitly recommends](https://github.com/holochain/launcher) developers ship standalone Electron/Tauri builds via [kangaroo-electron](https://github.com/holochain-apps/kangaroo-electron) or the [p2p Shipyard](https://darksoil.studio/p2p-shipyard/). Reasoning: the App Store relied on a large DHT to distribute hApps and Holochain's scalability isn't there yet. Discovery has reverted to GitHub releases pages and per-app websites.

## Version compatibility — the matrix

From [resources/compatibility](https://developer.holochain.org/resources/compatibility/):

| Component | Major version relationship to Holochain core |
|---|---|
| `hdk` | Same major (core 0.6 → hdk 0.6.x) |
| `hdi` | Core major +1 (core 0.6 → hdi 0.7.x) |
| `hc` CLI / `hcterm` | Same major as core |
| `scaffolding`, `hc-spin`, playground (0.5+) | Major = core × 100 (core 0.6 → 0.600.x) |
| `@holochain/client` (JS), Rust client, lair-keystore, tryorama | Advance one major per core release series, by convention |

The "× 100" trick for tooling is a kludge to give independent semver to tooling without colliding with core's 0.x. Also a tell — they ran out of major-version namespace within a 0.x line and had to invent a sub-numbering convention.

## What teams actually did for 0.4 → 0.5

The breakage list ([upgrade-0.5](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.5/)):

- Kitsune2 wire protocol — 0.4 conductors and 0.5 conductors **cannot talk to each other**. Networks fork at the runtime version line.
- `origin_time` and `quantum_time` removed from `dna.yaml`.
- `hc run-local-services` replaced by a separate `kitsune2-bootstrap-srv` binary.
- DPKI gated behind `unstable-dpki` feature flag; `agent_latest_pubkey` requires it. Most apps switched to `agent_initial_pubkey`.
- DNA lineage moved behind `unstable-migration`.
- `HoloHash::from_raw_39_panicky` renamed; timestamp imports moved from `kitsune_p2p_timestamp` to `holochain_timestamp`.
- JS client enums changed serialization shape from `{VariantName: data}` to `{type: "variant_name", value: data}` — touches every UI codebase.
- `cap_secret: null` no longer accepted in `callZome`.

Then **0.5 → 0.6** ([upgrade-0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)) removed DPKI entirely, swapped tx5 → iroh as default transport, removed `hash_blake2b`, restructured ChainFilter, and reset the manifest version to `'0'`. The pattern: every minor release is a re-port. Teams responded by pinning to a single minor (Acorn stayed on 0.3 for a long stretch; hREA tracks releases; Volla apps shipped against 0.4 and have not yet been confirmed on 0.6).

## Sources

- [Build Guide — DNAs](https://developer.holochain.org/build/dnas/)
- [Build Guide — hApps](https://developer.holochain.org/build/happs/)
- [Build Guide — Cloning](https://developer.holochain.org/build/cloning/)
- [hc CLI: Test, Run, Package](https://blog.holochain.org/hc-cli-test-run-and-package-your-happ/)
- [Cell cloning milestone (DevHub)](https://blog.holochain.org/cell-cloning-milestone/)
- [Distribute your app](https://developer.holochain.org/get-started/distribute-your-app/)
- [Quantised gossip dev pulse](https://blog.holochain.org/quantised-gossip-optional-countersigners/)
- [Launcher repo (status)](https://github.com/holochain/launcher)
- [Holochain security review](https://hackmd.io/@hololtd/S1c3sipEq)
- [Upgrade 0.1→0.2](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.2/)
- [Upgrade 0.2→0.3](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.3/)
- [Upgrade 0.3→0.4](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.4)
- [Upgrade 0.4→0.5](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.5/)
- [Upgrade 0.5→0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
- [Compatibility tables](https://developer.holochain.org/resources/compatibility/)
