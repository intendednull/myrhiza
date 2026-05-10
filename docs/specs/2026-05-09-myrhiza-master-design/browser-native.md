**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Browser and native


## 14. Browser and native: dual-stack at v1

### 14.1 Decision

Both Wasmtime native and jco browser ship at v1. Kernel internals
abstract over a stable internal trait that both backends satisfy.

### 14.2 Wasmtime native

Default backend on macOS / Linux / Windows. Mobile (iOS / Android)
uses Wasmtime's AOT path (cranelift + winch baseline compiler) since
iOS prohibits JIT.

WebView desktop wrappers (Tauri, Wails) embed the native kernel
binary, not the browser kernel. This preserves native iroh transport
in the desktop app.

**Wasmtime version pin**: v1 commits to **Wasmtime LTS** (the next
LTS release available at v1 ship time, expected to be v48 at end-of-
2026 per Wasmtime's 12-month LTS cadence). **Bumping Wasmtime LTS
is a kernel MAJOR version bump**, not minor — fuel-cost-table
shifts between Wasmtime majors are convergence-breaking per [distribution.md](distribution.md) §10.2's
ABI versioning rule (deterministic-helper additions are major;
fuel-cost recalibration falls in the same convergence-breaking
class). LTS is mandatory because:

- **Cross-peer fuel determinism requires identical fuel-cost tables.**
  Cranelift's per-instruction fuel costs may shift between Wasmtime
  majors (`prior-art/wasm-component-model/wasmtime.md`). Two peers
  on different Wasmtime versions can produce different fuel exhaustion
  outcomes for the same event, causing convergence divergence at the
  fuel boundary.
- **LTS provides 12+ months of stability** before forced bump, matching
  Myrhiza's release cadence.
- **Bumping Wasmtime LTS is a kernel MAJOR version bump** (consistent
  with [distribution.md](distribution.md) §10.2 ABI versioning rule for convergence-breaking changes).
  Apps re-publish bundles built against the new kernel major; older
  kernels cannot interoperate with newer-major topics. ABI advisory
  alone is insufficient because fuel-cost-table shifts are
  convergence-breaking, not merely API-compat-breaking.

Mid-cadence Wasmtime majors (non-LTS) MAY be supported by the kernel
build but are not the canonical fuel-determinism reference. Operators
running mixed Wasmtime versions accept the convergence-divergence
risk; the canonical reference is LTS.

**Apps cannot interoperate across kernel-major boundaries.** App
`manifest.toml` declares `kernel-major`; topic IDs include the
kernel-major in the `app_bundle_hash` derivation, so peers running
different kernel-majors cannot subscribe to the same topic.
Kernel-major-bump rollouts therefore split the network — apps must
re-publish with the new kernel-major, and users must update kernels
before re-joining.

### 14.3 jco browser

Browser path. jco preview2 is the v1 target; preview3 when stable
migrates in-place (no API churn for app authors).

Constraints:

- Sync ABI only at preview2. Submit-and-poll ([abi.md](abi.md) §8.5) is the workaround.
- ~350KB JS shim floor accepted as the cost of browser parity.
- Browser peers use iroh-relay-bridged QUIC for connectivity.

### 14.4 Why dual-stack at v1

- Browser is the project's pitch surface. Native-only-v1 undersells.
- "v1.5 fast-follow" framing for browser risks indefinite slip.
- Architecture pressure on backend abstraction is healthy from day
  one (avoids painful retrofit).
- Willow refactor onto Myrhiza targets v1; Willow is browser-shipped.

### 14.5 Native ≠ trusted-Rust apps

Critical clarification: "native" means the kernel runs as a native
Rust binary. Inside that kernel, **apps still run as WASM components
via Wasmtime**. The sandbox model requires WASM execution on every
backend. Compiling apps to native code for performance is explicitly
rejected — the only way to guarantee "WASM code can never access
more than what it's granted" is to run everything through the WASM
execution environment.

**Performance trade accepted, honestly**:

- **Steady-state straight-line numeric code**: ~2–5% Wasmtime overhead
  vs native code. The headline figure.
- **Hot-path state-apply with frequent host-import crossings** (sig
  verify, hash, payload-MAC verify): ~5–15% overhead. Host-call ABI
  translation costs dominate over WASM execution costs.
- **Cold component instantiation**: ms-class on Wasmtime, higher on
  jco. Aggressive caching (`Engine::precompile_component` +
  `InstancePre` reuse) is required; without it, per-event instantiation
  cost dominates everything else.

Sandbox is non-negotiable; this is the cost of the security model.
v1 commits to measuring overhead during MVP development and
documenting actual figures (rather than relying on the headline ~2-5%).


