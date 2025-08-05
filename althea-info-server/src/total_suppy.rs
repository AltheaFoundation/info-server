//! Yes computing the total supply of tokens on the chain does in fact require all the junk in this file.
//! This is mostly complexity around vesting and the fact that there's no convient function to total everything up
//! on the server side. Logic would dictate that you make the endpoint on client but this code uses highly parallel rust futures
//! to effectively query all the data in a reasonable amount of time and compute the result locally
//! This code provides a generic way to compute the total liquid supply for a cosmos chain across all vesting types

use crate::{ALTHEA_NODE_GRPC, ALTHEA_PREFIX, REQUEST_TIMEOUT};
use actix_web::rt::System;
use cosmos_sdk_proto_althea::cosmos::bank::v1beta1::query_client::QueryClient as BankQueryClient;
use cosmos_sdk_proto_althea::cosmos::bank::v1beta1::QueryBalanceRequest;
use cosmos_sdk_proto_althea::cosmos::distribution::v1beta1::query_client::QueryClient as DistQueryClient;
use cosmos_sdk_proto_althea::cosmos::distribution::v1beta1::QueryDelegationTotalRewardsRequest;
use cosmos_sdk_proto_althea::cosmos::staking::v1beta1::query_client::QueryClient as StakingQueryClient;
use cosmos_sdk_proto_althea::cosmos::staking::v1beta1::QueryDelegatorDelegationsRequest;
use cosmos_sdk_proto_althea::cosmos::vesting::v1beta1::BaseVestingAccount;
use deep_space::client::types::AccountType;
use deep_space::client::PAGE;
use deep_space::error::CosmosGrpcError;
use deep_space::{Coin, Contact};
use futures::future::{join3, join_all};
use log::{error, info, trace};
use num256::Uint256;
use serde::Serialize;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tonic::transport::channel::Channel;

// update once a day
const LOOP_TIME: Duration = Duration::from_secs(86400);
pub const ALTHEA_DENOM: &str = "aalthea";

#[derive(Debug, Clone, Serialize)]
pub struct ChainTotalSupplyNumbers {
    /// The total amount of Graviton tokens currently in existance including those vesting and in the community pool
    pub total_supply: Uint256,
    /// The total amount of Gravition in the community pool
    pub community_pool: Uint256,
    /// All tokens that are 'liquid' meaning in a balance, claimable now as rewards
    /// or staked and eligeable to withdraw and spend, essentially just exludes vesting
    pub total_liquid_supply: Uint256,
    /// All tokens that are in users balances and can be sent instantly
    pub total_liquid_balances: Uint256,
    /// All tokens that are unclaimed as rewards
    pub total_unclaimed_rewards: Uint256,
    /// All tokens staked, but not vesting
    pub total_nonvesting_staked: Uint256,
    /// All tokens not yet vested, including those staked
    pub total_vesting: Uint256,
    /// All tokens that are vesting and staked
    pub total_vesting_staked: Uint256,
    /// All tokens that have vested so far
    pub total_vested: Uint256,
}

lazy_static! {
    static ref TOTAL_SUPPLY: Arc<RwLock<Option<ChainTotalSupplyNumbers>>> =
        Arc::new(RwLock::new(None));
}

fn set_supply_info(input: ChainTotalSupplyNumbers) {
    let mut r = TOTAL_SUPPLY.write().unwrap();
    *r = Some(input);
}

pub fn get_supply_info() -> Option<ChainTotalSupplyNumbers> {
    TOTAL_SUPPLY.read().unwrap().clone()
}

pub fn chain_total_supply_thread() {
    info!("Starting supply calculation thread");

    thread::spawn(move || loop {
        let runner = System::new();
        runner.block_on(async move {
            let contact = Contact::new(ALTHEA_NODE_GRPC, REQUEST_TIMEOUT, ALTHEA_PREFIX).unwrap();
            match compute_liquid_supply(&contact, ALTHEA_DENOM.to_string()).await {
                Ok(v) => {
                    info!("Successfully updated supply info!");
                    set_supply_info(v);
                    thread::sleep(LOOP_TIME);
                }
                Err(e) => error!("Failed to update supply info with {:?}", e),
            }
        });
    });
}

