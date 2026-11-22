//! Signing against avalanchego's own vectors (utils/crypto/secp256k1,
//! v1.15.3): its first test key must give the same address, public key and
//! signatures, byte for byte.

use avax_tx::codec;
use avax_tx::pchain::{Tx, UnsignedTx};
use avax_tx::sign;
use k256::ecdsa::SigningKey;
use sha2::{Digest, Sha256};

const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn base58_decode(s: &str) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new();
    for c in s.bytes() {
        let mut carry = BASE58.iter().position(|&b| b == c).expect("base58 digit") as u32;
        for b in bytes.iter_mut() {
            carry += (*b as u32) * 58;
            *b = carry as u8;
            carry >>= 8;
        }
        while carry > 0 {
            bytes.push(carry as u8);
            carry >>= 8;
        }
    }
    let zeros = s.bytes().take_while(|&c| c == b'1').count();
    let mut out = vec![0u8; zeros];
    out.extend(bytes.iter().rev());
    out
}

/// CB58 decode: Base58, then check and drop the 4-byte SHA-256 checksum.
fn cb58_decode(s: &str) -> Vec<u8> {
    let raw = base58_decode(s);
    let (body, check) = raw.split_at(raw.len() - 4);
    assert_eq!(&Sha256::digest(body)[28..], check, "CB58 checksum");
    body.to_vec()
}

/// avalanchego's `secp256k1.TestKeys()[0]`, a public test key.
fn test_key_0() -> SigningKey {
    let bytes = cb58_decode("24jUJ9vZexUM6expyMcT48LBx27k1m7xpraoV62oSQAHdziao5");
    SigningKey::from_slice(&bytes).unwrap()
}

#[test]
fn test_key_0_has_avalanchegos_address_and_public_key() {
    let key = test_key_0();
    let public = key.verifying_key();
    assert_eq!(
        hex::encode(public.to_encoded_point(true).as_bytes()),
        "0373935347884478e4945cd0fd948ecf088b94dfc92074f0fb03da6f4dbc94357d"
    );
    assert_eq!(
        sign::address(public).to_vec(),
        cb58_decode("Q4MzFZZDPHRPAHFeDs3NiyyaZDvxHKivf")
    );
}

#[test]
fn signatures_match_avalanchegos_test_signing() {
    // TestSigning: key.Sign(msg) signs SHA-256(msg).
    let cases: [(&str, &str); 3] = [
        ("hello world", "178cb6096b3ca5820a4c6ecedf15b68b6f50e252c2b64f3774888602cc9fa08c5d019d82fdde95fdf234aa2d12ad79b5abb345fe953a9f72f70914fd3139063b00"),
        ("scooby doo", "c2573f29b0d17ae7009a9f17a4558d32462e5b8d059e3832ecb032541abc7daf5751f96b8571bcb718d26be8ed8d59b0d60369ab57acc0f7133b219456038ec701"),
        ("a really long string", "1bf561c36007d2a61268e9e13a902a9c2ba43e28f8d47554215711dcdcc6d35e784318f62291373c95779f6794910a4416bfa3ae9f25fa34a014ea9c6fe0203700"),
    ];
    let key = test_key_0();
    for (msg, expected) in cases {
        let hash: [u8; 32] = Sha256::digest(msg.as_bytes()).into();
        let sig = sign::sign_hash(&key, &hash);
        assert_eq!(hex::encode(sig), expected, "{msg}");
        let recovered = sign::recover(&hash, &sig).unwrap();
        assert_eq!(&recovered, key.verifying_key());
    }
}

#[test]
fn high_s_and_bad_recovery_ids_are_rejected() {
    let key = test_key_0();
    let hash: [u8; 32] = Sha256::digest(b"hello world").into();
    let sig = sign::sign_hash(&key, &hash);
    // s -> n - s gives the same signature in the upper half, which
    // avalanchego refuses.
    let n =
        hex::decode("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141").unwrap();
    let mut high = sig;
    let mut borrow = 0i16;
    for i in (0..32).rev() {
        let d = n[i] as i16 - sig[32 + i] as i16 - borrow;
        high[32 + i] = d.rem_euclid(256) as u8;
        borrow = (d < 0) as i16;
    }
    high[64] ^= 1;
    assert_eq!(
        sign::recover(&hash, &high),
        Err(sign::SignError::BadSignature)
    );
    let mut bad_v = sig;
    bad_v[64] = 27;
    assert_eq!(
        sign::recover(&hash, &bad_v),
        Err(sign::SignError::BadSignature)
    );
}

#[test]
fn a_signed_transaction_round_trips_and_its_signer_is_recoverable() {
    // The simple BaseTx vector from avalanchego, signed for its one input.
    let doc: serde_json::Value =
        serde_json::from_str(include_str!("../vectors/pchain-avalanchego.json")).unwrap();
    let v = doc["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["var"] == "expectedUnsignedSimpleBaseTxBytes")
        .unwrap();
    let unsigned =
        UnsignedTx::from_bytes(&hex::decode(v["hex"].as_str().unwrap()).unwrap()).unwrap();
    let key = test_key_0();
    let tx = sign::sign(unsigned.clone(), &[vec![&key]]);
    let bytes = tx.to_bytes();
    let back = Tx::from_bytes(&bytes).unwrap();
    assert_eq!(back, tx);
    assert_eq!(codec::to_bytes(&back.unsigned), unsigned.to_bytes());
    let avax_tx::components::Cred::Secp256k1(cred) = &back.creds[0];
    let signer = sign::recover(&sign::signing_hash(&back.unsigned), &cred.sigs[0]).unwrap();
    assert_eq!(sign::address(&signer), sign::address(key.verifying_key()));
    assert_eq!(tx.id(), <[u8; 32]>::from(Sha256::digest(&bytes)));
}

#[test]
fn addresses_format_as_on_mainnet_and_fuji() {
    let addr = sign::address(test_key_0().verifying_key());
    let mainnet = sign::format_address("P", "avax", &addr).unwrap();
    let fuji = sign::format_address("P", "fuji", &addr).unwrap();
    assert!(mainnet.starts_with("P-avax1") && fuji.starts_with("P-fuji1"));
    assert_eq!(mainnet.len(), "P-avax1".len() + 32 + 6);
}
