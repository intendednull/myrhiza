**Date:** 2026-05-29
**Status:** active
**Subject:** Reproducible builds — the bit-for-bit "is this binary really from that source" leg that closes the gap SLSA provenance leaves. Status, mechanics, and the Myrhiza fit (kernel binary + WASM modules).

# Reproducible builds

A **reproducible build** produces **bit-for-bit identical** output binaries every time the same source is compiled under the same declared conditions — not "similar," not "functionally equivalent," *identical*. The payoff: **anyone** (a researcher, a distributor, an enterprise, a suspicious peer) can rebuild a published artifact from source and confirm the published bytes match. It closes the gap SLSA provenance leaves open — SLSA attests *a* build happened on *a* platform; reproducibility lets a third party *independently re-run* the build and check it, removing the need to trust the build platform at all.

`distribution.md` §10.10 already names reproducible builds as a v1 kernel-distribution channel ("kernel source is open; users may build from source and compare against published checksums") — so unlike transparency logs, this is a leg Myrhiza *already commits to*, not a v2 deferral.

## Status (verified 2026)

- The **Reproducible Builds** project (reproducible-builds.org) is the cross-distro umbrella effort; it defines the practices and tooling.
- **Debian** is the flagship: **Debian 14 ("Forky")** plans to **penalize reproducibility regressions** — newly-submitted and regressing packages will be blocked from testing if they fail to build reproducibly. The project's `reproduce.debian.net` runs **rebuilderd** verifying packages across release architectures (riscv64 nodes included).
- The hard problems are catalogued (the project has identified hundreds of distinct causes of non-reproducibility): timestamps, build paths, locale, filesystem ordering, parallelism non-determinism, embedded build-host metadata.

A major distro moving reproducibility from "nice to have" to "gate for entry" indicates it is a deployed supply-chain practice, not aspirational.

## Mechanics (what makes a build reproducible)

The recurring fixes:

- **`SOURCE_DATE_EPOCH`** — a standardized env var pinning embedded timestamps to the source's last-modified time, so the same source yields the same timestamps.
- **Deterministic paths** — strip or normalize build-directory paths embedded in binaries.
- **Stable ordering** — sort filesystem listings, archive members, symbol tables; don't depend on inode order or hash-map iteration order.
- **Controlled toolchain + environment** — pin compiler version, flags, locale; record them as part of the declared build conditions.
- **`diffoscope`** — the project's tool for *explaining* why two builds differ, byte region by byte region.

## The Rust / WASM angle

- **Rust** builds can embed absolute paths (e.g. `$HOME/.cargo/...`) and build metadata; `--remap-path-prefix` and a pinned toolchain are the standard mitigations. Myrhiza's kernel is Rust, so the kernel-binary reproducibility story (§10.10 channel 3) lives here.
- **WASM components** are an *easier* reproducibility target than native binaries in some respects (no ASLR, no platform-specific linker quirks in the final `.wasm`) but inherit the source-toolchain's non-determinism (timestamps in custom sections, build-path leakage, `wasm-opt` version drift). A Myrhiza module author claiming "build from source and check the hash" must pin the full WASM toolchain, not just `rustc`.

## Why this is the right fit for Myrhiza (vs transparency logs)

Reproducible builds are the **one member of this folder that needs no server and no central log** — which is exactly why §10.8 can live with them while it defers transparency logs:

- Verification is **local and offline**: rebuild, compare hashes. No operator, no witness network, no `rekor.sigstore.dev` reachability.
- It composes cleanly with Myrhiza's **content-addressed** transport (iroh-blobs): the "published checksum" *is* the content address. If a peer rebuilds and gets a different hash, the artifact is by definition a different blob.
- It directly hardens the §10.10 acknowledged risk ("a sophisticated adversary controlling both the project's release infrastructure AND the OS package manager could distribute a compromised kernel") — reproducibility lets independent rebuilders catch exactly that, with no extra infrastructure.

The limit: reproducibility proves *binary matches source*; it does **not** prove the *source* is benign (that is review/provenance) nor that the *channel* is fresh (that is TUF). It is one leg of the tripod — pair it with SLSA provenance ([`in-toto-slsa-provenance.md`](./in-toto-slsa-provenance.md)) and TUF-style channel protection ([`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md)) for the full picture. See [`lessons.md`](./lessons.md).

## Sources

- Reproducible Builds project: <https://reproducible-builds.org/>
- reproduce.debian.net (rebuilderd): <https://reproduce.debian.net/>
- Debian ReproducibleBuilds wiki: <https://wiki.debian.org/ReproducibleBuilds>
- LWN history/status: <https://lwn.net/Articles/985739/>
- SOURCE_DATE_EPOCH spec: <https://reproducible-builds.org/docs/source-date-epoch/>
- diffoscope: <https://diffoscope.org/>