/// This is extremely complicated with vesting, but people want to know
/// so we'll do an estimation, essentially what we need to do is iterate over
/// the entire set of accounts on chain and sum up tokens from non-module accounts
/// taking into account vesting by interpreting the vesting rules ourselves as we go
/// there is no neatly stored value of who has vested how much because doing so would be
/// impractical, if you have 100k vesting accounts that's 100k store changes per block to do
/// continuous vesting, it's intractable so instead liquid amounts are computed when a transfer
/// is attempted we're going to compute it all at once in this function. This function is useful
/// for any cosmos chain using standard vesting
///
/// Returns liquid supply (not including community pool, including staked but liquid tokens)
async fn compute_liquid_supply(
    contact: &Contact,
    denom: String,
) -> Result<ChainTotalSupplyNumbers, CosmosGrpcError> {
    // lets do the easy totals first, grand total and communiy pool
    let totals = contact.query_total_supply().await?;
    let mut total_supply = None;
    for i in totals {
        if i.denom == denom {
            total_supply = Some(i.amount);
        }
    }
    let total_supply = total_supply.unwrap();

    let mut community_pool = None;
    let pool_totals = contact.query_community_pool().await?;
    for i in pool_totals {
        if i.denom == denom {
            community_pool = Some(i.amount);
        }
    }
    let community_pool = community_pool.unwrap();

    let start = Instant::now();
    info!("Starting get all accounts");
    // start by getting every account on chain and every balance for every account
    let accounts = contact.get_all_accounts().await?;
    info!("Got all accounts after {}ms", start.elapsed().as_millis());
    let users = get_balances_for_accounts(accounts, denom.clone()).await?;
    info!(
        "Got all balances/vesting after {}s",
        start.elapsed().as_secs()
    );
    // now that we have every account with every balance we can start computing the totals

    // all the tokens that are 'liquid' including staking rewards and non-vesting staking tokens
    let mut total_liquid_supply: Uint256 = 0u8.into();

    let mut total_liquid_balances: Uint256 = 0u8.into();
    let mut total_unclaimed_rewards: Uint256 = 0u8.into();
    let mut total_nonvesting_staked: Uint256 = 0u8.into();

    let mut total_vesting: Uint256 = 0u8.into();
    let mut total_vested: Uint256 = 0u8.into();
    let mut total_vesting_staked: Uint256 = 0u8.into();

    for user in users {
        match user.account {
            // account with no vesting, simple case, all is liquid
            AccountType::ProtoBaseAccount(_) => {
                total_liquid_balances += user.balance;
                total_nonvesting_staked += user.total_staked;
                total_unclaimed_rewards += user.unclaimed_rewards;

                total_liquid_supply += user.balance;
                total_liquid_supply += user.unclaimed_rewards;
                total_liquid_supply += user.total_staked;
            }
            // In ethermint chains ethermint accounts are module accounts
            AccountType::ModuleAccount(ma) => {
                // here we must skip accounts representing mdoule like minting or governance
                // these accounts are not liquid, but we must include ethermint accounts.
                // the ethermint account will have a 0x address as it's name and a pubkey that is
                // an actual value
                if ma.name.starts_with("0x") && ma.base_account.unwrap().pub_key.is_some() {
                    total_liquid_balances += user.balance;
                    total_nonvesting_staked += user.total_staked;
                    total_unclaimed_rewards += user.unclaimed_rewards;

                    total_liquid_supply += user.balance;
                    total_liquid_supply += user.unclaimed_rewards;
                    total_liquid_supply += user.total_staked;
                } else {
                    // this is not an evm account
                }
            }
            // account with periodic vesting, now we need to determine how much has vested then compare
            // that to their account balance
            AccountType::PeriodicVestingAccount(account_info) => {
                let vesting_start_time =
                    UNIX_EPOCH + Duration::from_secs(account_info.start_time as u64);
                let base = account_info.base_vesting_account.unwrap();
                // delegated vesting may be more than the remaining vesting amount if the user hasn't updated thier delegation
                // since the last vesting event
                let (total_delegated_free, total_delegated_vesting, original_vesting_amount) =
                    sum_vesting(base, denom.clone());
                // obvious stuff requiring no computation
                total_liquid_supply += user.unclaimed_rewards;
                total_liquid_supply += total_delegated_free;
                total_vesting_staked += total_delegated_vesting;
                total_nonvesting_staked += total_delegated_free;

                // vesting has started
                if vesting_start_time < SystemTime::now() {
                    let mut total_amount_vested: Uint256 = 0u8.into();
                    // seconds offset from vesting start time
                    let mut time_counter = 0;
                    for vesting_period in account_info.vesting_periods {
                        time_counter += vesting_period.length;
                        // if this vesting period has already elapsed, add the mount
                        if vesting_start_time + Duration::from_secs(time_counter as u64)
                            <= SystemTime::now()
                        {
                            // hack assumes vesting is only one coin
                            let amount: Coin = vesting_period.amount[0].clone().into();
                            assert_eq!(amount.denom, denom);
                            total_amount_vested += amount.amount;
                        }
                    }
                    assert!(total_amount_vested <= original_vesting_amount);
                    let total_amount_still_vesting = original_vesting_amount - total_amount_vested;

                    total_vested += total_amount_vested;
                    total_vesting += total_amount_still_vesting;

                    // this is a hard edegcase to handle in the current implementation. If someone has delegated and not touched their delegation for a long time
                    // vesting events have elapsed but their delegated vesting number has not been updated. In this case we can be confident that the total_delegated_vesting
                    // is the original amount they have delegated out of their vesting total. So what has vested since then can't be in their balance since that would require them
                    // to interact with thier account and update the total_delegated_vesting number. 
                    let vesting_in_balance = if total_delegated_vesting > total_amount_still_vesting {
                        // vested tokens show up in the balance first, so we take what they originally left in their balance
                        // subtract it from what's vested so far and that's what's delegated and still vesting in this case
                        let org_vest_bal = original_vesting_amount - total_delegated_vesting;
                        let delegated_vesting = total_amount_vested - org_vest_bal;
                        total_amount_vested - delegated_vesting
                    } else {
                        total_amount_still_vesting - total_delegated_vesting
                    };
                    // unvested tokens show up in the balance
                    // but unvested delegated tokens do not, in the case where a user
                    // has some vesting, some delegation, some balance, and some unclaimed rewards
                    assert!(user.balance >= vesting_in_balance);
                    total_liquid_supply += user.balance - vesting_in_balance;
                }
                // vesting has not started yet, in this case we subtract total vesting amount
                // from current balance, if the number is positive (staking could make it negative)
                // we add to our total
                else {
                    assert!(original_vesting_amount >= total_delegated_vesting);
                    let vesting_in_balance = original_vesting_amount - total_delegated_vesting;
                    assert!(total_vested > original_vesting_amount);

                    total_vested += original_vesting_amount;

                    assert!(user.balance > vesting_in_balance);

                    total_liquid_supply += user.balance - vesting_in_balance;
                }
            }
            AccountType::ContinuousVestingAccount(account_info) => {
                let vesting_start_time =
                    UNIX_EPOCH + Duration::from_secs(account_info.start_time as u64);
                let base = account_info.base_vesting_account.unwrap();
                assert!(base.end_time > account_info.start_time);
                let vesting_duration =
                    Duration::from_secs(base.end_time as u64 - account_info.start_time as u64);
                let (total_delegated_free, total_delegated_vesting, original_vesting_amount) =
                    sum_vesting(base, denom.clone());

                // obvious stuff requiring no computation
                total_unclaimed_rewards += user.unclaimed_rewards;
                total_liquid_supply += user.unclaimed_rewards;
                total_liquid_supply += total_delegated_free;
                total_vesting_staked += total_delegated_vesting;
                total_nonvesting_staked += total_delegated_free;

                // vesting has started, since this is continuous we'll do a rough protection
                // between the start and the end time, determine what percentage has elapsed
                // and grant that as liquid
                if vesting_start_time < SystemTime::now() {
                    let elapsed_since_vesting_started = vesting_start_time.elapsed().unwrap();
                    let vesting_percent_complete = elapsed_since_vesting_started.as_secs() as f64
                        / vesting_duration.as_secs() as f64;

                    if vesting_percent_complete > 1.0 {
                        total_liquid_supply += user.balance;
                        continue;
                    }

                    let original_vesting_amount_float: f64 =
                        original_vesting_amount.to_string().parse().unwrap();
                    let total_amount_vested: f64 =
                        original_vesting_amount_float * vesting_percent_complete;
                    let total_amount_vested: Uint256 = (total_amount_vested.ceil() as u128).into();

                    assert!(original_vesting_amount >= total_amount_vested);
                    let total_amount_still_vesting = original_vesting_amount - total_amount_vested;

                    total_vested += total_amount_vested;
                    total_vesting += total_amount_still_vesting;

                    // this can happen because the delegated vesting number is only updated on undelegation / rewards withdraw
                    // while our total amount still vesting is pro-rated to find the current amount
                    let vesting_in_balance = if total_amount_still_vesting > total_delegated_vesting
                    {
                        total_amount_still_vesting - total_delegated_vesting
                    } else {
                        0u8.into()
                    };
                    // unvested tokens show up in the balance
                    // but unvested delegated tokens do not, in the case where a user
                    // has some vesting, some delegation, some balance, and some unclaimed rewards
                    assert!(user.balance >= vesting_in_balance);
                    total_liquid_supply += user.balance - vesting_in_balance;
                }
                // vesting has not started yet, in this case we subtract total vesting amount
                // from current balance, if the number is positive (staking could make it negative)
                // we add to our total
                else {
                    assert!(original_vesting_amount >= total_delegated_vesting);

                    let vesting_in_balance = original_vesting_amount - total_delegated_vesting;

                    assert!(user.balance >= vesting_in_balance);

                    let liquid = user.balance - vesting_in_balance;

                    total_liquid_balances += liquid;
                    total_vesting += original_vesting_amount;
                    total_liquid_supply += liquid;
                }
            }
            AccountType::DelayedVestingAccount(_) => todo!(),
            // it's locked, not liquid
            AccountType::PermenantLockedAccount(_) => {}
        }
    }

    info!("Finishes totals after {}s", start.elapsed().as_secs());
    Ok(ChainTotalSupplyNumbers {
        total_liquid_supply,
        total_liquid_balances,
        total_unclaimed_rewards,
        total_nonvesting_staked,
        total_vesting,
        total_vesting_staked,
        total_vested,
        total_supply,
        community_pool,
    })
}

