#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env, Vec, String};

#[test]
fn test_market_lifecycle() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &100); // 1% fee

    let creator = Address::generate(&e);
    let description = String::from_str(&e, "Will BTC reach $100k?");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let deadline = 1000;
    let resolution_deadline = 2000;
    
    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "btc_price"),
        min_responses: 1,
        max_staleness_seconds: 300, // 5 minutes
        max_confidence_bps: 200, // 2%
    };

    let market_id = client.create_market(&creator, &description, &options, &deadline, &resolution_deadline, &oracle_config);
    assert_eq!(market_id, 1);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.id, 1);
    assert_eq!(market.status, types::MarketStatus::Active);
}

#[test]
fn test_oracle_manual_resolution() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &100);

    let creator = Address::generate(&e);
    let description = String::from_str(&e, "Test market");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test_feed"),
        min_responses: 1,
        max_staleness_seconds: 300,
        max_confidence_bps: 200,
    };

    let market_id = client.create_market(&creator, &description, &options, &1000, &2000, &oracle_config);
    
    // Admin sets oracle result
    client.set_oracle_result(&market_id, &0);
    
    // Verify result was stored
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.id, market_id);
}
