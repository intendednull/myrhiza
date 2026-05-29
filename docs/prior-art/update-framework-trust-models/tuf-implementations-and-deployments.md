**Date:** 2026-05-29
**Status:** active
**Subject:** TUF in the wild — python-tuf, go-tuf, tough (Rust); and the production deployments: PyPI, Sigstore's own root of trust, RustSec, Bottlerocket.

# TUF implementations and deployments

TUF is not a paper that never shipped. It is CNCF-Graduated (2019-12-18, the **first** CNCF spec project to graduate) and runs in several systems Myrhiza already touches or resembles. This file records the implementation surface and the deployments — both as evidence the design is real and as concrete reference points for a Rust runtime.

## Reference implementations

| Impl | Lang | Stewardship | Status / notes |
|---|---|---|---|
| **python-tuf** | Python | CNCF / Linux Foundation | The reference implementation. Latest **7.0.0** (2026-05-18), Apache-2.0 OR MIT. Redesigned ~2 years ago around a clean Metadata API. |
| **go-tuf** | Go | theupdateframework | Legacy `v0.7.0` is **deprecated** for being hard to maintain; **go-tuf/v2** (originally `rdimitrov/go-tuf-metadata`, modeled on python-tuf's redesign) is the maintained line. |
| **tough** | Rust | AWS `awslabs/tough` | Client lib + `tuftool` CLI, written for **Bottlerocket**'s update system in 2019. Multiple signing backends (local files, AWS KMS, AWS SSM). The Rust-ecosystem reference. |
| **rust-tuf** | Rust | theupdateframework | Separate Rust impl (the `tuf` crate); less active than `tough` for production use. |

For Myrhiza (a Rust runtime), **`tough` is the closest reference** — it is the only production-grade Rust TUF client, and its KMS/SSM backend abstraction is a useful model even if Myrhiza never embeds TUF wholesale. The python-tuf Metadata API redesign is the cleanest *data-model* reference (the go-tuf/v2 effort explicitly copied it).

## Deployments

### PyPI (Python Package Index)

PyPI is the flagship motivating case. **PEP 458** ("Secure PyPI downloads with signed repository metadata") specifies a TUF *online-only* profile where PyPI itself holds the snapshot/timestamp (online) keys and signs repository metadata — defending against a compromised PyPI mirror/CDN. **PEP 480** ("Surviving a Compromise of PyPI") extends this with **delegated developer signing** so an attacker who owns PyPI's online keys still cannot forge a *package* without the developer's offline key. PEP 458 implementation has been a long-running Warehouse effort (Trail of Bits / VMware involvement); PEP 480 is the further, end-to-end goal. The PEP titles themselves — "surviving a compromise" — are the survivable-key-compromise thesis in deployment.

### Sigstore's own root of trust

This one is the sharpest cross-link. Myrhiza's [`app-distribution/signing.md`](../app-distribution/signing.md) covers Sigstore's *signing* surface (Cosign/Fulcio/Rekor) but does **not** mention that **Sigstore distributes its own trust root via TUF**. The `sigstore/root-signing` repository is a TUF repository, established in a **public root-key signing ceremony** (live-streamed, June 18 2021) with **five keyholders** from different companies and academic institutions — Luke Hinds and Bob Callaway (Red Hat), Marina Moore (NYU), Santiago Torres-Arias (Purdue), and Dan Lorenc (Google) — minting the initial `root.json` under a **3-of-5 threshold** signature (at least 3 of the 5 must sign for the root to validate). This is the M-of-N threshold idea from [`tuf-roles-and-metadata.md`](./tuf-roles-and-metadata.md) made concrete: losing up to 2 keys leaks nothing, and the remaining holders can still rotate. Clients fetch the Sigstore `trusted_root.json` over a TUF repo at `tuf-repo-cdn.sigstore.dev`. So even the keyless-signing poster child sits **on top of** TUF's role-separation + threshold model for its own bootstrap. The metadata can be generated with the python-tuf CLI, the go-tuf CLI, or the Sigstore ceremony tooling (hardware tokens). This is direct evidence that the trust-model layer in *this* folder sits above the mechanics layer in `app-distribution/` — Sigstore itself stacks them that way.

### RustSec advisory database

The **RustSec** advisory DB (`rustsec/advisory-db`, maintained by the Rust Secure Code WG) is the security-advisory source for crates published via crates.io. It is frequently cited as a TUF-adjacent distribution concern; note that the *advisory data* distribution is the relevant surface, not crates.io's own signing. Myrhiza-relevant as the Rust ecosystem's vulnerability channel — a peer that wants "is this module known-bad?" looks here. (Verify the exact TUF integration state before citing it as a deployed TUF repository; RustSec's primary distribution is a git repo, and TUF coverage of crates.io has been proposed more than shipped — flagged in [`open-problems.md`](./open-problems.md) §corpus-drift.)

### Bottlerocket

**Bottlerocket** (AWS's container-host Linux OS) ships OS updates through TUF repositories verified by `tough`, with role-based metadata and enforced expiration. The `updog` update client uses `tough` to verify metadata and download signed OS images. Bottlerocket is the cleanest "TUF in a Rust update client, end to end" production example and the reason `tough` exists.

## What the deployments tell Myrhiza

1. **TUF is the boring, vetted answer** for "trustworthy update channel rooted in offline keys" — the exact problem `distribution.md` §10.9–10.10 solves by hand.
2. **A Rust TUF client exists and is production-proven** (`tough` / Bottlerocket). Myrhiza is not blocked on tooling if it ever wants to adopt TUF semantics.
3. **Every one of these deployments assumes a served repository the client polls.** PyPI, Sigstore's CDN, Bottlerocket's update server — all server-shaped. That is precisely what Myrhiza's §10.8 forbids. So the lesson is *the role/threshold/version model*, not *the deployment*. See [`lessons.md`](./lessons.md) and the P2P tension in [`transparency-logs.md`](./transparency-logs.md).

## Sources

- python-tuf: <https://pypi.org/project/tuf/>, <https://github.com/theupdateframework/python-tuf>
- go-tuf (v2 migration): <https://github.com/theupdateframework/go-tuf>
- tough: <https://github.com/awslabs/tough>
- Bottlerocket TUF (re:Invent OPN401): <https://d1.awsstatic.com/events/reinvent/2020/Securing_Bottlerocket_updates_with_TUF_and_Rust_OPN401.pdf>
- PEP 458: <https://peps.python.org/pep-0458/>
- PEP 480: <https://peps.python.org/pep-0480/>
- Sigstore root-signing: <https://github.com/sigstore/root-signing>, <https://docs.sigstore.dev/about/security/>
- Sigstore root ceremony (3-of-5, June 18 2021, keyholders): <https://www.cncf.io/blog/2021/06/16/a-new-kind-of-trust-root/>, <https://blog.sigstore.dev/a-new-kind-of-trust-root-f11eeeed92ef/>
- RustSec advisory DB: <https://github.com/rustsec/advisory-db>, <https://rustsec.org/>
- CNCF TUF graduation: <https://www.cncf.io/projects/the-update-framework-tuf/>