/// Dispatching utility function for building an array of joinable futures containing sets of batch requests
async fn get_balances_for_accounts(
    input: Vec<AccountType>,
    denom: String,
) -> Result<Vec<UserInfo>, CosmosGrpcError> {
    // handed tuned parameter for the ideal number of queryes per BankQueryClient
    const BATCH_SIZE: usize = 500;
    info!(
        "Querying {} accounts in {} batches of {}",
        input.len(),
        input.len() / BATCH_SIZE,
        BATCH_SIZE
    );
    let mut index = 0;
    let mut futs = Vec::new();
    while index + BATCH_SIZE < input.len() - 1 {
        futs.push(batch_query_user_information(
            &input[index..index + BATCH_SIZE],
            denom.clone(),
        ));
        index += BATCH_SIZE;
    }
    futs.push(batch_query_user_information(&input[index..], denom.clone()));

    let executed_futures = join_all(futs).await;
    let mut balances = Vec::new();
    for b in executed_futures {
        balances.extend(b?);
    }
    Ok(balances)
}

/// Utility function for batching balance requests so that they occupy a single bankqueryclient which represents a connection
/// to the rpc server, opening connections is overhead intensive so we want to do a few thousand requests per client to really
/// make it worth our while
async fn batch_query_user_information(
    input: &[AccountType],
    denom: String,
) -> Result<Vec<UserInfo>, CosmosGrpcError> {
    trace!("Starting batch of {}", input.len());
    let mut bankrpc = BankQueryClient::connect(ALTHEA_NODE_GRPC).await?;
    let mut distrpc = DistQueryClient::connect(ALTHEA_NODE_GRPC).await?;
    let mut stakingrpc = StakingQueryClient::connect(ALTHEA_NODE_GRPC).await?;

    let mut ret = Vec::new();
    for account in input {
        let res = merge_user_information(
            account.clone(),
            denom.clone(),
            &mut bankrpc,
            &mut distrpc,
            &mut stakingrpc,
        )
        .await?;
        ret.push(res);
    }
    trace!("Finished batch of {}", input.len());
    Ok(ret)
}

