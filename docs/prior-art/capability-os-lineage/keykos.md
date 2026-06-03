**Date:** 2026-05-29
**Status:** active
**Subject:** KeyKOS — pure capabilities + persistent single-level store + checkpoint (Hardy, Tymshare/Key Logic)

# KeyKOS

KeyKOS is the genesis of this lineage: a **pure** capability operating system —
*all* authority in the system is held in capabilities ("keys"), down to the
granularity of individual pages — built around a tiny nanokernel and an
**orthogonally persistent single-level store**. Norman Hardy was its prime
motivator. It is historical (it ran on IBM System/370 mainframes) but
load-bearing: every later idea in this folder is a reaction to KeyKOS.

## Provenance and dates

- Development began in the **mid-1970s at Tymshare, Inc.** under the name
  **GNOSIS** ("Great New Operating System In the Sky").
- **1984:** McDonnell Douglas acquired Tymshare. A year later it spun off
  **Key Logic**, which took GNOSIS and renamed it **KeyKOS**.
- In production use providing the Tymnet hosts' security, reliability, and
  24-hour availability; reported in production from **1983**.
- Key Logic closed in **1991**, which is what prompted the EROS clean-room
  reconstruction (see [eros.md](eros.md)).

Norman Hardy was senior architect at Key Logic and senior scientist at
Tymshare/McDonnell Douglas. He authored *The Confused Deputy* (1988, see
[capability-model.md](capability-model.md)) and later worked alongside Mark
Miller at Agorics — the bridge from this OS lineage into the language-side ocap
work ([spritely-ocapn](../spritely-ocapn/README.md),
[agoric-endo](../agoric-endo/README.md)). Per Mark Miller's March 2023 account,
Hardy chose cryonic suspension via California's End of Life Option Act in 2018;
the Foresight Institute's **Norm Hardy Prize** for usable security is named for
him.

## The three pillars

### 1. Keys are the only authority

A KeyKOS "key" is a capability. A *domain* (KeyKOS's process abstraction) holds
a fixed set of key slots; it can act only through the keys it holds. There is no
ambient authority and no global namespace — the radical version of the model in
[capability-model.md](capability-model.md). Keys come in kernel-defined types
(page keys, node keys, segment keys, start/resume keys for invoking other
domains).

### 2. Persistent single-level store

There is no distinction between "memory" and "disk files" from the program's
view. All objects — code, data, capability tables — live in one uniform,
persistent address space. The disk is not a filesystem; it is the backing store
of the entire object world. This is **orthogonal persistence**: persistence is a
property of the substrate, not something programs opt into with save/load calls.
(See [agoric-endo/persistence.md](../agoric-endo/persistence.md) for the
language-side revival of this idea.)

### 3. System-wide transparent checkpoint

KeyKOS periodically takes a **consistent system-wide checkpoint** of *all*
object state to disk — asynchronously, with no application cooperation and no
application-specific logic. After a crash (or power loss), the system restarts
from the last checkpoint as if nothing happened: every process resumes
mid-computation. Hardy's design treated a crash as equivalent to a very long
context switch. This is the property Mark Miller invoked when he described Hardy
himself as "checkpointed to non-volatile storage."

## The factory — confinement before EROS named it

KeyKOS's **factory** is the mechanism that lets a client run a service it does
not trust *and* lets a service author protect proprietary code from the client —
simultaneously. A factory can certify that the object it builds is **confined**:
it holds no keys that would let it leak data to a third party except those the
client explicitly approves (a "hole"). The client can *verify* confinement
before trusting the object with secrets. This is the ancestor of the EROS
**constructor** ([eros.md](eros.md)) and the conceptual root of the confinement
theory in [confinement-and-take-grant.md](confinement-and-take-grant.md).

## Why it matters to Myrhiza

KeyKOS is the existence proof that **capabilities + a kernel-as-broker can be
the *whole* security model of a real, production OS** — not a bolt-on. That is
the bet Myrhiza is making at the WASM-host boundary
([abi.md §8.4](../../specs/2026-05-09-myrhiza-master-design/abi.md), "the kernel
is the call broker"). The checkpoint/single-level-store idea is *adjacent* to
Myrhiza but not adopted wholesale: Myrhiza persists a per-author Merkle event
DAG and deterministic state, not an opaque whole-machine image — see
[lessons.md](lessons.md) for why the deterministic-replay model is the better
fit, and the storage-engine prior art for the on-disk substrate.

## Sources

- https://en.wikipedia.org/wiki/KeyKOS
- https://css.csail.mit.edu/6.566/2018/readings/keykos.pdf (Bomberger, Hardy et al., *The KeyKOS Nanokernel Architecture*)
- http://cap-lore.com/CapTheory/upenn/Checkpoint.html (Hardy, *The Checkpoint Mechanism in KeyKOS*)
- https://erights.medium.com/norm-hardys-place-in-history-cecf191df641 (Mark S. Miller, 2023-03-16)
- https://foresight.org/press/norm-hardy-prize-winners-2025/
