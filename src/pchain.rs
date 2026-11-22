//! P-Chain transactions and blocks, as avalanchego v1.15 encodes them.
//!
//! Field order follows the `serialize:"true"` fields of the Go structs in
//! `vms/platformvm/platform`; embedded structs are fields here. Type IDs are
//! assigned by registration order in that package's `codec.go`.

use sha2::{Digest, Sha256};

use crate::codec::{self, codec_enum, codec_struct, Error, Str};
use crate::components::*;

codec_struct! {
    /// Type 12. Primary network validator (pre-Banff form).
    pub struct AddValidatorTx {
        pub base: BaseTx,
        pub validator: Validator,
        pub stake_outs: Vec<TransferableOutput>,
        pub rewards_owner: Owner,
        pub delegation_shares: u32,
    }
}

codec_struct! {
    /// Type 13. Validator of a permissioned subnet.
    pub struct AddSubnetValidatorTx {
        pub base: BaseTx,
        pub validator: SubnetValidator,
        pub subnet_auth: Auth,
    }
}

codec_struct! {
    /// Type 14. Primary network delegator (pre-Banff form).
    pub struct AddDelegatorTx {
        pub base: BaseTx,
        pub validator: Validator,
        pub stake_outs: Vec<TransferableOutput>,
        pub delegation_rewards_owner: Owner,
    }
}

codec_struct! {
    /// Type 15. A new chain on a subnet.
    pub struct CreateChainTx {
        pub base: BaseTx,
        pub subnet_id: Id,
        pub chain_name: Str,
        pub vm_id: Id,
        pub fx_ids: Vec<Id>,
        pub genesis_data: Vec<u8>,
        pub subnet_auth: Auth,
    }
}

codec_struct! {
    /// Type 16. A new subnet.
    pub struct CreateSubnetTx {
        pub base: BaseTx,
        pub owner: Owner,
    }
}

codec_struct! {
    /// Type 17. Funds exported to the P-Chain by another chain.
    pub struct ImportTx {
        pub base: BaseTx,
        pub source_chain: Id,
        pub imported_inputs: Vec<TransferableInput>,
    }
}

codec_struct! {
    /// Type 18. Funds sent to another chain.
    pub struct ExportTx {
        pub base: BaseTx,
        pub destination_chain: Id,
        pub exported_outputs: Vec<TransferableOutput>,
    }
}

codec_struct! {
    /// Type 19. Chain time advance (pre-Banff, made by validators).
    pub struct AdvanceTimeTx {
        pub time: u64,
    }
}

codec_struct! {
    /// Type 20. Ends a staking period (made by validators).
    pub struct RewardValidatorTx {
        pub tx_id: Id,
    }
}

codec_struct! {
    /// Type 23. Removes a validator from a permissioned subnet.
    pub struct RemoveSubnetValidatorTx {
        pub base: BaseTx,
        pub node_id: NodeId,
        pub subnet: Id,
        pub subnet_auth: Auth,
    }
}

codec_struct! {
    /// Type 24. Turns a subnet permissionless (no longer accepted since Etna).
    pub struct TransformSubnetTx {
        pub base: BaseTx,
        pub subnet: Id,
        pub asset_id: Id,
        pub initial_supply: u64,
        pub maximum_supply: u64,
        pub min_consumption_rate: u64,
        pub max_consumption_rate: u64,
        pub min_validator_stake: u64,
        pub max_validator_stake: u64,
        pub min_stake_duration: u32,
        pub max_stake_duration: u32,
        pub min_delegation_fee: u32,
        pub min_delegator_stake: u64,
        pub max_validator_weight_factor: u8,
        pub uptime_requirement: u32,
        pub subnet_auth: Auth,
    }
}

codec_struct! {
    /// Type 25. Validator, primary network or permissionless subnet.
    pub struct AddPermissionlessValidatorTx {
        pub base: BaseTx,
        pub validator: Validator,
        pub subnet: Id,
        pub signer: Signer,
        pub stake_outs: Vec<TransferableOutput>,
        pub validator_rewards_owner: Owner,
        pub delegator_rewards_owner: Owner,
        pub delegation_shares: u32,
    }
}

codec_struct! {
    /// Type 26. Delegator, primary network or permissionless subnet.
    pub struct AddPermissionlessDelegatorTx {
        pub base: BaseTx,
        pub validator: Validator,
        pub subnet: Id,
        pub stake_outs: Vec<TransferableOutput>,
        pub delegation_rewards_owner: Owner,
    }
}

