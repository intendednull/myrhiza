**Date:** 2026-05-29
**Status:** active
**Subject:** Soroban / Stellar — the Wasmi-interpreter case for metering determinism; dual CPU + memory cost model

# Soroban / Stellar

Soroban is Stellar's smart-contract platform. It is the deliberate
**conservative** design point in this folder: it runs contracts on the **Wasmi
interpreter** (~13 KLOC) rather than any JIT, and it argues explicitly that an
interpreter is the *right* substrate for portable, deterministic metering. It is
Myrhiza's runner-up paradigm — the path Myrhiza does not take (Wasmtime/Cranelift
is the pinned codegen strategy,
[`determinism.md §5.2`](../../specs/2026-05-09-myrhiza-master-design/determinism.md))
— so it is worth understanding *why* a serious team chose the interpreter.

## "Why doesn't Soroban use a JIT?" — the argument, verbatim

Stellar's reasoning, quoted from their engineering blog:

- **Conservatism:** "We are conservative engineers – that's an important thing
  when building financial infrastructure trusted by some of the biggest financial
  services in the world."
- **Security surface of a JIT:** a JIT's larger codebase means "bugs in its
  implementation are significantly more likely to be critical (remote code
  execution – taking over a validator)," and JITs are exposed to **"JIT bombs"** —
  inputs that cause excessive compile time/memory.
- **Metering portability — the key point for this folder:** "Given the complex
  nature of JITs and unexpected optimization paths it's hard to ensure stability
  or portability of JIT-based metering." Because Wasmi is an interpreter, "we are
  essentially already paying enough per-WASM-instruction overhead that it was easy
  to modify the interpreter to count WASM instructions executed and charge for
  them."

The performance objection (interpreters are slow) is answered by an architectural
move Myrhiza shares: **push the inner loops into native host functions.** Quoting:
"WASM code drive the 'outer loops' of execution (which don't matter if they are
interpreted) and have host-function native code do all the 'inner loops'."
Myrhiza's deterministic helper set
([`determinism.md §5.1`](../../specs/2026-05-09-myrhiza-master-design/determinism.md))
is exactly this shape — hashing, signature verification, MAC verification all run
as native host calls, not in WASM.

## The dual cost model: CPU and memory, separately

Soroban meters **two** resources independently, each with its own budget:

- **CPU instructions** — metered during execution.
- **Memory (RAM)** — capped; per the docs, memory is "capped but not subject to
  any charge" (it bounds work but isn't a billed fee dimension).

Both use a **linear cost model**: `y = a + bx`, where `x` is the runtime input
size, `a` and `b` are a constant and a linear coefficient per **cost type**, and
"each component cost increases at most linearly (constant or linear) with respect
to its input." Crucially the parameters are **"calibrated and fitted offline
against inputs of various sizes."** Costs are expressed in **cost types** — "meta
instructions" representing host operations of known complexity — so both WASM
execution and host-function calls are accounted in equivalent CPU-instruction and
memory-byte terms. When consumption exceeds a limit, "execution is terminated"
and an error is produced.

### Borrow boundary — the dual meter and the linear-cost discipline

Myrhiza already has both axes: a fuel budget (CPU-analogue) **and** a 64 MB
memory cap per instance
([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md),
`Store::limiter`). Soroban validates that **metering CPU and memory as separate
budgets is the right structure** — a component can be CPU-cheap but memory-hungry
or vice versa, and one budget cannot substitute for the other. The
**linear-cost-model-per-cost-type with offline calibration** is also directly
borrowable methodology for the fuel-cost-table child spec: every host call's fuel
cost should be `a + b*input_size` (Myrhiza already does this for `host.hash` =
n*5 and `host.log` = 100+n; the constant-cost calls like `host.verify-signature`
= 5000 are the `b=0` case). Soroban is the prior art that says: do this for
*every* metered operation, and **calibrate `a` and `b` offline**, don't guess.

## Cost params are network/consensus settings

Soroban's cost-model parameters are "network configurable entries" that "can be
updated through network consensus." Like NEAR, the cost table is **not** a
hardcoded constant in client code — it is versioned network state changed by an
agreed protocol action (Stellar CAPs / network upgrades). Same discipline as
[near.md](near.md): **recalibration is a governed, versioned event.** (The Soroban
docs reviewed did not give specific protocol-version numbers; the
consensus-updatable nature is stated, the per-version mechanics less so than
NEAR's `RuntimeConfigStore`.)

## What Soroban validates / cautions for Myrhiza

- **Validates:** interpreter-based metering is the *most* portable option — no JIT
  optimization paths to diverge. Myrhiza's choice of Wasmtime (a JIT) is therefore
  the *less* conservative path, and the engine-config pins
  ([`determinism.md §5.2`](../../specs/2026-05-09-myrhiza-master-design/determinism.md):
  opt-level=Speed, strategy=Cranelift, all feature flags pinned) are precisely the
  work Soroban *avoided* by not using a JIT. The pins are load-bearing because of
  this choice.
- **Caution:** if the Cranelift-fuel-determinism story ever proves shaky across a
  Wasmtime bump, Wasmtime's **Pulley** interpreter is the Soroban-shaped fallback.
  The spec already names Pulley as an open question
  ([`../wasm-component-model/wasmtime.md`](../wasm-component-model/wasmtime.md)
  open-problems). Soroban is the existence proof that the interpreter path works
  in production for high-value workloads.

## Sources

- "Why Doesn't Soroban Use a JIT?" — https://stellar.org/blog/developers/why-doesnt-soroban-use-a-jit
- Soroban fees, resource limits & metering — https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering
- "How Soroban's Fee Structure Contributes to Scalability" — https://stellar.org/blog/developers/sorobans-fee-structure-contributes-stellar-network-scalability
- Wasmi project — https://github.com/wasmi-labs/wasmi
- Myrhiza spec: determinism.md §5.1/§5.2/§5.3