/// utility function for keeping the Account and Balance info
/// in the same scope rather than zipping them on return
async fn merge_user_information(
    account: AccountType,
    denom: String,
    bankrpc: &mut BankQueryClient<Channel>,
    distrpc: &mut DistQueryClient<Channel>,
    stakingrpc: &mut StakingQueryClient<Channel>,
) -> Result<UserInfo, CosmosGrpcError> {
    // required because dec coins are multiplied by 1*10^18
    const ONE_ETH: u128 = 10u128.pow(18);

    let address = account.get_base_account().address;
    let balance_fut = bankrpc.balance(QueryBalanceRequest {
        address: address.to_string(),
        denom: denom.clone(),
    });
    let delegation_rewards_fut =
        distrpc.delegation_total_rewards(QueryDelegationTotalRewardsRequest {
            delegator_address: address.to_string(),
        });
    let total_delegated_fut = stakingrpc.delegator_delegations(QueryDelegatorDelegationsRequest {
        delegator_addr: address.to_string(),
        pagination: PAGE,
    });

    let (balance, delegation_rewards, total_delegated) =
        join3(balance_fut, delegation_rewards_fut, total_delegated_fut).await;

    let balance = balance?.into_inner();
    let delegation_rewards = delegation_rewards?.into_inner();
    let delegated = total_delegated?.into_inner();

    let balance = match balance.balance {
        Some(v) => {
            let v: Coin = v.into();
            v.amount
        }
        None => 0u8.into(),
    };

    let mut delegation_rewards_total: Uint256 = 0u8.into();
    for reward in delegation_rewards.total {
        if reward.denom == denom {
            delegation_rewards_total += reward.amount.parse().unwrap();
        }
        // you can total non-native token rewards in an else case here
    }
    delegation_rewards_total /= ONE_ETH.into();

    let mut total_delegated: Uint256 = 0u8.into();
    for delegated in delegated.delegation_responses {
        if let Some(b) = delegated.balance {
            let b: Coin = b.into();
            assert_eq!(b.denom, denom);
            total_delegated += b.amount
        }
    }

    Ok(UserInfo {
        account,
        balance,
        unclaimed_rewards: delegation_rewards_total,
        total_staked: total_delegated,
    })
}

