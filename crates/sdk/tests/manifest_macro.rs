//! Tests for the `manifest!` declarative macro.

use myrhiza_sdk::prelude::*;

#[test]
fn manifest_macro_matches_hand_built_three_component_manifest() {
    let macro_built = myrhiza_sdk::manifest! {
        app {
            name: "test-fixture",
            version: "0.1.0",
            description: "test",
            author_class: third_party,
        }
        abi {
            kernel_major: 1,
            kernel_minor_min: 0,
            state_digest_format: bincode13,
        }
        capabilities {
            deterministic_helpers: ["host.hash", "host.log"],
        }
        components {
            state_apply: "components/state-apply.wasm",
            state_propose: "components/state-propose.wasm",
            interaction: "components/interaction.wasm",
        }
    };

    // Verify expected field values
    assert_eq!(macro_built.app.name, "test-fixture");
    assert_eq!(macro_built.app.version, "0.1.0");
    assert_eq!(
        macro_built.app.author_identity_class,
        AuthorIdentityClass::ThirdParty
    );
    assert_eq!(
        macro_built.abi.state_digest_format,
        StateDigestFormat::Bincode13
    );
    assert_eq!(macro_built.capabilities.deterministic_helpers.len(), 2);
    assert!(
        macro_built
            .capabilities
            .deterministic_helpers
            .contains_key("host.hash")
    );
    assert!(
        macro_built
            .capabilities
            .deterministic_helpers
            .contains_key("host.log")
    );
    assert_eq!(
        macro_built.components.state_apply.as_deref(),
        Some("components/state-apply.wasm")
    );
    assert_eq!(macro_built.components.behavior, None);
}

#[test]
fn manifest_macro_canonicalize_is_idempotent() {
    let mut m = myrhiza_sdk::manifest! {
        app { name: "x", version: "1", description: "y", author_class: third_party, }
        abi { kernel_major: 1, kernel_minor_min: 0, state_digest_format: bincode13, }
        capabilities { }
        components { state_apply: "s.wasm", }
    };
    let first = m.clone();
    m.canonicalize();
    assert_eq!(
        m, first,
        "canonicalize should be idempotent after macro expansion"
    );
}

#[test]
fn manifest_macro_optional_sections_default_to_empty_or_none() {
    let m = myrhiza_sdk::manifest! {
        app { name: "minimal", version: "1", description: "y", author_class: third_party, }
        abi { kernel_major: 1, kernel_minor_min: 0, state_digest_format: bincode13, }
        capabilities { }
        components { state_apply: "s.wasm", }
    };
    assert!(m.capabilities.deterministic_helpers.is_empty());
    assert!(m.capabilities.host_imports.is_empty());
    assert_eq!(m.components.state_propose, None);
    assert_eq!(m.components.interaction, None);
    assert_eq!(m.components.behavior, None);
}
