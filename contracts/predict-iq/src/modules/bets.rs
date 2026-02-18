use soroban_sdk::{Env, Address, String, contracttype, token};
use crate::types::{Bet, MarketStatus};
use crate::modules::markets;

#[contracttype]
pub enum DataKey {
    Bet(u64, Address), // market_id, bettor
}

pub fn place_bet(
    e: &Env,
    bettor: Address,
    market_id: u64,
    outcome: u32,
    amount: i128,
    token_address: Address,
) {
    bettor.require_auth();

    let mut market = markets::get_market(e, market_id).expect("Market not found");
    
    if market.status != MarketStatus::Active {
        panic!("Market is not active");
    }

    if e.ledger().timestamp() >= market.deadline {
        panic!("Market deadline passed");
    }

    if outcome >= market.options.len() {
        panic!("Invalid outcome index");
    }

    // Transfer tokens from bettor to contract
    let client = token::Client::new(e, &token_address);
    client.transfer(&bettor, &e.current_contract_address(), &amount);

    let bet_key = DataKey::Bet(market_id, bettor.clone());
    let mut existing_bet: Bet = e.storage().persistent().get(&bet_key).unwrap_or(Bet {
        market_id,
        bettor: bettor.clone(),
        outcome,
        amount: 0,
    });

    if existing_bet.amount > 0 && existing_bet.outcome != outcome {
        panic!("Cannot change outcome for an existing bet");
    }

    existing_bet.amount += amount;
    market.total_staked += amount;

    e.storage().persistent().set(&bet_key, &existing_bet);
    markets::update_market(e, market);

    e.events().publish(
        (String::from_str(e, "bet_placed"), market_id, bettor),
        amount,
    );
}

pub fn get_bet(e: &Env, market_id: u64, bettor: Address) -> Option<Bet> {
    e.storage().persistent().get(&DataKey::Bet(market_id, bettor))
}
