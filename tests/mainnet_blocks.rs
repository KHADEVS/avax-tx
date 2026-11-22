//! Real Mainnet blocks, checked against the node's own decoding.
//!
//! vectors/mainnet-blocks.jsonl holds blocks fetched by tools/fetch_blocks.py
//! from a public avalanchego API, each as raw bytes and as the node's JSON.
//! Each block must decode, encode back to the same bytes, and, written out
//! the way avalanchego writes JSON, equal the node's JSON field for field,
//! including the transaction and block IDs (recomputed here). Only `fxID`
//! is dropped from the node's side: the node derives it, it is not in the
//! bytes.

use avax_tx::components::*;
use avax_tx::pchain::*;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn base58(data: &[u8]) -> String {
    let zeros = data.iter().take_while(|&&b| b == 0).count();
    let mut digits: Vec<u8> = Vec::new();
    for &byte in data {
        let mut carry = byte as u32;
        for d in digits.iter_mut() {
            carry += (*d as u32) << 8;
            *d = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    let mut s = "1".repeat(zeros);
    s.extend(digits.iter().rev().map(|&d| BASE58[d as usize] as char));
    s
}

/// CB58: Base58 of the bytes followed by the last 4 bytes of their SHA-256.
fn cb58(data: &[u8]) -> String {
    let mut v = data.to_vec();
    v.extend_from_slice(&Sha256::digest(data)[28..]);
    base58(&v)
}

fn hex0x(b: &[u8]) -> Value {
    json!(format!("0x{}", hex::encode(b)))
}

fn base64(b: &[u8]) -> Value {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    json!(s)
}

struct Ctx {
    hrp: &'static str,
}

impl Ctx {
    fn id(&self, id: &Id) -> Value {
        json!(cb58(id))
    }
    fn node_id(&self, id: &NodeId) -> Value {
        json!(format!("NodeID-{}", cb58(id)))
    }
    fn owners(&self, o: &OutputOwners) -> Value {
        let addrs: Vec<Value> = o
            .addrs
            .iter()
            .map(|a| json!(avax_tx::sign::format_address("P", self.hrp, a).unwrap()))
            .collect();
        json!({"addresses": addrs, "locktime": o.locktime, "threshold": o.threshold})
    }
    fn transfer_out(&self, t: &TransferOutput) -> Value {
        let mut v = self.owners(&t.owners);
        v["amount"] = json!(t.amount);
        v
    }
    fn out(&self, o: &Out) -> Value {
        match o {
            Out::Transfer(t) => self.transfer_out(t),
            Out::Locked(l) => {
                let LockableOut::Transfer(t) = &l.out;
                json!({"locktime": l.locktime, "output": self.transfer_out(t)})
            }
        }
    }
    fn transfer_in(&self, t: &TransferInput) -> Value {
        json!({"amount": t.amount, "signatureIndices": t.input.sig_indices})
    }
    fn input(&self, i: &In) -> Value {
        match i {
            In::Transfer(t) => self.transfer_in(t),
            In::Locked(l) => {
                let LockableIn::Transfer(t) = &l.input;
                json!({"locktime": l.locktime, "input": self.transfer_in(t)})
            }
        }
    }
    fn outputs(&self, outs: &[TransferableOutput]) -> Value {
        outs.iter()
            .map(|o| json!({"assetID": self.id(&o.asset_id), "output": self.out(&o.out)}))
            .collect()
    }
    fn inputs(&self, ins: &[TransferableInput]) -> Value {
        ins.iter()
            .map(|i| {
                json!({"txID": self.id(&i.tx_id), "outputIndex": i.output_index,
                       "assetID": self.id(&i.asset_id), "input": self.input(&i.input)})
            })
            .collect()
    }
    fn owner(&self, o: &Owner) -> Value {
        let Owner::OutputOwners(o) = o;
        self.owners(o)
    }
    fn auth(&self, a: &Auth) -> Value {
        let Auth::Input(i) = a;
        json!({"signatureIndices": i.sig_indices})
    }
    fn pop(&self, p: &ProofOfPossession) -> Value {
        json!({"publicKey": hex0x(&p.public_key), "proofOfPossession": hex0x(&p.proof)})
    }
    fn signer(&self, s: &Signer) -> Value {
        match s {
            Signer::Empty(_) => json!({}),
            Signer::ProofOfPossession(p) => self.pop(p),
        }
    }
    fn pchain_owner(&self, o: &PChainOwner) -> Value {
        let addrs: Vec<Value> = o.addresses.iter().map(|a| json!(cb58(a))).collect();
        json!({"threshold": o.threshold, "addresses": addrs})
    }
    fn validator(&self, v: &Validator) -> Map<String, Value> {
        obj(
            json!({"nodeID": self.node_id(&v.node_id), "start": v.start, "end": v.end, "weight": v.weight}),
        )
    }
    fn base(&self, b: &BaseTx) -> Map<String, Value> {
        obj(json!({
            "networkID": b.network_id, "blockchainID": self.id(&b.blockchain_id),
            "outputs": self.outputs(&b.outs), "inputs": self.inputs(&b.ins), "memo": hex0x(&b.memo),
        }))
    }

    fn unsigned(&self, u: &UnsignedTx) -> Value {
        use UnsignedTx::*;
        let (mut m, extra) = match u {
            AdvanceTime(t) => return json!({"time": t.time}),
            RewardValidator(t) => return json!({"txID": self.id(&t.tx_id)}),
            RewardAutoRenewedValidator(t) => {
                return json!({"txID": self.id(&t.tx_id), "timestamp": t.timestamp})
            }
            Base(b) => (self.base(b), json!({})),
            AddValidator(t) => (
                self.base(&t.base),
                json!({
                "validator": self.validator(&t.validator), "stake": self.outputs(&t.stake_outs),
                "rewardsOwner": self.owner(&t.rewards_owner), "shares": t.delegation_shares}),
            ),
            AddSubnetValidator(t) => {
                let mut v = self.validator(&t.validator.validator);
                v.insert("subnetID".into(), self.id(&t.validator.subnet));
                (
                    self.base(&t.base),
                    json!({"validator": v, "subnetAuthorization": self.auth(&t.subnet_auth)}),
                )
            }
            AddDelegator(t) => (
                self.base(&t.base),
                json!({
                "validator": self.validator(&t.validator), "stake": self.outputs(&t.stake_outs),
                "rewardsOwner": self.owner(&t.delegation_rewards_owner)}),
            ),
            CreateChain(t) => (
                self.base(&t.base),
                json!({
                "subnetID": self.id(&t.subnet_id),
                "chainName": String::from_utf8(t.chain_name.0.clone()).unwrap(),
                "vmID": self.id(&t.vm_id), "fxIDs": t.fx_ids.iter().map(|i| self.id(i)).collect::<Vec<_>>(),
                "genesisData": base64(&t.genesis_data), "subnetAuthorization": self.auth(&t.subnet_auth)}),
            ),
            CreateSubnet(t) => (self.base(&t.base), json!({"owner": self.owner(&t.owner)})),
            Import(t) => (
                self.base(&t.base),
                json!({
                "sourceChain": self.id(&t.source_chain), "importedInputs": self.inputs(&t.imported_inputs)}),
            ),
            Export(t) => (
                self.base(&t.base),
                json!({
                "destinationChain": self.id(&t.destination_chain),
                "exportedOutputs": self.outputs(&t.exported_outputs)}),
            ),
            RemoveSubnetValidator(t) => (
                self.base(&t.base),
                json!({
                "nodeID": self.node_id(&t.node_id), "subnetID": self.id(&t.subnet),
                "subnetAuthorization": self.auth(&t.subnet_auth)}),
            ),
            TransformSubnet(t) => (
                self.base(&t.base),
                json!({
                "subnetID": self.id(&t.subnet), "assetID": self.id(&t.asset_id),
                "initialSupply": t.initial_supply, "maximumSupply": t.maximum_supply,
                "minConsumptionRate": t.min_consumption_rate, "maxConsumptionRate": t.max_consumption_rate,
                "minValidatorStake": t.min_validator_stake, "maxValidatorStake": t.max_validator_stake,
                "minStakeDuration": t.min_stake_duration, "maxStakeDuration": t.max_stake_duration,
                "minDelegationFee": t.min_delegation_fee, "minDelegatorStake": t.min_delegator_stake,
                "maxValidatorWeightFactor": t.max_validator_weight_factor,
                "uptimeRequirement": t.uptime_requirement, "subnetAuthorization": self.auth(&t.subnet_auth)}),
            ),
            AddPermissionlessValidator(t) => (
                self.base(&t.base),
                json!({
                "validator": self.validator(&t.validator), "subnetID": self.id(&t.subnet),
                "signer": self.signer(&t.signer), "stake": self.outputs(&t.stake_outs),
                "validationRewardsOwner": self.owner(&t.validator_rewards_owner),
                "delegationRewardsOwner": self.owner(&t.delegator_rewards_owner),
                "shares": t.delegation_shares}),
            ),
            AddPermissionlessDelegator(t) => (
                self.base(&t.base),
                json!({
                "validator": self.validator(&t.validator), "subnetID": self.id(&t.subnet),
                "stake": self.outputs(&t.stake_outs), "rewardsOwner": self.owner(&t.delegation_rewards_owner)}),
            ),
            TransferSubnetOwnership(t) => (
                self.base(&t.base),
                json!({
                "subnetID": self.id(&t.subnet), "subnetAuthorization": self.auth(&t.subnet_auth),
                "newOwner": self.owner(&t.owner)}),
            ),
            ConvertSubnetToL1(t) => (
                self.base(&t.base),
                json!({
                "subnetID": self.id(&t.subnet), "chainID": self.id(&t.chain_id), "address": hex0x(&t.address),
                "validators": t.validators.iter().map(|v| json!({
                    "nodeID": hex0x(&v.node_id), "weight": v.weight, "balance": v.balance,
                    "signer": self.pop(&v.signer),
                    "remainingBalanceOwner": self.pchain_owner(&v.remaining_balance_owner),
                    "deactivationOwner": self.pchain_owner(&v.deactivation_owner)})).collect::<Vec<_>>(),
                "subnetAuthorization": self.auth(&t.subnet_auth)}),
            ),
            RegisterL1Validator(t) => (
                self.base(&t.base),
                json!({
                "balance": t.balance, "proofOfPossession": t.proof_of_possession.to_vec(),
                "message": hex0x(&t.message)}),
            ),
            SetL1ValidatorWeight(t) => (self.base(&t.base), json!({"message": hex0x(&t.message)})),
            IncreaseL1ValidatorBalance(t) => (
                self.base(&t.base),
                json!({
                "validationID": self.id(&t.validation_id), "balance": t.balance}),
            ),
            DisableL1Validator(t) => (
                self.base(&t.base),
                json!({
                "validationID": self.id(&t.validation_id), "disableAuthorization": self.auth(&t.disable_auth)}),
            ),
            AddAutoRenewedValidator(t) => (
                self.base(&t.base),
                json!({
                "nodeID": hex0x(&t.validator_node_id), "signer": self.signer(&t.signer),
                "stake": self.outputs(&t.stake_outs),
                "validationRewardsOwner": self.owner(&t.validator_rewards_owner),
                "delegationRewardsOwner": self.owner(&t.delegator_rewards_owner),
                "validatorAuthority": self.owner(&t.validator_authority),
                "delegationShares": t.delegation_shares,
                "autoCompoundRewardShares": t.auto_compound_reward_shares, "period": t.period}),
            ),
            SetAutoRenewedValidatorConfig(t) => (
                self.base(&t.base),
                json!({
                "txID": self.id(&t.tx_id), "auth": self.auth(&t.auth),
                "autoCompoundRewardShares": t.auto_compound_reward_shares, "period": t.period}),
            ),
        };
        m.extend(obj(extra));
        Value::Object(m)
    }

    fn tx(&self, tx: &Tx) -> Value {
        let creds: Vec<Value> = tx
            .creds
            .iter()
            .map(|Cred::Secp256k1(c)| json!({"signatures": c.sigs.iter().map(|s| hex0x(s)).collect::<Vec<_>>()}))
            .collect();
        json!({"unsignedTx": self.unsigned(&tx.unsigned), "credentials": creds, "id": cb58(&tx.id())})
    }

    fn block(&self, b: &Block) -> Value {
        let txs = |list: &[Tx]| list.iter().map(|t| self.tx(t)).collect::<Vec<_>>();
        let common =
            |parent: &Id, height: u64| json!({"parentID": self.id(parent), "height": height});
        let mut v = match b {
            Block::ApricotProposal(b) => merge(
                common(&b.parent_id, b.height),
                json!({"tx": self.tx(&b.tx)}),
            ),
            Block::ApricotAbort(b) => common(&b.parent_id, b.height),
            Block::ApricotCommit(b) => common(&b.parent_id, b.height),
            Block::ApricotStandard(b) => merge(
                common(&b.parent_id, b.height),
                json!({"txs": txs(&b.transactions)}),
            ),
            Block::ApricotAtomic(b) => merge(
                common(&b.parent_id, b.height),
                json!({"tx": self.tx(&b.tx)}),
            ),
            Block::BanffProposal(b) => merge(
                common(&b.parent_id, b.height),
                json!({"time": b.time, "txs": txs(&b.transactions), "tx": self.tx(&b.tx)}),
            ),
            Block::BanffAbort(b) => merge(common(&b.parent_id, b.height), json!({"time": b.time})),
            Block::BanffCommit(b) => merge(common(&b.parent_id, b.height), json!({"time": b.time})),
            Block::BanffStandard(b) => merge(
                common(&b.parent_id, b.height),
                json!({"time": b.time, "txs": txs(&b.transactions)}),
            ),
        };
        v["id"] = json!(cb58(&b.id()));
        v
    }
}

fn obj(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => unreachable!(),
    }
}

fn merge(a: Value, b: Value) -> Value {
    let mut m = obj(a);
    m.extend(obj(b));
    Value::Object(m)
}

fn strip_fx_id(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.remove("fxID");
            m.values_mut().for_each(strip_fx_id);
        }
        Value::Array(a) => a.iter_mut().for_each(strip_fx_id),
        _ => {}
    }
}