codec_struct! {
    /// Type 33. Changes a subnet's owner.
    pub struct TransferSubnetOwnershipTx {
        pub base: BaseTx,
        pub subnet: Id,
        pub subnet_auth: Auth,
        pub owner: Owner,
    }
}

codec_struct! {
    /// Type 35. Converts a subnet to an L1 with its initial validators.
    pub struct ConvertSubnetToL1Tx {
        pub base: BaseTx,
        pub subnet: Id,
        pub chain_id: Id,
        /// The validator manager contract's address on `chain_id`.
        pub address: Vec<u8>,
        pub validators: Vec<ConvertSubnetToL1Validator>,
        pub subnet_auth: Auth,
    }
}

codec_struct! {
    /// An initial L1 validator in a `ConvertSubnetToL1Tx`.
    pub struct ConvertSubnetToL1Validator {
        /// Variable length: L1 node IDs need not be 20 bytes.
        pub node_id: Vec<u8>,
        pub weight: u64,
        /// nAVAX set aside to pay for the validator's continuous fee.
        pub balance: u64,
        pub signer: ProofOfPossession,
        pub remaining_balance_owner: PChainOwner,
        pub deactivation_owner: PChainOwner,
    }
}

codec_struct! {
    /// Type 36. Registers an L1 validator from a Warp message.
    pub struct RegisterL1ValidatorTx {
        pub base: BaseTx,
        pub balance: u64,
        /// BLS proof of possession of the key in the message.
        pub proof_of_possession: [u8; 96],
        /// A signed Warp message carrying a `RegisterL1ValidatorMessage`.
        pub message: Vec<u8>,
    }
}

codec_struct! {
    /// Type 37. Changes an L1 validator's weight from a Warp message.
    pub struct SetL1ValidatorWeightTx {
        pub base: BaseTx,
        /// A signed Warp message carrying an `L1ValidatorWeightMessage`.
        pub message: Vec<u8>,
    }
}

codec_struct! {
    /// Type 38. Tops up an L1 validator's fee balance.
    pub struct IncreaseL1ValidatorBalanceTx {
        pub base: BaseTx,
        pub validation_id: Id,
        pub balance: u64,
    }
}

codec_struct! {
    /// Type 39. Deactivates an L1 validator, signed by its deactivation owner.
    pub struct DisableL1ValidatorTx {
        pub base: BaseTx,
        pub validation_id: Id,
        pub disable_auth: Auth,
    }
}

codec_struct! {
    /// Type 40 (Helicon). Primary network validator that renews itself.
    pub struct AddAutoRenewedValidatorTx {
        pub base: BaseTx,
        pub validator_node_id: Vec<u8>,
        pub signer: Signer,
        pub stake_outs: Vec<TransferableOutput>,
        pub validator_rewards_owner: Owner,
        pub delegator_rewards_owner: Owner,
        pub validator_authority: Owner,
        pub delegation_shares: u32,
        pub auto_compound_reward_shares: u32,
        pub period: u64,
    }
}

codec_struct! {
    /// Type 41 (Helicon). Changes an auto-renewed validator's settings.
    pub struct SetAutoRenewedValidatorConfigTx {
        pub base: BaseTx,
        pub tx_id: Id,
        pub auth: Auth,
        pub auto_compound_reward_shares: u32,
        pub period: u64,
    }
}

codec_struct! {
    /// Type 42 (Helicon). Ends an auto-renewed validator's period.
    pub struct RewardAutoRenewedValidatorTx {
        pub tx_id: Id,
        pub timestamp: u64,
    }
}

