//! Shadow regression test for the v1 normative resource caps.
//!
//! /// Covers: determinism.md §5.3, verification.md §22.4
//!
//! These literals re-state every constant in
//! crates/types/src/limits.rs. Editing only one side fails CI.
//! Bumping a constant requires editing both AND a kernel-major
//! version bump per distribution.md §10.2.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use myrhiza_types::limits::{
    COMPONENT_MEMORY_CAP_V1, DAG_DEPS_CAP_V1, EVENT_PAYLOAD_CAP_V1, HOST_HASH_FUEL_PER_BYTE,
    HOST_INSTALL_KEY_FUEL, HOST_LOG_FUEL_BASE, HOST_NOW_HLC_FROM_EVENT_FUEL,
    HOST_VERIFY_PAYLOAD_MAC_FUEL, HOST_VERIFY_SIGNATURE_FUEL, STATE_APPLY_FUEL_BUDGET_V1,
    STATE_PROPOSE_FUEL_BUDGET_V1,
};

#[test]
fn fuel_budgets_match_spec_v1() {
    assert_eq!(STATE_APPLY_FUEL_BUDGET_V1, 10_000_000);
    assert_eq!(STATE_PROPOSE_FUEL_BUDGET_V1, 50_000_000);
}

#[test]
fn resource_caps_match_spec_v1() {
    assert_eq!(COMPONENT_MEMORY_CAP_V1, 64 * 1024 * 1024);
    assert_eq!(EVENT_PAYLOAD_CAP_V1, 1024 * 1024);
    assert_eq!(DAG_DEPS_CAP_V1, 64);
}

#[test]
fn per_host_call_fuel_costs_match_spec_v1() {
    assert_eq!(HOST_HASH_FUEL_PER_BYTE, 5);
    assert_eq!(HOST_VERIFY_SIGNATURE_FUEL, 5_000);
    assert_eq!(HOST_VERIFY_PAYLOAD_MAC_FUEL, 1_000);
    assert_eq!(HOST_INSTALL_KEY_FUEL, 100);
    assert_eq!(HOST_NOW_HLC_FROM_EVENT_FUEL, 50);
    assert_eq!(HOST_LOG_FUEL_BASE, 100);
}
