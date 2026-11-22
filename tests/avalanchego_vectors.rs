//! Every byte vector from avalanchego's P-Chain serialization tests
//! (vectors/pchain-avalanchego.json, extracted by tools/extract_vectors.py)
//! must decode, carry the type ID its first bytes name, and encode back to
//! exactly the same bytes.

use avax_tx::codec::{self, Codec};
use avax_tx::pchain::{Block, UnsignedTx};

fn vectors() -> Vec<serde_json::Value> {
    let text = include_str!("../vectors/pchain-avalanchego.json");
    let doc: serde_json::Value = serde_json::from_str(text).unwrap();
    doc["vectors"].as_array().unwrap().clone()
}

fn round_trip<T: Codec + std::fmt::Debug>(bytes: &[u8], type_id: impl Fn(&T) -> u32, label: &str) {
    let value: T = codec::from_bytes(bytes).unwrap_or_else(|e| panic!("{label}: {e}"));
    let expected_id = u32::from_be_bytes(bytes[2..6].try_into().unwrap());
    assert_eq!(type_id(&value), expected_id, "{label}: type ID");
    assert_eq!(
        codec::to_bytes(&value),
        bytes,
        "{label}: re-encoding differs"
    );
}

#[test]
fn every_avalanchego_vector_round_trips() {
    let vectors = vectors();
    assert_eq!(vectors.len(), 29);
    let (mut txs, mut blocks) = (0, 0);
    for v in &vectors {
        let bytes = hex::decode(v["hex"].as_str().unwrap()).unwrap();
        let label = format!(
            "{}:{} {}",
            v["file"].as_str().unwrap(),
            v["line"],
            v["var"].as_str().unwrap()
        );
        if v["file"] == "serialization_test.go" {
            round_trip::<Block>(&bytes, Block::type_id, &label);
            blocks += 1;
        } else {
            round_trip::<UnsignedTx>(&bytes, UnsignedTx::type_id, &label);
            txs += 1;
        }
    }
    assert_eq!((txs, blocks), (25, 4));
}

#[test]
fn a_truncated_vector_is_rejected_everywhere() {
    for v in vectors() {
        let bytes = hex::decode(v["hex"].as_str().unwrap()).unwrap();
        let is_block = v["file"] == "serialization_test.go";
        for cut in 0..bytes.len() {
            let short = &bytes[..cut];
            let ok = if is_block {
                Block::from_bytes(short).is_ok()
            } else {
                UnsignedTx::from_bytes(short).is_ok()
            };
            assert!(!ok, "{} accepted when cut at {cut}", v["var"]);
        }
    }
}

#[test]
fn trailing_bytes_and_other_versions_are_rejected() {
    let bytes = hex::decode(vectors()[0]["hex"].as_str().unwrap()).unwrap();
    let mut longer = bytes.clone();
    longer.push(0);
    assert_eq!(
        UnsignedTx::from_bytes(&longer),
        Err(codec::Error::TrailingBytes(1))
    );
    let mut v1 = bytes;
    v1[1] = 1;
    assert_eq!(
        UnsignedTx::from_bytes(&v1),
        Err(codec::Error::UnsupportedVersion(1))
    );
}
