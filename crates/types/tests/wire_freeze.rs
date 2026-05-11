//! Wire-frozen canonical bincode byte layout for plan-B-1 wire types.
//!
//! Per spec §6.2, this test pins the byte string each variant
//! produces under the v1 canonical bincode options chain. Any change
//! that mutates these bytes is a wire-incompatible change and MUST
//! be a kernel-major bump per browser-native.md §14.2.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use bincode::Options;
use myrhiza_types::{
    AuthorHead, AuthorPubkey, AuthorSeq, DriftAnchor, DriftMessage, DriftSignedPayload, EventHash,
    EventRequest, GenesisV1, HeadsRequest, HeadsSummary, PeerPubkey, canonical_bincode,
};

fn hex_dump(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(&mut out, "{b:02x}").expect("write to String");
    }
    out
}

#[test]
fn genesis_v1_wire_layout() {
    let g = GenesisV1 {
        seed: [0x11; 32],
        founder_pubkey: AuthorPubkey::from_bytes([0x22; 32]),
        app_payload: vec![0xAA, 0xBB],
    };
    let bytes = canonical_bincode().serialize(&g).expect("encode");
    // Layout: seed (32 raw — fixed array, no len prefix) +
    //         founder_pubkey (40 = 8 len-prefix + 32 bytes) +
    //         app_payload (8 len-prefix + N bytes)
    // = 32 + 40 + 8 + 2 = 82 bytes
    assert_eq!(bytes.len(), 82, "GenesisV1 wire size for 2-byte payload");
    // First 32 bytes are seed (raw).
    assert_eq!(&bytes[0..32], &[0x11; 32]);
    // Bytes 32..40 are author-pubkey length prefix (u64 BE) = 32.
    assert_eq!(&bytes[32..40], &[0, 0, 0, 0, 0, 0, 0, 32]);
    // Bytes 40..72 are the author bytes.
    assert_eq!(&bytes[40..72], &[0x22; 32]);
    // Bytes 72..80 are payload length prefix (u64 BE) = 2.
    assert_eq!(&bytes[72..80], &[0, 0, 0, 0, 0, 0, 0, 2]);
    // Bytes 80..82 are payload.
    assert_eq!(&bytes[80..82], &[0xAA, 0xBB]);
}

#[test]
fn drift_anchor_wire_layout() {
    let a = DriftAnchor {
        event_hash: EventHash::from_bytes([0x33; 32]),
        author_seq_vec: vec![AuthorSeq {
            author: AuthorPubkey::from_bytes([0x44; 32]),
            max_seq: 7,
        }],
    };
    let bytes = canonical_bincode().serialize(&a).expect("encode");
    // event_hash: 40 (8 len + 32 bytes)
    // author_seq_vec: 8 (vec len = 1) + 40 (author = 8+32) + 8 (max_seq u64 BE)
    // = 40 + 8 + 40 + 8 = 96
    assert_eq!(bytes.len(), 96, "DriftAnchor wire size for 1-author anchor");
}

#[test]
fn drift_signed_payload_field_order_is_anchor_digest_format() {
    // Verifies the spec §8.1 normative field order: anchor, digest, digest_format.
    let p = DriftSignedPayload {
        anchor: DriftAnchor {
            event_hash: EventHash::ZERO,
            author_seq_vec: vec![],
        },
        digest: [0xCD; 32],
        digest_format: "x".into(),
    };
    let bytes = canonical_bincode().serialize(&p).expect("encode");
    // anchor: event_hash (40) + author_seq_vec len (8) = 48
    // digest: 32 raw (fixed array)
    // digest_format: 8 len + 1 byte = 9
    // total: 89
    assert_eq!(bytes.len(), 89);
    // First 48 bytes are anchor; the next 32 are digest.
    assert_eq!(
        &bytes[48..80],
        &[0xCD; 32],
        "digest must immediately follow anchor (field order)"
    );
}

#[test]
fn drift_message_first_three_fields_match_signed_payload_bytes() {
    // Spec §8.1: DriftMessage's first 3 fields in declaration order
    // (anchor, digest, digest_format) MUST byte-match DriftSignedPayload.
    let anchor = DriftAnchor {
        event_hash: EventHash::ZERO,
        author_seq_vec: vec![],
    };
    let digest = [0xEE; 32];
    let fmt = "bincode-1.3".to_string();

    let signed = DriftSignedPayload {
        anchor: anchor.clone(),
        digest,
        digest_format: fmt.clone(),
    };
    let signed_bytes = canonical_bincode().serialize(&signed).expect("encode");

    let msg = DriftMessage {
        anchor,
        digest,
        digest_format: fmt,
        signed_by_peer: PeerPubkey::from_bytes([0xFF; 32]),
        signature: [0x11; 64],
    };
    let msg_bytes = canonical_bincode().serialize(&msg).expect("encode");

    // The first signed_bytes.len() bytes of msg must equal signed_bytes exactly.
    assert_eq!(
        &msg_bytes[..signed_bytes.len()],
        signed_bytes.as_slice(),
        "DriftMessage prefix bytes must match DriftSignedPayload byte-for-byte (spec §8.1)"
    );
    let _ = hex_dump(&[]);
}

