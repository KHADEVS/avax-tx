//! The building blocks shared by P-Chain transactions: inputs and outputs
//! (`avax`), the secp256k1 feature extension (`secp256k1fx`), stakeable
//! locks, BLS proofs of possession and owners.
//!
//! Type IDs are the ones avalanchego's P-Chain codec registers, in
//! `vms/platformvm/platform/codec.go`.

use crate::codec::{codec_enum, codec_struct};

/// A 32-byte ID (transaction, asset, chain, subnet, validation).
pub type Id = [u8; 32];
/// A 20-byte address.
pub type ShortId = [u8; 20];
/// A 20-byte node ID.
pub type NodeId = [u8; 20];

codec_struct! {
    /// `secp256k1fx.OutputOwners`: who can spend, from when, with how many
    /// signatures.
    pub struct OutputOwners {
        /// Unix time before which the output can't be spent.
        pub locktime: u64,
        /// The number of signatures needed.
        pub threshold: u32,
        /// The addresses that can sign, sorted.
        pub addrs: Vec<ShortId>,
    }
}

codec_struct! {
    /// `secp256k1fx.TransferOutput`.
    pub struct TransferOutput {
        /// The amount, in nAVAX for AVAX.
        pub amount: u64,
        /// Who can spend it.
        pub owners: OutputOwners,
    }
}

codec_struct! {
    /// `secp256k1fx.Input`: which of the owners' keys sign.
    pub struct Input {
        /// Indices into the owners' addresses, sorted.
        pub sig_indices: Vec<u32>,
    }
}

codec_struct! {
    /// `secp256k1fx.TransferInput`.
    pub struct TransferInput {
        /// The amount consumed.
        pub amount: u64,
        /// Which keys sign.
        pub input: Input,
    }
}

codec_struct! {
    /// `secp256k1fx.Credential`: one recoverable secp256k1 signature
    /// (`r ‖ s ‖ v`) per signature index of the matching input.
    pub struct Credential {
        /// The signatures.
        pub sigs: Vec<[u8; 65]>,
    }
}

codec_enum! {
    /// The output inside a stakeable lock. avalanchego rejects nested locks,
    /// so only a transfer output is accepted here.
    pub enum LockableOut {
        /// `secp256k1fx.TransferOutput`.
        Transfer(TransferOutput) = 7,
    }
}

codec_enum! {
    /// The input inside a stakeable lock.
    pub enum LockableIn {
        /// `secp256k1fx.TransferInput`.
        Transfer(TransferInput) = 5,
    }
}

codec_struct! {
    /// `stakeable.LockOut`: an output locked for staking until `locktime`.
    pub struct LockOut {
        /// Unix time until which the output can only be staked.
        pub locktime: u64,
        /// The output.
        pub out: LockableOut,
    }
}

codec_struct! {
    /// `stakeable.LockIn`: spends a `LockOut`.
    pub struct LockIn {
        /// The lock's time.
        pub locktime: u64,
        /// The input.
        pub input: LockableIn,
    }
}

codec_enum! {
    /// `avax.TransferableOut`.
    pub enum Out {
        /// `secp256k1fx.TransferOutput`.
        Transfer(TransferOutput) = 7,
        /// `stakeable.LockOut`.
        Locked(LockOut) = 22,
    }
}

codec_enum! {
    /// `avax.TransferableIn`.
    pub enum In {
        /// `secp256k1fx.TransferInput`.
        Transfer(TransferInput) = 5,
        /// `stakeable.LockIn`.
        Locked(LockIn) = 21,
    }
}

codec_struct! {
    /// `avax.TransferableOutput`.
    pub struct TransferableOutput {
        /// The asset.
        pub asset_id: Id,
        /// The output.
        pub out: Out,
    }
}

codec_struct! {
    /// `avax.TransferableInput`: spends output `output_index` of `tx_id`.
    pub struct TransferableInput {
        /// The transaction that created the UTXO.
        pub tx_id: Id,
        /// Its index in that transaction's outputs.
        pub output_index: u32,
        /// The asset.
        pub asset_id: Id,
        /// The input.
        pub input: In,
    }
}

codec_enum! {
    /// `fx.Owner`: who receives rewards or controls a subnet.
    pub enum Owner {
        /// `secp256k1fx.OutputOwners`.
        OutputOwners(OutputOwners) = 11,
    }
}

codec_enum! {
    /// The `verify.Verifiable` that authorizes a subnet or validator action:
    /// which of the owner's keys sign.
    pub enum Auth {
        /// `secp256k1fx.Input`.
        Input(Input) = 10,
    }
}

codec_enum! {
    /// A credential in a signed transaction.
    pub enum Cred {
        /// `secp256k1fx.Credential`.
        Secp256k1(Credential) = 9,
    }
}

codec_struct! {
    /// `signer.Empty`: no BLS key.
    pub struct Empty {}
}

codec_struct! {
    /// `signer.ProofOfPossession`: a compressed BLS12-381 public key and its
    /// proof of possession.
    pub struct ProofOfPossession {
        /// The G1 public key, compressed.
        pub public_key: [u8; 48],
        /// The G2 signature over the public key, compressed.
        pub proof: [u8; 96],
    }
}

codec_enum! {
    /// `signer.Signer`.
    pub enum Signer {
        /// No BLS key.
        Empty(Empty) = 27,
        /// A BLS key with its proof of possession.
        ProofOfPossession(ProofOfPossession) = 28,
    }
}

codec_struct! {
    /// `message.PChainOwner`: the owner of an L1 validator's remaining
    /// balance, or of its deactivation.
    pub struct PChainOwner {
        /// The number of signatures needed.
        pub threshold: u32,
        /// The addresses that can sign.
        pub addresses: Vec<ShortId>,
    }
}

codec_struct! {
    /// `avax.BaseTx`: the fields every transaction that moves funds has.
    pub struct BaseTx {
        /// 1 for Mainnet, 5 for Fuji.
        pub network_id: u32,
        /// The P-Chain's ID (all zeros).
        pub blockchain_id: Id,
        /// Outputs, sorted.
        pub outs: Vec<TransferableOutput>,
        /// Inputs, sorted by UTXO.
        pub ins: Vec<TransferableInput>,
        /// Free-form bytes, at most 256.
        pub memo: Vec<u8>,
    }
}

codec_struct! {
    /// `platform.Validator`.
    pub struct Validator {
        /// The node.
        pub node_id: NodeId,
        /// Start time (Unix seconds).
        pub start: u64,
        /// End time (Unix seconds).
        pub end: u64,
        /// Stake weight.
        pub weight: u64,
    }
}

codec_struct! {
    /// `platform.SubnetValidator`.
    pub struct SubnetValidator {
        /// The validator.
        pub validator: Validator,
        /// The subnet it validates.
        pub subnet: Id,
    }
}
