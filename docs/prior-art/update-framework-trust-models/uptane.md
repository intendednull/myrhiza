**Date:** 2026-05-29
**Status:** active
**Subject:** Uptane — the ISO/IEEE-standardized automotive derivative of TUF. Two-repository (Image + Director) model, primary/secondary ECUs, and what its constrained-device + targeting design teaches Myrhiza.

# Uptane

Uptane is TUF adapted for **over-the-air software updates to ground vehicles** — the most safety-critical, most adversarial, most constrained deployment TUF has reached. It is worth a file here because (a) it is an *actual standard* (not just a CNCF project), proving the TUF model survives formalization, and (b) its two-repository split solves a problem Myrhiza also has: **how do you let a central party target specific updates at specific recipients without making that party able to forge arbitrary software?**

## Standardization status

- **IEEE-ISTO 6100.1.0.0: Uptane Standard for Design and Implementation**, the first release, came out **2019-07-31** (= Standard v1.0.0). The standard has since iterated through 1.0.1, 1.1.0, 1.2.0, and 2.0.0; the **current version is 2.1.0** (released 2023-06-23, "primarily clarity and wording improvements"). Stewardship moved to the Linux Foundation's Joint Development Foundation (from v1.1.0 onward). The two-repository model and role structure described below are stable across all versions.
- Sits alongside the automotive software-update governance standards: **ISO 24089:2023** (Road vehicles — Software update engineering) and **ISO/SAE 21434** (cybersecurity engineering). Uptane is the *technical* update-security framework; ISO 24089 is the *process* standard it slots under. (Uptane is referenced by, not identical to, these ISO standards.)

An update-security design that hardened into an IEEE/ISTO standard *and* underpins ISO-process-compliant automotive practice is evidence the TUF role model survives formalization and field deployment, not just academic publication.

## The two-repository model

Uptane's defining innovation over plain TUF is splitting the repository in two:

- **Image Repository** — the long-lived store of images and their TUF-style metadata (root/targets/snapshot/timestamp). Offline-key-signed, slow-moving, high-assurance. "What firmware *exists and is authentic*."
- **Director Repository** — generates per-vehicle metadata saying **which ECU should install which image** right now, validated against the Image Repository. Online, fast-moving, vehicle-specific. "What firmware *this vehicle should install*."

A client (vehicle) requires an update to be vouched for by **both** repositories. The Director can *target* an update at a specific vehicle, but it can only point at images the Image Repository already authenticated — so a compromised (online) Director **cannot introduce malicious firmware**, only mis-target authentic firmware. This is the same minimal-trust-in-online-keys principle as TUF's timestamp role, generalized to "the party that does the targeting is not the party that authenticates the artifact."

This split is directly relevant to Myrhiza: any future "the network operator endorses/targets this module at this peer group" feature (the two-signer composition noted in [`app-distribution/supply-chain.md`](../app-distribution/supply-chain.md) §implications) should keep the *targeter* and the *authenticator* as separate trust roots, exactly as Uptane does — not collapse them into one key.

## Primary and secondary ECUs

Vehicles contain many ECUs of wildly varying capability. Uptane defines:

- **Primary ECU** — has network access; downloads and verifies metadata + images for itself and for the secondaries it serves; assembles the **vehicle manifest** (what is installed where) and sends it to the Director.
- **Secondary ECU** — constrained; verifies metadata/images received *from its primary*, reports a signed ECU manifest upward.

Uptane defines **full verification** (a secondary does the complete TUF check) and **partial verification** (a very constrained secondary checks only a reduced metadata set, trusting its primary for the rest). This graceful degradation for constrained verifiers is a useful pattern for Myrhiza's heterogeneity: a browser peer (jco, see [`jco/`](../jco/)) and a native desktop kernel are *not* equally capable verifiers, and a tiered "full vs partial verification" posture is a cleaner answer than "every peer does identical work or none."

## Time as an attack surface

Uptane treats **secure time** as a first-class problem: a vehicle that has been offline or had its clock attacked cannot trust expiration-based freshness. It defines a time-server / nonce mechanism so a constrained ECU can establish current time before honoring metadata expiry. This is the freeze defense from [`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md) hardened against clock manipulation — relevant to Myrhiza's "potentially stale" warning (`distribution.md` §10.7), which implicitly trusts the local clock.

## Implications for Myrhiza

- **Validates** the role-separation thesis under standardization pressure: the design survived becoming an IEEE/ISTO standard and ISO-process anchor.
- **Borrow** the Image/Director split as the shape for any "operator targets a module at a peer cohort" feature — keep authenticate-the-artifact and target-the-recipient as separate roots.
- **Borrow** the full-vs-partial verification tiering for heterogeneous peers (browser vs native).
- **Borrow** the explicit secure-time mechanism — Myrhiza's freshness story currently leans on an unguarded local clock.
- **Avoid** importing the centralized Director shape itself: it is a server, which §10.8 forbids. Take the *separation*, not the *topology*. See [`lessons.md`](./lessons.md).

## Sources

- Uptane Standard 2.1.0 (current): <https://uptane.org/docs/2.1.0/standard/uptane-standard>
- Uptane Standard version history: <https://github.com/uptane/uptane-standard/releases>, <https://uptane.org/docs/latest/all-versions>
- Uptane Standard first release (IEEE-ISTO 6100.1.0.0, 2019): <https://uptane.org/papers/ieee-isto-6100.1.0.0.uptane-standard.html>
- Uptane first whitepaper: <https://uptane.org/papers/Uptane_first_whitepaper.pdf>
- Uptane deployment best practices 2.1.0: <https://uptane.org/docs/2.1.0/deployment/best-practices>
- ISO 24089:2023: <https://www.iso.org/standard/77796.html>
- Uptane regulations & standards: <https://github.com/uptane/deployment-considerations/blob/master/regulations_and_standards.md>