#[test]
fn heads_summary_wire_layout() {
    let h = HeadsSummary {
        authors: vec![AuthorHead {
            author: AuthorPubkey::from_bytes([1; 32]),
            seq: 5,
            hash: EventHash::ZERO,
        }],
        kernel_fuel_table_version: 1,
    };
    let bytes = canonical_bincode().serialize(&h).expect("encode");
    // authors: 8 (vec len = 1) + AuthorHead (40 + 8 + 40 = 88)
    // kernel_fuel_table_version: 4 (u32 BE)
    // = 8 + 88 + 4 = 100
    assert_eq!(bytes.len(), 100);
}

#[test]
fn event_request_wire_layout() {
    let r = EventRequest {
        author: AuthorPubkey::from_bytes([7; 32]),
        from_seq: 1,
        to_seq: 100,
    };
    let bytes = canonical_bincode().serialize(&r).expect("encode");
    // author: 40, from_seq: 8, to_seq: 8 = 56
    assert_eq!(bytes.len(), 56);
}

#[test]
fn heads_request_wire_layout() {
    let r = HeadsRequest { requests: vec![] };
    let bytes = canonical_bincode().serialize(&r).expect("encode");
    // requests: 8 (vec len = 0)
    assert_eq!(bytes.len(), 8);
}

// ---------------------------------------------------------------------------
// GossipMessage outer-variant wire-freeze (spec §6.2; review I-3 / M-11).
//
// Canonical bincode encodes Rust enum variants as a u32 fixint big-endian
// discriminator. Current variant declaration order in
// `crates/network/src/lib.rs` is:
//
//   GossipMessage::Event         = 0
//   GossipMessage::HeadsSummary  = 1
//   GossipMessage::HeadsRequest  = 2
//   GossipMessage::Drift         = 3
//
// These tests pin the byte string for each variant tag so a future reorder
// fails CI loudly — variant reordering is a wire-incompatible change.
// ---------------------------------------------------------------------------

fn sample_event_envelope() -> myrhiza_types::Event {
    myrhiza_types::Event {
        author: AuthorPubkey::from_bytes([1; 32]),
        seq: 1,
        prev: EventHash::ZERO,
        deps: std::collections::BTreeSet::new(),
        hlc: myrhiza_types::Hlc {
            wall_ms: 1_700_000_000_000,
            logical: 0,
        },
        payload: vec![0x01, 0x02],
        signature: [0xFF; 64],
    }
}

fn sample_heads_summary() -> HeadsSummary {
    HeadsSummary {
        authors: vec![],
        kernel_fuel_table_version: 0,
    }
}

fn sample_heads_request() -> HeadsRequest {
    HeadsRequest { requests: vec![] }
}

fn sample_drift_message() -> DriftMessage {
    DriftMessage {
        anchor: DriftAnchor {
            event_hash: EventHash::ZERO,
            author_seq_vec: vec![],
        },
        digest: [0; 32],
        digest_format: "bincode-1.3".into(),
        signed_by_peer: PeerPubkey::from_bytes([0; 32]),
        signature: [0; 64],
    }
}

#[test]
fn gossip_message_event_variant_tag_is_zero_u32_be() {
    use myrhiza_network::GossipMessage;

    let msg = GossipMessage::Event(sample_event_envelope());
    let bytes = canonical_bincode().serialize(&msg).expect("encode");
    assert_eq!(
        &bytes[..4],
        &[0x00, 0x00, 0x00, 0x00],
        "variant tag for GossipMessage::Event must be 0 (u32 BE)"
    );
}

#[test]
fn gossip_message_heads_summary_variant_tag_is_one_u32_be() {
    use myrhiza_network::GossipMessage;

    let msg = GossipMessage::HeadsSummary(sample_heads_summary());
    let bytes = canonical_bincode().serialize(&msg).expect("encode");
    assert_eq!(
        &bytes[..4],
        &[0x00, 0x00, 0x00, 0x01],
        "variant tag for GossipMessage::HeadsSummary must be 1 (u32 BE)"
    );
}

#[test]
fn gossip_message_heads_request_variant_tag_is_two_u32_be() {
    use myrhiza_network::GossipMessage;

    let msg = GossipMessage::HeadsRequest(sample_heads_request());
    let bytes = canonical_bincode().serialize(&msg).expect("encode");
    assert_eq!(
        &bytes[..4],
        &[0x00, 0x00, 0x00, 0x02],
        "variant tag for GossipMessage::HeadsRequest must be 2 (u32 BE)"
    );
}

#[test]
fn gossip_message_drift_variant_tag_is_three_u32_be() {
    use myrhiza_network::GossipMessage;

    let msg = GossipMessage::Drift(sample_drift_message());
    let bytes = canonical_bincode().serialize(&msg).expect("encode");
    assert_eq!(
        &bytes[..4],
        &[0x00, 0x00, 0x00, 0x03],
        "variant tag for GossipMessage::Drift must be 3 (u32 BE)"
    );
}
