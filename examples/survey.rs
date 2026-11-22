//! Decodes every block of a JSONL file written by tools/fetch_blocks.py,
//! checks it encodes back to the same bytes, and counts block and
//! transaction types.
use std::collections::BTreeMap;

use avax_tx::pchain::Block;

fn main() {
    let path = std::env::args().nth(1).expect("blocks.jsonl");
    let (mut blocks, mut txs) = (BTreeMap::new(), BTreeMap::new());
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        let bytes = hex::decode(v["hex"].as_str().unwrap()).unwrap();
        let block =
            Block::from_bytes(&bytes).unwrap_or_else(|e| panic!("height {}: {e}", v["height"]));
        assert_eq!(block.to_bytes(), bytes, "height {}", v["height"]);
        *blocks.entry(block.type_id()).or_insert(0) += 1;
        let list: Vec<&avax_tx::pchain::Tx> = match &block {
            Block::ApricotProposal(b) => vec![&b.tx],
            Block::ApricotAtomic(b) => vec![&b.tx],
            Block::ApricotStandard(b) => b.transactions.iter().collect(),
            Block::BanffProposal(b) => b
                .transactions
                .iter()
                .chain(std::iter::once(&b.tx))
                .collect(),
            Block::BanffStandard(b) => b.transactions.iter().collect(),
            _ => vec![],
        };
        for tx in list {
            txs.entry(tx.unsigned.type_id())
                .or_insert_with(Vec::new)
                .push(v["height"].as_u64().unwrap());
        }
    }
    println!("block types: {blocks:?}");
    for (t, hs) in &txs {
        println!("tx type {t}: {} (e.g. height {})", hs.len(), hs[0]);
    }
}
