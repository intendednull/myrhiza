//! Iroh-backed multi-peer test harness for kernel-tier acceptance
//! tests. Mirrors the shape of `InProcessHarness` (MemNetwork) but
//! wires `Runtime::start` to a real `IrohNetwork` over loopback UDP
//! via a shared `iroh::address_lookup::MemoryLookup`.
//!
//! Per docs/specs/2026-05-22-e2e-test-coverage-design.md §3.2.

#![cfg(feature = "network-iroh")]