#[test]
fn mainnet_blocks_match_the_nodes_json() {
    let path =
        std::env::var("AVAX_BLOCKS").unwrap_or_else(|_| "vectors/mainnet-blocks.jsonl".into());
    let ctx = Ctx { hrp: "avax" };
    let mut seen = std::collections::BTreeSet::new();
    let mut count = 0;
    for line in std::fs::read_to_string(&path).unwrap().lines() {
        let rec: Value = serde_json::from_str(line).unwrap();
        let bytes = hex::decode(rec["hex"].as_str().unwrap()).unwrap();
        let height = &rec["height"];
        let block = Block::from_bytes(&bytes).unwrap_or_else(|e| panic!("height {height}: {e}"));
        assert_eq!(block.to_bytes(), bytes, "height {height}: re-encoding");
        let mut node = rec["json"].clone();
        strip_fx_id(&mut node);
        let mine = ctx.block(&block);
        if mine != node {
            panic!(
                "height {height}: JSON differs\nmine: {}\nnode: {}",
                serde_json::to_string(&mine).unwrap(),
                serde_json::to_string(&node).unwrap()
            );
        }
        seen.insert(block.type_id());
        let list: Vec<&Tx> = match &block {
            Block::ApricotProposal(b) => vec![&b.tx],
            Block::ApricotAtomic(b) => vec![&b.tx],
            Block::ApricotStandard(b) => b.transactions.iter().collect(),
            Block::BanffProposal(b) => b.transactions.iter().chain([&b.tx]).collect(),
            Block::BanffStandard(b) => b.transactions.iter().collect(),
            _ => vec![],
        };
        for tx in list {
            seen.insert(1000 + tx.unsigned.type_id());
        }
        count += 1;
    }
    eprintln!("{count} blocks; block types and 1000+tx types seen: {seen:?}");
    assert!(count > 0);
}
