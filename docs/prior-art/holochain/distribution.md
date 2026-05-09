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

## Sources

- [Build Guide — DNAs](https://developer.holochain.org/build/dnas/)
- [hc CLI: Test, Run, Package](https://blog.holochain.org/hc-cli-test-run-and-package-your-happ/)
- [Upgrade 0.1→0.2](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.2/)
- [Upgrade 0.2→0.3](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.3/)
- [Upgrade 0.3→0.4](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.4)
- [Upgrade 0.4→0.5](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.5/)
- [Upgrade 0.5→0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
- [Compatibility tables](https://developer.holochain.org/resources/compatibility/)
