//! Build, decode and sign Avalanche P-Chain transactions.
//!
//! Every transaction type avalanchego v1.15 accepts on the P-Chain is here,
//! including the L1 types (ConvertSubnetToL1, RegisterL1Validator,
//! SetL1ValidatorWeight, IncreaseL1ValidatorBalance, DisableL1Validator)
//! and the Helicon auto-renewed validator types, plus blocks. The encoding
//! is checked byte for byte against avalanchego's own serialization tests
//! (`vectors/`).
//!
//! ```
//! use avax_tx::pchain::UnsignedTx;
//! # let bytes = hex_literal();
//! # fn hex_literal() -> Vec<u8> { vec![0, 0, 0, 0, 0, 42, 0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0, 0, 0, 0, 0, 0, 0, 7] }
//! let tx = UnsignedTx::from_bytes(&bytes)?;
//! assert_eq!(tx.type_id(), 42);
//! assert_eq!(tx.to_bytes(), bytes);
//! # Ok::<(), avax_tx::codec::Error>(())
//! ```

pub mod codec;
pub mod components;
pub mod pchain;
pub mod sign;
