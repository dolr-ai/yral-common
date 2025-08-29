use std::fmt::{self, Display, Formatter};

use candid::Principal;
use serde::{Deserialize, Serialize};

use super::token::balance::TokenBalance;

#[derive(Clone, Copy)]
pub enum TxnDirection {
    Transaction,
    Added,
    Deducted,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub enum TxnInfoType {
    Mint { to: Principal },
    Sent { to: Principal }, // only for keyed
    Burn { from: Principal },
    Received { from: Principal },                // only for keyed
    Transfer { from: Principal, to: Principal }, // only for public transaction
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct TxnInfoWallet {
    pub tag: TxnInfoType,
    pub timestamp: u64,
    pub amount: TokenBalance,
    pub id: u64,
}

impl From<TxnInfoType> for TxnDirection {
    fn from(value: TxnInfoType) -> TxnDirection {
        match value {
            TxnInfoType::Burn { .. } | TxnInfoType::Sent { .. } => TxnDirection::Deducted,
            TxnInfoType::Mint { .. } | TxnInfoType::Received { .. } => TxnDirection::Added,
            TxnInfoType::Transfer { .. } => TxnDirection::Transaction,
        }
    }
}

impl TxnInfoType {
    pub fn to_text(self) -> &'static str {
        match self {
            TxnInfoType::Burn { .. } => "Burned",
            TxnInfoType::Mint { .. } => "Minted",
            TxnInfoType::Received { .. } => "Received",
            TxnInfoType::Sent { .. } => "Sent",
            TxnInfoType::Transfer { .. } => "Transferred",
        }
    }
}

impl Display for TxnInfoType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_text())
    }
}
