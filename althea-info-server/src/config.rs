use std::collections::HashMap;

use clarity::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[allow(non_snake_case)]
pub struct Token {
    /// The Althea L1 EVM address of the token
    pub althea_evm_address: Address,
    /// The denom of this token on Althea L1 on the Cosmos layer (IBC tokens will have ibc/ prefix)
    pub althea_denom: String,
    /// The erc20 address of this token on Ethereum, may be null if this token does not exist there
    pub eth_address: Option<Address>,
    /// the source ibc channel of this token, may be null if this token is not an IBC token
    /// IBC port is assumed to be transfer for all tokens
    pub ibc_channel: Option<String>,
    /// The number of decimal places the token uses
    pub decimals: u32,
    /// The name of the token (multiple words)
    pub name: String,
    /// The symbol of the token (small number of letters)
    pub symbol: String,
    /// The coingecko ID of this token for price lookups, essential for the DefiLlama integration
    pub coingecko_id: String,
}

pub fn get_token(token: &str) -> Option<Token> {
    let tokens = get_tokens();
    tokens
        .values()
        .find(|t| {
            t.althea_evm_address.to_string() == token
                || t.althea_denom == token
                || {
                    if let Some(eth_address) = &t.eth_address {
                        eth_address.to_string() == token
                    } else {
                        false
                    }
                }
                || t.name == token
                || t.symbol == token
        })
        .cloned()
}

pub fn get_tokens() -> HashMap<String, Token> {
    let mut tokens = HashMap::new();
    let althea = Token {
        althea_evm_address: "0x0000000000000000000000000000000000000000"
            .parse()
            .unwrap(),
        althea_denom: "aalthea".to_string(),
        ibc_channel: Some("channel-0".to_string()),
        eth_address: Some(
            "0xF9e595BC0aF20cfa1561dfE085E3DE9Fcf9Fbfa2"
                .parse()
                .unwrap(),
        ),
        decimals: 18,
        name: "ALTHEA".to_string(),
        symbol: "ALTHEA".to_string(),
        coingecko_id: "althea".to_string(),
    };
    tokens.insert(althea.symbol.clone(), althea);

    let usdc = Token {
        althea_evm_address: "0x80b5a32E4F032B2a058b4F29EC95EEfEEB87aDcd"
            .parse()
            .unwrap(),
        eth_address: Some(
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                .parse()
                .unwrap(),
        ),
        althea_denom: "ibc/17CD484EE7D9723B847D95015FA3EBD1572FD13BC84FB838F55B18A57450F25B"
            .to_string(),
        ibc_channel: Some("channel-0".to_string()),
        decimals: 6,
        name: "Circle USD Stablecoin".to_string(),
        symbol: "USDC".to_string(),
        coingecko_id: "usdc".to_string(),
    };
    tokens.insert(usdc.symbol.clone(), usdc);

    let usdt = Token {
        althea_evm_address: "0xecEEEfCEE421D8062EF8d6b4D814efe4dc898265"
            .parse()
            .unwrap(),
        eth_address: Some(
            "0xdAC17F958D2ee523a2206206994597C13D831ec7"
                .parse()
                .unwrap(),
        ),
        althea_denom: "ibc/4F6A2DEFEA52CD8D90966ADCB2BD0593D3993AB0DF7F6AEB3EFD6167D79237B0"
            .to_string(),
        ibc_channel: Some("channel-0".to_string()),
        decimals: 6,
        name: "Tether Stablecoin".to_string(),
        symbol: "USDT".to_string(),
        coingecko_id: "tether".to_string(),
    };
    tokens.insert(usdt.symbol.clone(), usdt);

    let usds = Token {
        althea_evm_address: "0xd567B3d7B8FE3C79a1AD8dA978812cfC4Fa05e75"
            .parse()
            .unwrap(),
        althea_denom: "ibc/AE1B617F7F329ED83C20AC584B03579EEFE3322EF601CE88936A0271BE1157DD"
            .to_string(),
        eth_address: Some(
            "0xdC035D45d973E3EC169d2276DDab16f1e407384F"
                .parse()
                .unwrap(),
        ),
        ibc_channel: Some("channel-0".to_string()),
        decimals: 18,
        name: "USDS".to_string(),
        symbol: "USDS".to_string(),
        coingecko_id: "usds".to_string(),
    };
    tokens.insert(usds.symbol.clone(), usds);

    let susds = Token {
        althea_evm_address: "0x5FD55A1B9FC24967C4dB09C513C3BA0DFa7FF687"
            .parse()
            .unwrap(),
        eth_address: Some(
            "0xa3931d71877C0E7a3148CB7Eb4463524FEc27fbD"
                .parse()
                .unwrap(),
        ),
        althea_denom: "ibc/576150049104D47DFD447482EEED2FC8B44AB0D9A772D673717062B49D9820C5"
            .to_string(),
        ibc_channel: Some("channel-0".to_string()),
        decimals: 18,
        name: "Savings USDS".to_string(),
        symbol: "sUSDS".to_string(),
        coingecko_id: "susds".to_string(),
    };
    tokens.insert(susds.symbol.clone(), susds);

    let grav = Token {
        althea_evm_address: "0x1D54EcB8583Ca25895c512A8308389fFD581F9c9"
            .parse()
            .unwrap(),
        eth_address: Some(
            "0x9f2ef66a09A5d2dAB13D84A7638668EA36679e03"
                .parse()
                .unwrap(),
        ),
        althea_denom: "ibc/FC9D92EC12BC974E8B6179D411351524CD5C2EBC3CE29D5BA856414FEFA47093"
            .to_string(),
        ibc_channel: Some("channel-0".to_string()),
        decimals: 6,
        name: "Graviton".to_string(),
        symbol: "GRAV".to_string(),
        coingecko_id: "graviton".to_string(),
    };
    tokens.insert(grav.symbol.clone(), grav);

    let weth = Token {
        althea_evm_address: "0xc03345448969Dd8C00e9E4A85d2d9722d093aF8E"
            .parse()
            .unwrap(),
        eth_address: Some(
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                .parse()
                .unwrap(),
        ),
        althea_denom: "ibc/DC186CA7A8C009B43774EBDC825C935CABA9743504CE6037507E6E5CCE12858A"
            .to_string(),
        ibc_channel: Some("channel-0".to_string()),
        decimals: 18,
        name: "Ethereum".to_string(),
        symbol: "WETH".to_string(),
        coingecko_id: "weth".to_string(),
    };
    tokens.insert(weth.symbol.clone(), weth);

    tokens
}
