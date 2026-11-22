//! Signing and addresses.
//!
//! A P-Chain input is signed with secp256k1 over SHA-256 of the unsigned
//! transaction bytes. A signature is 65 bytes, `r ‖ s ‖ v`, with `s` in the
//! lower half of the curve order (avalanchego rejects the other half) and
//! `v` the recovery ID, 0 or 1. An address is RIPEMD-160 of SHA-256 of the
//! compressed public key, shown in Bech32 as `P-avax1…` on Mainnet.

use bech32::{Bech32, Hrp};
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

use crate::components::{Cred, Credential, ShortId};
use crate::pchain::{Tx, UnsignedTx};

/// Why a signature could not be made or read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignError {
    /// The signature bytes are not a valid low-S secp256k1 signature.
    BadSignature,
    /// No public key can be recovered from this signature and hash.
    Unrecoverable,
}

impl std::fmt::Display for SignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignError::BadSignature => write!(f, "invalid or high-S signature"),
            SignError::Unrecoverable => write!(f, "no public key recovers from this signature"),
        }
    }
}

impl std::error::Error for SignError {}

/// SHA-256 of the unsigned transaction bytes: what each key signs.
pub fn signing_hash(unsigned: &UnsignedTx) -> [u8; 32] {
    Sha256::digest(unsigned.to_bytes()).into()
}

/// Signs a 32-byte hash. RFC 6979 nonces, so the result is deterministic.
pub fn sign_hash(key: &SigningKey, hash: &[u8; 32]) -> [u8; 65] {
    let (sig, recid) = key
        .sign_prehash_recoverable(hash)
        .expect("a 32-byte prehash is always accepted");
    // k256 already returns low-S; normalize anyway, flipping v with s.
    let (sig, recid) = match sig.normalize_s() {
        Some(low) => (
            low,
            RecoveryId::from_byte(recid.to_byte() ^ 1).expect("0 or 1"),
        ),
        None => (sig, recid),
    };
    let mut out = [0u8; 65];
    out[..64].copy_from_slice(&sig.to_bytes());
    out[64] = recid.to_byte();
    out
}

/// Recovers the public key that made `sig` over `hash`, rejecting high-S
/// signatures and recovery IDs other than 0 and 1, as avalanchego does.
pub fn recover(hash: &[u8; 32], sig: &[u8; 65]) -> Result<VerifyingKey, SignError> {
    let signature = Signature::from_slice(&sig[..64]).map_err(|_| SignError::BadSignature)?;
    if signature.normalize_s().is_some() {
        return Err(SignError::BadSignature);
    }
    let recid = match sig[64] {
        v @ (0 | 1) => RecoveryId::from_byte(v).expect("0 or 1"),
        _ => return Err(SignError::BadSignature),
    };
    VerifyingKey::recover_from_prehash(hash, &signature, recid)
        .map_err(|_| SignError::Unrecoverable)
}

/// Signs `unsigned`: one credential per entry of `signers`, in input order,
/// each holding one signature per key, in signature-index order.
pub fn sign(unsigned: UnsignedTx, signers: &[Vec<&SigningKey>]) -> Tx {
    let hash = signing_hash(&unsigned);
    let creds = signers
        .iter()
        .map(|keys| {
            Cred::Secp256k1(Credential {
                sigs: keys.iter().map(|k| sign_hash(k, &hash)).collect(),
            })
        })
        .collect();
    Tx { unsigned, creds }
}

/// The address of a public key: RIPEMD-160(SHA-256(compressed key)).
pub fn address(key: &VerifyingKey) -> ShortId {
    let compressed = key.to_encoded_point(true);
    Ripemd160::digest(Sha256::digest(compressed.as_bytes())).into()
}

/// Formats an address as Avalanche shows it, e.g. `P-avax1…` for chain
/// alias `P` and HRP `avax` (Mainnet) or `fuji` (Fuji).
pub fn format_address(
    chain: &str,
    hrp: &str,
    addr: &ShortId,
) -> Result<String, bech32::EncodeError> {
    let hrp = Hrp::parse(hrp).expect("a valid human-readable part");
    Ok(format!("{chain}-{}", bech32::encode::<Bech32>(hrp, addr)?))
}