codec_enum! {
    /// Any P-Chain unsigned transaction.
    pub enum UnsignedTx {
        AddValidator(AddValidatorTx) = 12,
        AddSubnetValidator(AddSubnetValidatorTx) = 13,
        AddDelegator(AddDelegatorTx) = 14,
        CreateChain(CreateChainTx) = 15,
        CreateSubnet(CreateSubnetTx) = 16,
        Import(ImportTx) = 17,
        Export(ExportTx) = 18,
        AdvanceTime(AdvanceTimeTx) = 19,
        RewardValidator(RewardValidatorTx) = 20,
        RemoveSubnetValidator(RemoveSubnetValidatorTx) = 23,
        TransformSubnet(TransformSubnetTx) = 24,
        AddPermissionlessValidator(AddPermissionlessValidatorTx) = 25,
        AddPermissionlessDelegator(AddPermissionlessDelegatorTx) = 26,
        TransferSubnetOwnership(TransferSubnetOwnershipTx) = 33,
        Base(BaseTx) = 34,
        ConvertSubnetToL1(ConvertSubnetToL1Tx) = 35,
        RegisterL1Validator(RegisterL1ValidatorTx) = 36,
        SetL1ValidatorWeight(SetL1ValidatorWeightTx) = 37,
        IncreaseL1ValidatorBalance(IncreaseL1ValidatorBalanceTx) = 38,
        DisableL1Validator(DisableL1ValidatorTx) = 39,
        AddAutoRenewedValidator(AddAutoRenewedValidatorTx) = 40,
        SetAutoRenewedValidatorConfig(SetAutoRenewedValidatorConfigTx) = 41,
        RewardAutoRenewedValidator(RewardAutoRenewedValidatorTx) = 42,
    }
}

impl UnsignedTx {
    /// Decodes unsigned transaction bytes (codec version, type ID, fields).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        codec::from_bytes(bytes)
    }

    /// The bytes that are hashed and signed.
    pub fn to_bytes(&self) -> Vec<u8> {
        codec::to_bytes(self)
    }
}

codec_struct! {
    /// A signed transaction: the unsigned transaction, then one credential
    /// per input that needs signatures, in input order.
    pub struct Tx {
        pub unsigned: UnsignedTx,
        pub creds: Vec<Cred>,
    }
}

impl Tx {
    /// Decodes signed transaction bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        codec::from_bytes(bytes)
    }

    /// The signed bytes, as broadcast.
    pub fn to_bytes(&self) -> Vec<u8> {
        codec::to_bytes(self)
    }

    /// The transaction ID: SHA-256 of the signed bytes.
    pub fn id(&self) -> Id {
        Sha256::digest(self.to_bytes()).into()
    }
}

codec_struct! {
    /// Type 0.
    pub struct ApricotProposalBlock {
        pub parent_id: Id,
        pub height: u64,
        pub tx: Tx,
    }
}

codec_struct! {
    /// Type 1.
    pub struct ApricotAbortBlock {
        pub parent_id: Id,
        pub height: u64,
    }
}

codec_struct! {
    /// Type 2.
    pub struct ApricotCommitBlock {
        pub parent_id: Id,
        pub height: u64,
    }
}

codec_struct! {
    /// Type 3.
    pub struct ApricotStandardBlock {
        pub parent_id: Id,
        pub height: u64,
        pub transactions: Vec<Tx>,
    }
}

codec_struct! {
    /// Type 4.
    pub struct ApricotAtomicBlock {
        pub parent_id: Id,
        pub height: u64,
        pub tx: Tx,
    }
}

codec_struct! {
    /// Type 29.
    pub struct BanffProposalBlock {
        pub time: u64,
        pub transactions: Vec<Tx>,
        pub parent_id: Id,
        pub height: u64,
        pub tx: Tx,
    }
}

codec_struct! {
    /// Type 30.
    pub struct BanffAbortBlock {
        pub time: u64,
        pub parent_id: Id,
        pub height: u64,
    }
}

codec_struct! {
    /// Type 31.
    pub struct BanffCommitBlock {
        pub time: u64,
        pub parent_id: Id,
        pub height: u64,
    }
}

codec_struct! {
    /// Type 32.
    pub struct BanffStandardBlock {
        pub time: u64,
        pub parent_id: Id,
        pub height: u64,
        pub transactions: Vec<Tx>,
    }
}

codec_enum! {
    /// Any P-Chain block.
    pub enum Block {
        ApricotProposal(ApricotProposalBlock) = 0,
        ApricotAbort(ApricotAbortBlock) = 1,
        ApricotCommit(ApricotCommitBlock) = 2,
        ApricotStandard(ApricotStandardBlock) = 3,
        ApricotAtomic(ApricotAtomicBlock) = 4,
        BanffProposal(BanffProposalBlock) = 29,
        BanffAbort(BanffAbortBlock) = 30,
        BanffCommit(BanffCommitBlock) = 31,
        BanffStandard(BanffStandardBlock) = 32,
    }
}

impl Block {
    /// Decodes block bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        codec::from_bytes(bytes)
    }

    /// The block's bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        codec::to_bytes(self)
    }

    /// The block ID: SHA-256 of its bytes.
    pub fn id(&self) -> Id {
        Sha256::digest(self.to_bytes()).into()
    }
}
