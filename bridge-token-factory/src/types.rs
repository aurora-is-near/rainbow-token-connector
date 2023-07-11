use std::fmt;
use std::str::FromStr;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Debug, Clone, PartialEq)]
pub struct EthAddressHex(pub String);

impl<'de> Deserialize<'de> for EthAddressHex {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as Deserialize>::deserialize(deserializer)?;
        EthAddressHex::from_str(&s).map_err(|e| serde::de::Error::custom(e.0))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseEthAddressError(String);

impl fmt::Display for ParseEthAddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The hex eth address is invalid")
    }
}

impl FromStr for EthAddressHex {
    type Err = ParseEthAddressError;
    fn from_str(s: &str) -> Result<EthAddressHex, Self::Err> {
        let s = if s.starts_with("0x") {
            s[2..].to_lowercase()
        } else {
            s.to_lowercase()
        };

        if !is_hex_string(&s) {
            return Err(ParseEthAddressError("Invalid hex character".to_owned()));
        }
        if s.len() != 40 {
            return Err(ParseEthAddressError(
                "Address should be 20 bytes long".to_owned(),
            ));
        }

        Ok(EthAddressHex(s))
    }
}

fn is_hex_string(hex_str: &str) -> bool {
    for c in hex_str.chars() {
        if !c.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeeBounds {
    pub lower_bound: U128,
    pub upper_bound: U128,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DepositFee {
    pub fee_percentage: DepositFeePercentage,
    pub bounds: FeeBounds,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WithdrawFee {
    pub fee_percentage: WithdrawFeePercentage,
    pub bounds: FeeBounds,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DepositFeePercentage {
    pub eth_to_near: U128,
    pub eth_to_aurora: U128,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WithdrawFeePercentage {
    pub near_to_eth: U128,
    pub aurora_to_eth: U128,
}
