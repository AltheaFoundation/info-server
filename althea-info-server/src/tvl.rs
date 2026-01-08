use cosmos_sdk_proto_althea::cosmos::{
    bank::v1beta1::{query_client::QueryClient as BankQueryClient, QueryTotalSupplyRequest},
    base::{query::v1beta1::PageRequest, v1beta1::Coin},
};
use cosmos_sdk_proto_althea::ibc::applications::transfer::v1::{
    query_client::QueryClient as IbcTransferQueryClient, QueryEscrowAddressRequest,
};
use num256::Uint256;
use rust_decimal::prelude::Zero;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::sleep;

use crate::{
    config::{get_token, get_tokens, Token},
    total_suppy::get_supply_info,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tvl {
    pub althea_on_chain: TokenAmount,
    pub ibc_tokens_on_chain: Vec<TokenAmount>,
    pub althea_native_erc20s_on_chain: Vec<TokenAmount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAmount {
    pub token: Token,
    pub amount: Uint256,
}

/// Type alias for the total supply result from the Cosmos bank module
pub type TotalSupply = Vec<Coin>;

pub const ALTHEA_TOKEN_DENOM: &str = "aalthea";

// Fetches and computes the supply of bridged IBC tokens, native althea (from the total supply thread info), and altheaL1-native erc20s
pub async fn get_unpriced_tvl(grpc: String) -> Result<Tvl, String> {
    let supply = get_total_supply(&grpc).await?;
    let supply = filter_supply_by_tokens(supply);
    let tokens_on_chain = get_tokens_on_chain(&supply, &grpc).await?;
    let althea_supply =
        get_supply_info().map_or_else(Uint256::zero, |info| info.total_liquid_supply);

    let althea_on_chain = TokenAmount {
        token: get_token(ALTHEA_TOKEN_DENOM).unwrap(),
        amount: althea_supply,
    };

    let ibc_tokens_on_chain: Vec<TokenAmount> = tokens_on_chain
        .clone()
        .into_iter()
        .filter_map(|(t, v)| {
            if t.althea_denom.starts_with("ibc/") {
                return Some(TokenAmount {
                    token: t,
                    amount: v,
                });
            }
            None
        })
        .collect();

    let althea_native_erc20s_on_chain: Vec<TokenAmount> = tokens_on_chain
        .clone()
        .into_iter()
        .filter_map(|(t, v)| {
            if !t.althea_denom.starts_with("ibc/") && t.althea_denom != ALTHEA_TOKEN_DENOM {
                return Some(TokenAmount {
                    token: t,
                    amount: v,
                });
            }
            None
        })
        .collect();
    let tvl = Tvl {
        althea_on_chain,
        ibc_tokens_on_chain,
        althea_native_erc20s_on_chain,
    };
    Ok(tvl)
}

pub fn filter_supply_by_tokens(supply: TotalSupply) -> TotalSupply {
    let tokens = get_tokens();
    supply
        .into_iter()
        .filter(|coin| {
            tokens
                .values()
                .any(|token| token.althea_denom == coin.denom)
        })
        .collect()
}

/// Gets the total supply of all tokens from the Cosmos bank module
pub async fn get_total_supply(grpc: &str) -> Result<TotalSupply, String> {
    let mut next_key: Option<Vec<u8>> = None;
    let mut all_supply = Vec::new();

    // Create a client and connect to the appropriate URL
    let mut client = BankQueryClient::connect(grpc.to_string())
        .await
        .map_err(|e| format!("Failed to connect to gRPC endpoint: {e}"))?;

    // Loop until we've retrieved all supply entries
    loop {
        let request = QueryTotalSupplyRequest {
            pagination: Some(PageRequest {
                key: next_key.unwrap_or_default(),
                offset: 0,
                limit: 100, // Request a reasonable batch size
                count_total: false,
                reverse: false,
            }),
        };

        // Retry logic for the total_supply request
        const MAX_RETRIES: u32 = 3;
        let mut response = None;

        for attempt in 1..=MAX_RETRIES {
            match client.total_supply(request.clone()).await {
                Ok(resp) => {
                    response = Some(resp);
                    break;
                }
                Err(e) => {
                    if attempt < MAX_RETRIES {
                        log::warn!(
                            "Failed to query total supply (attempt {}/{}): {}, retrying...",
                            attempt,
                            MAX_RETRIES,
                            e
                        );
                        sleep(tokio::time::Duration::from_millis(100 * attempt as u64)).await;
                    } else {
                        return Err(format!(
                            "Error querying total supply after {} attempts: {}",
                            MAX_RETRIES, e
                        ));
                    }
                }
            }
        }

        let response_inner = response
            .expect("Response should be Some after successful retry")
            .into_inner();

        // Add supply from this page to our result
        all_supply.extend(response_inner.supply);

        // Check if there are more pages
        if let Some(page_response) = response_inner.pagination {
            if !page_response.next_key.is_empty() {
                next_key = Some(page_response.next_key);
            } else {
                break; // No more pages
            }
        } else {
            break; // No pagination response means no more pages
        }
    }

    Ok(all_supply)
}

pub fn get_ibc_tokens_bridged_in(supply: &TotalSupply) -> HashMap<Token, Uint256> {
    let mut result = HashMap::new();

    for (_token_name, token) in get_tokens() {
        if token.althea_denom.starts_with("ibc/") {
            result.insert(token.clone(), Uint256::zero());
        }
    }

    // Filter for IBC tokens (denoms starting with "ibc/")
    for coin in supply {
        if coin.denom.starts_with("ibc/") {
            // Find the token in our result map that matches this denom
            for (token, amount) in result.iter_mut() {
                if token.althea_denom == coin.denom {
                    // Parse the coin amount and add it to the existing value
                    match coin.amount.parse::<Uint256>() {
                        Ok(parsed_amount) => {
                            *amount += parsed_amount;
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to parse amount '{}' for denom {}: {}",
                                coin.amount,
                                coin.denom,
                                e
                            );
                        }
                    }
                    break;
                }
            }
        }
    }

    result
}

pub async fn get_tokens_on_chain(
    supply: &TotalSupply,
    grpc: &str,
) -> Result<HashMap<Token, Uint256>, String> {
    let mut result = HashMap::new();

    // Create clients for IBC transfer and bank queries
    let mut ibc_client = IbcTransferQueryClient::connect(grpc.to_string())
        .await
        .map_err(|e| format!("Failed to connect to IBC transfer gRPC endpoint: {e}"))?;

    let ibc_port = "transfer".to_string();
    let channel_0_id = "channel-0".to_string();
    let mut channel0_escrow_address: String = String::new();
    const MAX_RETRIES: u32 = 3;
    for _ in 0..MAX_RETRIES {
        if let Ok(response) = ibc_client
            .escrow_address(QueryEscrowAddressRequest {
                port_id: ibc_port.clone(),
                channel_id: channel_0_id.clone(),
            })
            .await
        {
            channel0_escrow_address = response.into_inner().escrow_address;
            break;
        }
    }
    if channel0_escrow_address.is_empty() {
        return Err("Failed to get escrow address for channel-0 after retries".to_string());
    }

    let mut bank_client = BankQueryClient::connect(grpc.to_string())
        .await
        .map_err(|e| format!("Failed to connect to bank gRPC endpoint: {e}"))?;

    // Process each coin in the supply
    for coin in supply {
        // Find the matching token from our config
        let token = get_token(&coin.denom);

        if let Some(token) = token {
            // Parse the total supply amount
            let total_supply: Uint256 = match coin.amount.parse() {
                Ok(amount) => amount,
                Err(e) => {
                    log::error!(
                        "Failed to parse supply amount '{}' for denom {}: {}",
                        coin.amount,
                        coin.denom,
                        e
                    );
                    continue;
                }
            };

            // Get the escrow address for this token
            // Standard IBC transfer port is "transfer", channel needs to come from token config
            // TODO: Get channel_id from token configuration
            let port_id = "transfer".to_string();
            let channel_id = if let Some(ref channel) = token.ibc_channel {
                channel.clone()
            } else {
                // If no channel configured, assume all tokens are on-chain (no escrow)
                result.insert(token.clone(), total_supply);
                continue;
            };

            let escrow_address: String = if port_id == ibc_port && channel_id == channel_0_id {
                channel0_escrow_address.clone()
            } else {
                let escrow_request = QueryEscrowAddressRequest {
                    port_id: port_id.clone(),
                    channel_id: channel_id.clone(),
                };

                match ibc_client.escrow_address(escrow_request).await {
                    Ok(response) => response.into_inner().escrow_address,
                    Err(e) => {
                        log::warn!(
                            "Failed to get escrow address for {}/{}: {}, assuming no escrow",
                            port_id,
                            channel_id,
                            e
                        );
                        // If we can't get escrow address, assume all tokens are on-chain
                        result.insert(token.clone(), total_supply);
                        continue;
                    }
                }
            };

            // Query the balance at the escrow address for this denom
            let balance_request =
                cosmos_sdk_proto_althea::cosmos::bank::v1beta1::QueryBalanceRequest {
                    address: escrow_address.clone(),
                    denom: coin.denom.clone(),
                };

            let escrowed_amount: Uint256 = match bank_client.balance(balance_request).await {
                Ok(response) => {
                    if let Some(balance) = response.into_inner().balance {
                        match balance.amount.parse() {
                            Ok(amount) => amount,
                            Err(e) => {
                                log::error!(
                                    "Failed to parse escrowed amount '{}' for denom {}: {}",
                                    balance.amount,
                                    coin.denom,
                                    e
                                );
                                Uint256::zero()
                            }
                        }
                    } else {
                        Uint256::zero()
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to query balance for escrow address {}: {}, assuming zero escrow",
                        escrow_address,
                        e
                    );
                    Uint256::zero()
                }
            };

            // Calculate on-chain balance: total supply - escrowed amount
            let on_chain_amount = if total_supply >= escrowed_amount {
                total_supply - escrowed_amount
            } else {
                log::error!(
                    "Escrowed amount ({}) exceeds total supply ({}) for denom {}",
                    escrowed_amount,
                    total_supply,
                    coin.denom
                );
                Uint256::zero()
            };

            result.insert(token.clone(), on_chain_amount);
        }
    }

    Ok(result)
}
