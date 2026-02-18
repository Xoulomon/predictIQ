use soroban_sdk::{Env, Address, String, Vec, contracttype};
use crate::types::{Market, MarketStatus, OracleConfig, ConfigKey};

#[contracttype]
pub enum DataKey {
    Market(u64),
    MarketCount,
}

pub fn create_market(
    e: &Env,
    creator: Address,
    description: String,
    options: Vec<String>,
    deadline: u64,
    resolution_deadline: u64,
    oracle_config: OracleConfig,
) -> u64 {
    creator.require_auth();

    let mut count: u64 = e.storage().instance().get(&DataKey::MarketCount).unwrap_or(0);
    count += 1;

    let market = Market {
        id: count,
        creator,
        description,
        options,
        status: MarketStatus::Active,
        deadline,
        resolution_deadline,
        winning_outcome: None,
        oracle_config,
        total_staked: 0,
    };

    e.storage().persistent().set(&DataKey::Market(count), &market);
    e.storage().instance().set(&DataKey::MarketCount, &count);

    e.events().publish(
        (String::from_str(e, "market_created"), count),
        market.id,
    );

    count
}

pub fn get_market(e: &Env, id: u64) -> Option<Market> {
    e.storage().persistent().get(&DataKey::Market(id))
}

pub fn update_market(e: &Env, market: Market) {
    e.storage().persistent().set(&DataKey::Market(market.id), &market);
}
