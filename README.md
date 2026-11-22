# avax-tx

A Rust library to decode, encode and sign Avalanche P-Chain transactions and
blocks, checked byte for byte against avalanchego.

It covers every transaction type avalanchego v1.15 accepts on the P-Chain,
including the L1 types introduced by Etna (ConvertSubnetToL1,
RegisterL1Validator, SetL1ValidatorWeight, IncreaseL1ValidatorBalance,
DisableL1Validator) and the auto-renewed validator types introduced by
Helicon, plus all nine block types.

**Status: prototype.** Not audited, not on crates.io, and the API will
change. Don't use it to sign transactions that move real funds yet.

## Why

The official SDKs are in Go and TypeScript. ava-labs/avalanche-rs, the Rust
SDK, was archived in October 2026; its code had not been updated since 2024
and doesn't know the L1 transaction types. Rust code that reads P-Chain
transactions today carries its own partial parser, as the Ledger and Keystone
Avalanche apps do. This crate is meant to be one shared codec, written from
avalanchego (not from avalanche-rs) and tested against it.

## How it is checked

| Check | What it proves |
|-------|----------------|
| [avalanchego's own vectors](vectors/pchain-avalanchego.json): 29 byte strings from the P-Chain serialization tests of v1.15.3, extracted by [a script](tools/extract_vectors.py) | Each decodes, has the type ID its bytes name, and encodes back to the same bytes. Every truncation of each is rejected. |
| [78 real Mainnet blocks](vectors/mainnet-blocks.jsonl), fetched by [a script](tools/fetch_blocks.py) as raw bytes and as the node's own JSON | Each decodes and encodes back to the same bytes, and written out as avalanchego writes JSON, it equals the node's JSON field for field, including transaction and block IDs recomputed here. |
| avalanchego's secp256k1 test vectors | The same address, public key and signatures, byte for byte (RFC 6979 nonces, low-S). High-S signatures and recovery IDs other than 0 and 1 are rejected, as avalanchego does. |

Coverage by type (the Mainnet column counts transactions decoded and compared
with the node):

| Type ID | Transaction | avalanchego vectors | Mainnet |
|---|---|---|---|
| 12 | AddValidator | | yes |
| 13 | AddSubnetValidator | | yes |
| 14 | AddDelegator | | yes |
| 15 | CreateChain | | yes |
| 16 | CreateSubnet | | yes |
| 17 | Import | | yes |
| 18 | Export | | yes |
| 19 | AdvanceTime | | yes |
| 20 | RewardValidator | | yes |
| 23 | RemoveSubnetValidator | 2 | yes |
| 24 | TransformSubnet | 2 | |
| 25 | AddPermissionlessValidator | 4 | yes |
| 26 | AddPermissionlessDelegator | 4 | yes |
| 33 | TransferSubnetOwnership | 2 | yes |
| 34 | Base | 2 | yes |
| 35 | ConvertSubnetToL1 | 2 | yes |
| 36 | RegisterL1Validator | 1 | yes |
| 37 | SetL1ValidatorWeight | 1 | yes |
| 38 | IncreaseL1ValidatorBalance | 1 | yes |
| 39 | DisableL1Validator | 1 | yes |
| 40 | AddAutoRenewedValidator | 1 | yes |
| 41 | SetAutoRenewedValidatorConfig | 1 | |
| 42 | RewardAutoRenewedValidator | 1 | |

Blocks: Apricot proposal, abort, commit, standard and atomic (types 0 to 4)
from Mainnet; Banff proposal, commit and standard (29, 31, 32) from both;
Banff abort (30) from the vectors.

## Use

```rust
use avax_tx::pchain::{Tx, UnsignedTx};
use avax_tx::sign;

// Decode a transaction someone asks you to sign, and look at it.
let unsigned = UnsignedTx::from_bytes(&bytes)?;
if let UnsignedTx::Base(base) = &unsigned {
    for out in &base.outs { /* amounts, owners */ }
}

// Sign it: one entry per input, each with the keys for that input.
let tx: Tx = sign::sign(unsigned, &[vec![&key]]);
let signed_bytes = tx.to_bytes();
let tx_id = tx.id();
```

## Not done yet

- X-Chain (AVM) and C-Chain atomic transactions.
- Parsing the Warp messages inside RegisterL1Validator and
  SetL1ValidatorWeight (kept as bytes for now).
- Fees: the ACP-103 complexity calculation.
- The syntactic checks avalanchego runs before accepting a transaction
  (sorted inputs and outputs, sorted addresses, memo length).
- Building transactions from UTXOs.

## Updating the vectors

```sh
# from an avalanchego checkout at a release tag
python -I tools/extract_vectors.py <avalanchego> vectors/pchain-avalanchego.json
# more Mainnet blocks, by height
python -I tools/fetch_blocks.py vectors/mainnet-blocks.jsonl <height> ...
```

## License

MIT, see [LICENSE](LICENSE). The test vectors include byte strings from
avalanchego (BSD-3-Clause) and public Mainnet data; see
[vectors/NOTICE](vectors/NOTICE).
