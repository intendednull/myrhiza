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
| [0.5 → 0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6) | DPKI removed entirely; iroh added as transport (made default in 0.6.1-rc); `hash_blake2b` etc. removed; ChainFilter restructure; manifest format reset |

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

## Bundle binary format

The on-disk `.happ`, `.dna`, `.webhapp`, and `.coordinators` files are all the same shape: **a gzip-compressed, MessagePack-serialized `mr_bundle::Bundle<M>` struct**, where `M` is one of the manifest types ([`AppManifest`](https://docs.rs/holochain_types/latest/holochain_types/app/struct.AppManifest.html), `DnaManifest`, `WebAppManifest`, `CoordinatorManifest`).

There are **no magic bytes, no header, and no envelope** — just gzip-then-msgpack-then-Rust-struct. From [`mr_bundle/src/pack.rs`](https://github.com/holochain/holochain/blob/develop/crates/mr_bundle/src/pack.rs):

```rust
pub fn pack<T: Serialize>(data: &T) -> Result<Bytes> {
    let bytes = rmp_serde::to_vec_named(data)?;          // msgpack with field names
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&bytes)?;
    Ok(enc.finish()?.into())
}
```

The `Bundle<M>` struct itself has only two fields: `manifest: M` and `resources: BTreeMap<ResourceIdentifier, ResourceBytes>`. Resources are stored **inline as raw byte arrays** keyed by an opaque resource id (a hash-derived identifier the manifest generates from each declared zome path). At unpack time the deserializer rebuilds the BTreeMap and the manifest is responsible for reconciling resource IDs back to logical zome names.

**The file extension is purely a marker.** `mr_bundle` records a `bundle_extension()` per manifest impl (`"happ"`, `"dna"`, `"webhapp"`, `"coordinators"`) and uses it only when writing to disk; the wire format is identical, and the conductor identifies the bundle type by which manifest variant deserializes successfully, not by the file extension. Renaming `foo.happ` to `foo.dna` produces a deserialization error, not a content-type error.

**Bundles are not seekable or streamable.** `unpack` reads the entire gzip stream into a `Vec<u8>` before handing it to `rmp_serde::from_slice`, and msgpack itself requires the full byte range to decode the outer struct. Webhapps embed the UI as a tarball-of-assets resource bytes alongside the `.happ` resource bytes — same nested structure, same all-or-nothing decode.

Implication: **the format is opaque to anything but a Rust client**, has no integrity envelope (no signature, no checksum slot), and forces the entire bundle into RAM at install time. For Myrhiza, a content-addressed component-archive format with a stable header (magic + version + integrity field) and per-resource offsets would be a strict upgrade.

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

Then **0.5 → 0.6** ([upgrade-0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)) removed DPKI entirely, added iroh as transport (made default in the 0.6.1-rc line), removed `hash_blake2b`, restructured ChainFilter, and reset the manifest version to `'0'`. The pattern: every minor release is a re-port. Teams responded by pinning to a single minor (Acorn stayed on 0.3 for a long stretch; hREA tracks releases; Volla apps shipped against 0.4 and have not yet been confirmed on 0.6).

## Coordinator hot-swap via `UpdateCoordinators`

Since the integrity/coordinator split landed in 0.0.144, coordinator zomes can be replaced on a running cell without forking the network or invalidating any source-chain data. The mechanism is the [`AdminRequest::UpdateCoordinators`](https://docs.rs/holochain_conductor_api/latest/holochain_conductor_api/enum.AdminRequest.html) admin call, payload-typed as `UpdateCoordinatorsPayload { cell_id: CellId, source: CoordinatorSource }` where `source` is either an inlined `CoordinatorBundle` (a `mr_bundle::Bundle<CoordinatorManifest>`) or a path to one. The 0.5+ JS client takes a `CellId` rather than a `DnaHash` so a single DNA installed under multiple cells can be updated independently.

### What actually happens

From [`crates/holochain/src/conductor/conductor.rs`](https://github.com/holochain/holochain/blob/develop/crates/holochain/src/conductor) (`update_coordinators`):

1. Look up the cell's `RealRibosome` in the in-memory `RibosomeStore`.
2. Mutate its `DnaFile` in place: for each incoming zome, **replace any existing coordinator zome with the same name; append if the name is new**. Integrity zomes are untouched.
3. Persist the new WASM bytes and the updated `DnaDef` to the conductor's databases.
4. Re-insert the ribosome into the store.

The DNA hash **does not change** because integrity zomes (the only inputs to the hash, along with `network_seed` and `properties`) are untouched. The cell keeps its identity, source chain, and DHT ops.

### What the docs are silent on

- **No state migration hook.** No `migrate(old_zomes -> new_zomes)` callback. Coordinator code that introduces new entry types must use integrity zomes (which forks the DNA); coordinator code is, by construction, *not allowed to define new entry types*. So "schema migration" is not a coordinator-update concern by design — but a coordinator that read links by tag and now reads them by typed-link will silently see old data through the new code path. That's the app's problem.
- **In-flight calls.** The implementation comment is candid: *"Note this isn't really concurrent safe. It would be a race condition to update the same DNA concurrently."* No documented behavior for zome calls that arrive mid-swap.
- **Clients.** The admin API returns `CoordinatorsUpdated` once the ribosome is replaced. Existing app-websocket connections stay open; capability grants on the source chain are unchanged. **Clients do not need to reconnect**, but a client holding a `FunctionName` for a removed function will get a runtime error on next call.
- **Rollback.** No first-class rollback. To revert, call `UpdateCoordinators` again with the previous bundle. The old WASM bytes are not retained — if you didn't keep the bundle, the rollback is not recoverable from conductor state alone.
- **Init callbacks.** The `init` callback runs once on cell creation and is **not re-fired** by `UpdateCoordinators`. State that the new coordinator zome assumed `init` would establish must already exist (or the new code must self-bootstrap defensively).

A real and useful capability — Holochain ships hot-reload of business logic that VM-platforms like Ethereum do not — but the operational story is unfinished. For Myrhiza: a hot-swap primitive is desirable, but should ship with explicit *upgrade* and *downgrade* hooks the component declares, plus a versioned-state contract checked at swap time.

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
