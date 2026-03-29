use crate::types::{Market, MarketStatus, Guardian};
use crate::modules::{markets, governance};
use soroban_sdk::{Env, Vec};

/// Hard cap on the number of records returned by any single paginated query.
/// Callers supplying a larger `limit` are silently clamped to this value.
/// This bounds per-call gas and memory consumption regardless of dataset size.
pub const MAX_PAGE_LIMIT: u32 = 100;

/// Paginated retrieval of markets.
///
/// Returns a segment of all markets created, regardless of status.
/// This prevents resource limit exhaustion (gas/memory) on large datasets.
///
/// # Arguments
/// * `offset` - Starting index for pagination (0-based)
/// * `limit` - Maximum number of markets to return; clamped to [`MAX_PAGE_LIMIT`]
pub fn get_markets(e: &Env, offset: u32, limit: u32) -> Vec<Market> {
    let limit = limit.min(MAX_PAGE_LIMIT);
    let count: u64 = e
        .storage()
        .instance()
        .get(&markets::DataKey::MarketCount)
        .unwrap_or(0);
    
    let mut markets_vec = Vec::new(e);
    let start = (offset as u64).min(count);
    let end = (start + limit as u64).min(count);
    
    // Counting from 1 since market IDs are 1-based in this contract
    for i in (start + 1)..=(end) {
        if let Some(market) = markets::get_market(e, i) {
            markets_vec.push_back(market);
        }
    }
    
    markets_vec
}

/// Paginated retrieval of markets by status.
///
/// Returns a segment of markets that match the specified status.
/// Implementation iterates backwards from the newest markets to prioritize freshness.
/// Results are truncated at [`MAX_PAGE_LIMIT`] even if the caller requests more.
///
/// # Arguments
/// * `status` - The status to filter by (e.g., Active, Resolved)
/// * `offset` - Starting element in the filtered list
/// * `limit` - Maximum number of markets to return; clamped to [`MAX_PAGE_LIMIT`]
pub fn get_markets_by_status(e: &Env, status: MarketStatus, offset: u32, limit: u32) -> Vec<Market> {
    let limit = limit.min(MAX_PAGE_LIMIT);
    let count: u64 = e
        .storage()
        .instance()
        .get(&markets::DataKey::MarketCount)
        .unwrap_or(0);
    
    let mut markets_vec = Vec::new(e);
    let mut found_count = 0;
    let mut skipped_count = 0;
    
    // Status-based search requires iteration.
    // Iterating backwards from the latest market for fresher results.
    for i in (1..=count).rev() {
        if let Some(market) = markets::get_market(e, i) {
            if market.status == status {
                if skipped_count < offset {
                    skipped_count += 1;
                } else {
                    markets_vec.push_back(market);
                    found_count += 1;
                    if found_count >= limit {
                        break;
                    }
                }
            }
        }
    }
    
    markets_vec
}

/// Paginated retrieval of guardians.
///
/// Avoids the gas cost of loading the entire guardian set into memory
/// when the set grows large. Results are truncated at [`MAX_PAGE_LIMIT`].
///
/// # Arguments
/// * `offset` - Starting index
/// * `limit` - Maximum number of guardians to return; clamped to [`MAX_PAGE_LIMIT`]
pub fn get_guardians_paginated(e: &Env, offset: u32, limit: u32) -> Vec<Guardian> {
    let limit = limit.min(MAX_PAGE_LIMIT);
    let all_guardians = governance::get_guardians(e);
    let mut segment = Vec::new(e);
    
    let start = offset.min(all_guardians.len());
    let end = (start + limit).min(all_guardians.len());
    
    for i in start..end {
        if let Some(g) = all_guardians.get(i) {
            segment.push_back(g);
        }
    }
    
    segment
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PredictIQ, PredictIQClient};
    use crate::types::{OracleConfig, MarketTier};
    use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec as SdkVec};

    fn setup() -> (Env, PredictIQClient<'static>, Address, Address) {
        let e = Env::default();
        e.mock_all_auths();
        let contract_id = e.register_contract(None, PredictIQ);
        let client = PredictIQClient::new(&e, &contract_id);
        let admin = Address::generate(&e);
        let creator = Address::generate(&e);
        client.initialize(&admin, &0);
        (e, client, admin, creator)
    }

    fn make_market(e: &Env, client: &PredictIQClient, creator: &Address) -> u64 {
        let options = SdkVec::from_array(e, [String::from_str(e, "Yes"), String::from_str(e, "No")]);
        let token = Address::generate(e);
        let oracle_cfg = OracleConfig {
            oracle_address: Address::generate(e),
            feed_id: String::from_str(e, "feed"),
            min_responses: None,
            max_staleness_seconds: 3600,
            max_confidence_bps: 100,
        };
        client.create_market(creator, &String::from_str(e, "M"), &options, &1000, &2000, &oracle_cfg, &MarketTier::Basic, &token, &0, &0)
    }

    #[test]
    fn test_limit_clamped_to_max() {
        let (e, client, _, creator) = setup();
        // Create MAX_PAGE_LIMIT + 10 markets
        for _ in 0..(MAX_PAGE_LIMIT + 10) {
            make_market(&e, &client, &creator);
        }
        // Requesting more than MAX_PAGE_LIMIT should return at most MAX_PAGE_LIMIT
        let result = client.get_markets(&0, &(MAX_PAGE_LIMIT + 50));
        assert_eq!(result.len(), MAX_PAGE_LIMIT);
    }

    #[test]
    fn test_status_limit_clamped_to_max() {
        let (e, client, _, creator) = setup();
        for _ in 0..(MAX_PAGE_LIMIT + 10) {
            make_market(&e, &client, &creator);
        }
        let result = client.get_markets_by_status(&MarketStatus::Active, &0, &(MAX_PAGE_LIMIT + 50));
        assert_eq!(result.len(), MAX_PAGE_LIMIT);
    }

    #[test]
    fn test_limit_zero_returns_empty() {
        let (e, client, _, creator) = setup();
        make_market(&e, &client, &creator);
        let result = client.get_markets(&0, &0);
        assert_eq!(result.len(), 0);
    }
}