fn sum_vesting(input: BaseVestingAccount, denom: String) -> (Uint256, Uint256, Uint256) {
    let mut total_free = 0u8.into();
    let mut total_vesting = 0u8.into();
    let mut original_amount = 0u8.into();

    for coin in input.delegated_free {
        let coin: Coin = coin.into();
        assert_eq!(coin.denom, denom);
        total_free += coin.amount;
    }
    for coin in input.delegated_vesting {
        let coin: Coin = coin.into();
        assert_eq!(coin.denom, denom);
        total_vesting += coin.amount;
    }
    for coin in input.original_vesting {
        let coin: Coin = coin.into();
        assert_eq!(coin.denom, denom);
        original_amount += coin.amount;
    }

    (total_free, total_vesting, original_amount)
}

#[derive(Debug, Clone)]
struct UserInfo {
    account: AccountType,
    balance: Uint256,
    unclaimed_rewards: Uint256,
    total_staked: Uint256,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::{max, min};

    /// Test the vesting query and ensure a sane result, if the total supply is off by more than 1% we have a problem
    #[actix_web::test]
    async fn test_vesting_query() {
        let contact = Contact::new(ALTHEA_NODE_GRPC, REQUEST_TIMEOUT, ALTHEA_PREFIX).unwrap();
        let supply = compute_liquid_supply(&contact, ALTHEA_DENOM.to_string())
            .await
            .unwrap();
        info!("Got a liquid supply of {:?}", supply);
        let total = supply.community_pool + supply.total_liquid_supply + supply.total_vesting;
        let one_hundreth_of_total = supply.total_supply / 100u8.into();
        let bigger = max(total, supply.total_supply);
        let smaller = min(total, supply.total_supply);
        let diff = bigger - smaller;
        assert!(diff < one_hundreth_of_total);
    }
}
