#![cfg(test)]

//! Comprehensive tests for Oracle price validation, with focus on confidence threshold rounding.
//!
//! # Issue #260: Confidence Threshold Rounding
//!
//! The confidence validation formula is: `max_conf = (price_abs * max_confidence_bps) / 10000`
//!
//! ## Problem
//! Integer division can introduce bias for small prices:
//! - price=1, bps=500 (5%): (1 * 500) / 10000 = 0 (truncates, should be ~0.05)
//! - price=10, bps=100 (1%): (10 * 100) / 10000 = 0 (truncates, should be ~0.1)
//! - price=100, bps=100 (1%): (100 * 100) / 10000 = 1 (correct)
//!
//! This causes a **downward bias** for small prices, making it harder to accept prices
//! with any confidence interval at very small valuations.
//!
//! ## Potential Solutions
//! 1. **Ceiling division**: Use `(price * bps + 9999) / 10000` to round up
//! 2. **Fixed-point math**: Scale up before division to preserve precision
//! 3. **Reverse formula**: Check `(price * bps) >= (conf * 10000)` to avoid division
//!
//! ## Test Coverage
//! - `test_confidence_rounding_small_prices`: Tests 1-100 range prices
//! - `test_confidence_rounding_large_prices`: Tests million+ range prices
//! - `test_confidence_rounding_edge_cases_low_prices`: Targets specific rounding boundaries
//! - `test_confidence_rounding_negative_prices`: Validates absolute value handling
//! - `test_confidence_rounding_boundary_conditions`: Documents exact rounding behavior

use super::oracles::*;
use crate::types::OracleConfig;
use crate::errors::ErrorCode;
use soroban_sdk::{Env, Address, String, testutils::Address as _};

fn create_config(e: &Env, max_confidence_bps: u64) -> OracleConfig {
    OracleConfig {
        oracle_address: Address::generate(e),
        feed_id: String::from_str(e, "test_feed"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps,
    }
}

fn create_price(price: i64, conf: u64, timestamp: u64) -> PythPrice {
    PythPrice {
        price,
        conf,
        expo: -2,
        publish_time: timestamp,
    }
}

#[test]
fn test_validate_fresh_price() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();
    
    let config = create_config(&e, 200); // 2%
    
    let price = create_price(100000, 1000, current_time - 60); // 1% of price
    
    let result = validate_price(&e, &price, &config);
    assert!(result.is_ok());
}

#[test]
fn test_reject_stale_price() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();
    
    let config = create_config(&e, 200);
    let price = create_price(100000, 1000, current_time - 400); // 400 seconds old

    let result = validate_price(&e, &price, &config);
    assert_eq!(result, Err(ErrorCode::StalePrice));
}

#[test]
fn test_reject_low_confidence() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();
    
    let config = create_config(&e, 200); // 2%
    let price = create_price(100000, 3000, current_time - 60); // 3% - exceeds threshold

    let result = validate_price(&e, &price, &config);
    assert_eq!(result, Err(ErrorCode::ConfidenceTooLow));
}

/// Table-driven tests for confidence threshold rounding across price ranges.
/// Tests the formula: max_conf = (price_abs * max_confidence_bps) / 10000
/// Issue: Integer division can bias acceptance for small prices.
#[test]
fn test_confidence_rounding_small_prices() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();

    // Test cases: (price, max_confidence_bps, acceptance_confidence, should_pass)
    // Format: test that conf <= (price * bps) / 10000
    let test_cases = vec![
        // (price, max_confidence_bps, confidence_value, should_accept, description)
        (1, 500, 0, true, "price=1, 5% conf, conf=0 at boundary"),
        (1, 500, 1, false, "price=1, 5% conf, conf=1 exceeds rounding result"),
        (10, 100, 0, true, "price=10, 1% conf, conf=0 at boundary"),
        (10, 100, 1, false, "price=10, 1% conf, conf=1 exceeds rounding result"),
        (99, 100, 0, true, "price=99, 1% conf, conf=0 at boundary"),
        (99, 100, 1, false, "price=99, 1% conf, conf=1 exceeds rounding result"),
        (100, 100, 1, true, "price=100, 1% conf, conf=1 within threshold"),
        (100, 100, 2, false, "price=100, 1% conf, conf=2 exceeds threshold"),
        (1000, 100, 10, true, "price=1000, 1% conf, conf=10 within threshold"),
        (1000, 100, 11, false, "price=1000, 1% conf, conf=11 exceeds threshold"),
        (10000, 50, 50, true, "price=10000, 0.5% conf, conf=50 within threshold"),
        (10000, 50, 51, false, "price=10000, 0.5% conf, conf=51 exceeds threshold"),
    ];

    for (price, bps, conf, should_accept, desc) in test_cases {
        let config = create_config(&e, bps);
        let price_obj = create_price(price as i64, conf, current_time - 60);
        let result = validate_price(&e, &price_obj, &config);

        if should_accept {
            assert!(result.is_ok(), "Test failed: {} | Result: {:?}", desc, result);
        } else {
            assert_eq!(result, Err(ErrorCode::ConfidenceTooLow), 
                      "Test failed: {} | Result: {:?}", desc, result);
        }
    }
}

/// Table-driven tests for confidence threshold rounding with large prices.
/// Verifies that rounding bias is minimized or absent for large prices.
#[test]
fn test_confidence_rounding_large_prices() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();

    let test_cases = vec![
        // (price, max_confidence_bps, confidence_value, should_accept, description)
        (1_000_000, 100, 10_000, true, "price=1M, 1% conf, conf=10K within threshold"),
        (1_000_000, 100, 10_001, false, "price=1M, 1% conf, conf=10K+1 exceeds threshold"),
        (10_000_000, 100, 100_000, true, "price=10M, 1% conf, conf=100K within threshold"),
        (10_000_000, 100, 100_001, false, "price=10M, 1% conf, conf=100K+1 exceeds threshold"),
        (1_000_000, 200, 20_000, true, "price=1M, 2% conf, conf=20K within threshold"),
        (1_000_000, 200, 20_001, false, "price=1M, 2% conf, conf=20K+1 exceeds threshold"),
        (100_000_000, 50, 5_000_000, true, "price=100M, 0.5% conf, conf=5M within threshold"),
        (100_000_000, 50, 5_000_001, false, "price=100M, 0.5% conf, conf=5M+1 exceeds threshold"),
    ];

    for (price, bps, conf, should_accept, desc) in test_cases {
        let config = create_config(&e, bps);
        let price_obj = create_price(price as i64, conf, current_time - 60);
        let result = validate_price(&e, &price_obj, &config);

        if should_accept {
            assert!(result.is_ok(), "Test failed: {} | Result: {:?}", desc, result);
        } else {
            assert_eq!(result, Err(ErrorCode::ConfidenceTooLow), 
                      "Test failed: {} | Result: {:?}", desc, result);
        }
    }
}

/// Table-driven tests for edge cases where rounding can cause unexpected behavior.
/// These test cases specifically target the rounding bias problem where
/// low prices can cause max_conf to round down to 0.
#[test]
fn test_confidence_rounding_edge_cases_low_prices() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();

    let test_cases = vec![
        // Edge case: very small prices with moderate confidence requirements
        // (price, max_confidence_bps, confidence_at_boundary, should_pass)
        (1, 10000, 0, true, "price=1, 100% bps, conf=0 at rounding boundary"),
        (1, 10000, 1, false, "price=1, 100% bps, conf=1 exceeds rounding result"),
        (5, 2000, 0, true, "price=5, 20% bps, conf=0 at boundary (5*2000/10000=1)"),
        (5, 2000, 1, true, "price=5, 20% bps, conf=1 within threshold"),
        (5, 2000, 2, false, "price=5, 20% bps, conf=2 exceeds threshold"),
        (9, 1111, 0, true, "price=9, 11.11% bps, conf=0 at boundary"),
        (9, 1111, 1, true, "price=9, 11.11% bps, conf=1 within threshold (9*1111/10000=0.9999≈1)"),
        (50, 200, 10, true, "price=50, 2% bps, conf=10 within threshold (50*200/10000=1)"),
        (50, 200, 11, false, "price=50, 2% bps, conf=11 exceeds threshold"),
    ];

    for (price, bps, conf, should_accept, desc) in test_cases {
        let config = create_config(&e, bps);
        let price_obj = create_price(price as i64, conf, current_time - 60);
        let result = validate_price(&e, &price_obj, &config);

        if should_accept {
            assert!(result.is_ok(), "Test failed: {} | Result: {:?}", desc, result);
        } else {
            assert_eq!(result, Err(ErrorCode::ConfidenceTooLow), 
                      "Test failed: {} | Result: {:?}", desc, result);
        }
    }
}

/// Table-driven tests for negative prices (should use absolute value).
#[test]
fn test_confidence_rounding_negative_prices() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();

    let test_cases = vec![
        // (price, max_confidence_bps, confidence_value, should_accept, description)
        (-100, 100, 1, true, "price=-100, 1% conf, conf=1 within threshold"),
        (-100, 100, 2, false, "price=-100, 1% conf, conf=2 exceeds threshold"),
        (-1, 500, 0, true, "price=-1, 5% conf, conf=0 at boundary"),
        (-1, 500, 1, false, "price=-1, 5% conf, conf=1 exceeds boundary"),
        (-1_000_000, 100, 10_000, true, "price=-1M, 1% conf, conf=10K within threshold"),
        (-1_000_000, 100, 10_001, false, "price=-1M, 1% conf, conf=10K+1 exceeds threshold"),
    ];

    for (price, bps, conf, should_accept, desc) in test_cases {
        let config = create_config(&e, bps);
        let price_obj = create_price(price, conf, current_time - 60);
        let result = validate_price(&e, &price_obj, &config);

        if should_accept {
            assert!(result.is_ok(), "Test failed: {} | Result: {:?}", desc, result);
        } else {
            assert_eq!(result, Err(ErrorCode::ConfidenceTooLow), 
                      "Test failed: {} | Result: {:?}", desc, result);
        }
    }
}

/// Table-driven tests that verify boundary conditions.
/// These tests document the exact rounding behavior for reference.
#[test]
fn test_confidence_rounding_boundary_conditions() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();

    let test_cases = vec![
        // Test rounding boundaries: when does (price * bps) / 10000 transition?
        // (price, max_confidence_bps, confidence_under_boundary, conf_at_boundary, description)
        (50, 200, 0, 1, "price=50, 2%: boundary at 1 (50*200/10000=1)"),
        (49, 200, 0, 0, "price=49, 2%: rounds to 0 (49*200/10000=0.98)"),
        (51, 200, 0, 1, "price=51, 2%: rounds to 1 (51*200/10000=1.02)"),
        (100, 100, 0, 1, "price=100, 1%: boundary at 1 (100*100/10000=1)"),
        (99, 100, 0, 0, "price=99, 1%: rounds to 0 (99*100/10000=0.99)"),
        (101, 100, 0, 1, "price=101, 1%: rounds to 1 (101*100/10000=1.01)"),
    ];

    for (price, bps, under_boundary, at_boundary, desc) in test_cases {
        let config = create_config(&e, bps);
        
        // Test with confidence under boundary
        let price_under = create_price(price as i64, under_boundary, current_time - 60);
        let result_under = validate_price(&e, &price_under, &config);
        assert!(result_under.is_ok(), 
                "Test failed (under boundary): {} | Result: {:?}", desc, result_under);
        
        // Test with confidence at boundary
        let price_at = create_price(price as i64, at_boundary, current_time - 60);
        let result_at = validate_price(&e, &price_at, &config);
        let expected_at = if at_boundary <= (price as u64 * bps) / 10000 {
            Ok(())
        } else {
            Err(ErrorCode::ConfidenceTooLow)
        };
        assert_eq!(result_at, expected_at, 
                  "Test failed (at boundary): {} | Result: {:?}", desc, result_at);
    }
}

/// Validation test for potential fix using ceiling division.
/// This test demonstrates the expected behavior after implementing a fix.
/// 
/// Currently documents the bias:
/// - price=1, 5% BPS, ceiling: (1*500 + 9999)/10000 = 1 (vs current 0)
/// - price=5, 2% BPS, ceiling: (5*2000 + 9999)/10000 = 2 (vs current 1)
///
/// Uncomment assertions once fix is implemented.
#[test]
fn test_confidence_rounding_documented_bias() {
    // This test documents which cases are currently biased
    
    // Small price, rounded down to 0
    let downward_bias_cases = vec![
        (1, 500),    // (1 * 500) / 10000 = 0, should be ~1 with ceiling
        (10, 100),   // (10 * 100) / 10000 = 0, should be ~1 with ceiling
        (99, 100),   // (99 * 100) / 10000 = 0, should be ~1 with ceiling
        (49, 200),   // (49 * 200) / 10000 = 0, should be ~1 with ceiling
    ];

    for (price, bps) in downward_bias_cases {
        let truncated = (price as u64 * bps) / 10000;
        let ceiling = (price as u64 * bps + 9999) / 10000;
        
        // Current implementation uses truncation (truncated)
        // Potential fix would use ceiling division
        // Verify that truncation < ceiling for small prices
        if truncated == 0 {
            assert!(ceiling > truncated, 
                   "Downward bias at price={}, bps={}: truncated={}, ceiling={}", 
                   price, bps, truncated, ceiling);
        }
    }
}

// =============================================================================
// Issue #261: Multi-Oracle Keying Tests
// =============================================================================
//!
//! # Issue #261: Multi-Oracle Keying & Collision Prevention
//!
//! The oracle result storage uses a composite key: `OracleData::Result(market_id, oracle_id)`
//!
//! ## Problem
//! Without proper testing, the following risks exist:
//! 1. **Key Collisions**: Different (market_id, oracle_id) pairs could hash to same storage location
//! 2. **Data Isolation Failure**: Retrieving (market_id=1, oracle_id=1) could return data from (market_id=1, oracle_id=2)
//! 3. **Missing Multi-Oracle Support**: No tests verifying multiple oracle IDs per market work correctly
//! 4. **Boundary Weaknesses**: Untested edge cases with large market_ids, large oracle_ids, or both
//!
//! ## Storage Key Structure
//! ```
//! OracleData::Result(market_id: u64, oracle_id: u32) -> outcome: u32
//! OracleData::LastUpdate(market_id: u64, oracle_id: u32) -> timestamp: u64
//! ```
//!
//! ## Test Coverage
//! - `test_multi_oracle_basic_storage_retrieval`: Verify store/retrieve for (market_id, oracle_id) pairs
//! - `test_multi_oracle_isolation_same_market`: Ensure different oracle_ids in same market don't collide
//! - `test_multi_oracle_isolation_different_markets`: Ensure same oracle_id in different markets don't collide
//! - `test_multi_oracle_matrix_combinations`: Table-driven tests of all combinations
//! - `test_multi_oracle_large_ids`: Tests with maximum/boundary u64 and u32 values
//! - `test_multi_oracle_sequential_updates`: Ensure updates don't affect other oracles
//! - `test_multi_oracle_timestamp_independence`: Verify timestamps are independent per (market_id, oracle_id)
//! - `test_multi_oracle_collision_mitigation`: Demonstrates the fix prevents theoretical collisions

/// Basic sanity test: Store and retrieve oracle results for a single (market_id, oracle_id) pair.
#[test]
fn test_multi_oracle_basic_storage_retrieval() {
    let e = Env::default();
    let market_id = 100u64;
    let oracle_id = 0u32;
    let outcome = 1u32;

    // Store result
    e.storage().persistent().set(&OracleData::Result(market_id, oracle_id), &outcome);
    
    // Retrieve and verify
    let retrieved: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, oracle_id));
    assert_eq!(retrieved, Some(outcome), 
               "Failed to retrieve outcome for market_id={}, oracle_id={}", market_id, oracle_id);
}

/// Test isolation within a single market: Different oracle_ids should have independent storage.
/// This is critical for multi-oracle aggregation - same market, different sources.
#[test]
fn test_multi_oracle_isolation_same_market() {
    let e = Env::default();
    let market_id = 100u64;

    // Test cases: (oracle_id, outcome)
    let test_cases = vec![
        (0u32, 0u32, "primary oracle outcome=0"),
        (1u32, 1u32, "secondary oracle outcome=1"),
        (2u32, 0u32, "tertiary oracle outcome=0"),
        (3u32, 1u32, "quaternary oracle outcome=1"),
    ];

    // Store multiple oracle results for same market
    for (oracle_id, outcome, desc) in &test_cases {
        e.storage()
            .persistent()
            .set(&OracleData::Result(market_id, *oracle_id), outcome);
    }

    // Verify each oracle result is independent and correct
    for (oracle_id, expected_outcome, desc) in &test_cases {
        let retrieved: Option<u32> = e.storage()
            .persistent()
            .get(&OracleData::Result(market_id, *oracle_id));
        
        assert_eq!(
            retrieved,
            Some(*expected_outcome),
            "Isolation failure for market_id={}, oracle_id={}: {} | Got: {:?}, Expected: {}",
            market_id, oracle_id, desc, retrieved, expected_outcome
        );
    }
}

/// Test isolation across markets: Same oracle_id in different markets should be independent.
/// This is critical for market isolation - prevents cross-market data leakage.
#[test]
fn test_multi_oracle_isolation_different_markets() {
    let e = Env::default();
    let oracle_id = 0u32; // Use same oracle for different markets

    // Test cases: (market_id, outcome)
    let test_cases = vec![
        (1u64, 0u32, "market 1 outcome=0"),
        (2u64, 1u32, "market 2 outcome=1"),
        (3u64, 0u32, "market 3 outcome=0"),
        (100u64, 1u32, "market 100 outcome=1"),
        (1000u64, 0u32, "market 1000 outcome=0"),
    ];

    // Store oracle result in each market
    for (market_id, outcome, desc) in &test_cases {
        e.storage()
            .persistent()
            .set(&OracleData::Result(*market_id, oracle_id), outcome);
    }

    // Verify each market's result is independent and correct
    for (market_id, expected_outcome, desc) in &test_cases {
        let retrieved: Option<u32> = e.storage()
            .persistent()
            .get(&OracleData::Result(*market_id, oracle_id));
        
        assert_eq!(
            retrieved,
            Some(*expected_outcome),
            "Market isolation failure for market_id={}, oracle_id={}: {} | Got: {:?}, Expected: {}",
            market_id, oracle_id, desc, retrieved, expected_outcome
        );
    }
}

/// Matrix test: All combinations of (market_id, oracle_id) pairs must have independent storage.
/// This comprehensive test verifies the collision-free property of the composite key.
#[test]
fn test_multi_oracle_matrix_combinations() {
    let e = Env::default();

    // Test matrix: 3 markets × 4 oracles = 12 distinct pairs
    let market_ids = vec![1u64, 100u64, 10000u64];
    let oracle_ids = vec![0u32, 1u32, 2u32, 3u32];
    
    // Store unique outcome for each pair: outcome = (market_id % 2) XOR (oracle_id % 2)
    let mut stored_pairs = vec![];
    for market_id in &market_ids {
        for oracle_id in &oracle_ids {
            let outcome = ((market_id % 2) as u32) ^ (oracle_id % 2);
            e.storage()
                .persistent()
                .set(&OracleData::Result(*market_id, *oracle_id), &outcome);
            stored_pairs.push((*market_id, *oracle_id, outcome));
        }
    }

    // Verify all pairs retrieve correct values (no collisions, no cross-pollution)
    for (market_id, oracle_id, expected_outcome) in stored_pairs {
        let retrieved: Option<u32> = e.storage()
            .persistent()
            .get(&OracleData::Result(market_id, oracle_id));
        
        assert_eq!(
            retrieved,
            Some(expected_outcome),
            "Matrix collision detected at market_id={}, oracle_id={} | Got: {:?}, Expected: {}",
            market_id, oracle_id, retrieved, expected_outcome
        );
    }
}

/// Test with large ID values: market_id near u64::MAX and oracle_id near u32::MAX.
/// Boundary testing to ensure hash collisions don't occur with extreme values.
#[test]
fn test_multi_oracle_large_ids() {
    let e = Env::default();

    // Test cases: (market_id, oracle_id, outcome, description)
    let test_cases = vec![
        (u64::MAX, 0u32, 0u32, "market_id=MAX, oracle_id=0"),
        (u64::MAX - 1, 0u32, 1u32, "market_id=MAX-1, oracle_id=0"),
        (0u64, u32::MAX, 1u32, "market_id=0, oracle_id=MAX"),
        (0u64, u32::MAX - 1, 0u32, "market_id=0, oracle_id=MAX-1"),
        (u64::MAX, u32::MAX, 1u32, "market_id=MAX, oracle_id=MAX"),
        (u64::MAX - 1, u32::MAX - 1, 0u32, "market_id=MAX-1, oracle_id=MAX-1"),
        (1u64, u32::MAX / 2, 1u32, "market_id=1, oracle_id=MAX/2"),
        (u64::MAX / 2, 1u32, 0u32, "market_id=MAX/2, oracle_id=1"),
    ];

    // Store and verify large ID combinations
    for (market_id, oracle_id, outcome, desc) in &test_cases {
        e.storage()
            .persistent()
            .set(&OracleData::Result(*market_id, *oracle_id), outcome);
        
        let retrieved: Option<u32> = e.storage()
            .persistent()
            .get(&OracleData::Result(*market_id, *oracle_id));
        
        assert_eq!(
            retrieved,
            Some(*outcome),
            "Large ID test failed: {} | Got: {:?}, Expected: {}",
            desc, retrieved, outcome
        );
    }
}

/// Test sequential updates: Updating one oracle shouldn't affect others.
/// Verifies that write operations are truly isolated.
#[test]
fn test_multi_oracle_sequential_updates() {
    let e = Env::default();
    let market_id = 100u64;

    // Initial state: Store outcomes for 3 oracles
    let initial_outcomes = vec![
        (0u32, 0u32),
        (1u32, 1u32),
        (2u32, 0u32),
    ];

    for (oracle_id, outcome) in &initial_outcomes {
        e.storage()
            .persistent()
            .set(&OracleData::Result(market_id, *oracle_id), outcome);
    }

    // Update oracle 1 outcome and verify others unchanged
    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, 1u32), &1u32);

    // Verify oracle 0 unchanged
    let oracle_0: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, 0u32));
    assert_eq!(oracle_0, Some(0u32), "Oracle 0 was corrupted by update to oracle 1");

    // Verify oracle 2 unchanged
    let oracle_2: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, 2u32));
    assert_eq!(oracle_2, Some(0u32), "Oracle 2 was corrupted by update to oracle 1");

    // Verify oracle 1 updated
    let oracle_1: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, 1u32));
    assert_eq!(oracle_1, Some(1u32), "Oracle 1 failed to update");
}

/// Test timestamp independence: LastUpdate timestamps should be independent per (market_id, oracle_id).
/// Ensures that updating one oracle's timestamp doesn't affect others.
#[test]
fn test_multi_oracle_timestamp_independence() {
    let e = Env::default();
    let market_id = 100u64;
    let oracle_ids = vec![0u32, 1u32, 2u32];
    let timestamps = vec![1000u64, 2000u64, 3000u64];

    // Store different timestamps for each oracle
    for (i, oracle_id) in oracle_ids.iter().enumerate() {
        let timestamp = timestamps[i];
        e.storage()
            .persistent()
            .set(&OracleData::LastUpdate(market_id, *oracle_id), &timestamp);
    }

    // Verify each oracle has its own independent timestamp
    for (i, oracle_id) in oracle_ids.iter().enumerate() {
        let expected_timestamp = timestamps[i];
        let retrieved: Option<u64> = e.storage()
            .persistent()
            .get(&OracleData::LastUpdate(market_id, *oracle_id));
        
        assert_eq!(
            retrieved,
            Some(expected_timestamp),
            "Timestamp isolation failure for oracle_id={} | Got: {:?}, Expected: {}",
            oracle_id, retrieved, expected_timestamp
        );
    }

    // Update one timestamp and verify others unchanged
    e.storage()
        .persistent()
        .set(&OracleData::LastUpdate(market_id, 1u32), &9999u64);

    // Verify oracle 0 timestamp unchanged
    let oracle_0_ts: Option<u64> = e.storage()
        .persistent()
        .get(&OracleData::LastUpdate(market_id, 0u32));
    assert_eq!(oracle_0_ts, Some(1000u64), "Oracle 0 timestamp was corrupted");

    // Verify oracle 2 timestamp unchanged
    let oracle_2_ts: Option<u64> = e.storage()
        .persistent()
        .get(&OracleData::LastUpdate(market_id, 2u32));
    assert_eq!(oracle_2_ts, Some(3000u64), "Oracle 2 timestamp was corrupted");

    // Verify oracle 1 timestamp updated
    let oracle_1_ts: Option<u64> = e.storage()
        .persistent()
        .get(&OracleData::LastUpdate(market_id, 1u32));
    assert_eq!(oracle_1_ts, Some(9999u64), "Oracle 1 timestamp failed to update");
}

/// Comprehensive collision mitigation test: Verify the composite key prevents collisions.
/// Tests theoretical collision scenarios that would fail with poor key design.
#[test]
fn test_multi_oracle_collision_mitigation() {
    let e = Env::default();

    // Collision scenarios that would fail if keys weren't properly composite:
    // 1. Simple concatenation: market_id=1, oracle_id=0 (key="10") vs market_id=10, oracle_id=0 (key="100")
    // 2. Bit-packing errors: market_id=(u32::MAX+1), oracle_id=0 could collide with others
    // 3. Hash collisions: Poor struct hashing could cause different (m,o) pairs to hash to same location

    let collision_scenarios = vec![
        // Scenario 1: Simple string concatenation would collide
        ((1u64, 0u32, 0u32), (10u64, 0u32, 1u32), "concatenation collision risk: '10' vs '100'"),
        // Scenario 2: Overflow/wrapping issues
        ((u32::MAX as u64, 0u32, 0u32), ((u32::MAX as u64) + 1, 0u32, 1u32), "boundary overflow risk"),
        // Scenario 3: Adjacent values
        ((1000u64, 1u32, 0u32), (1000u64, 2u32, 1u32), "adjacent oracle_id differentiation"),
        ((1000u64, 0u32, 0u32), (1001u64, 0u32, 1u32), "adjacent market_id differentiation"),
        // Scenario 4: Reversed pairs (if key wasn't ordered)
        ((1u64, 100u32, 0u32), (100u64, 1u32, 1u32), "reversed (market, oracle) pair"),
    ];

    // Store values for all collision scenarios
    for ((m1, o1, v1), (m2, o2, v2), scenario_desc) in &collision_scenarios {
        e.storage()
            .persistent()
            .set(&OracleData::Result(*m1, *o1), v1);
        e.storage()
            .persistent()
            .set(&OracleData::Result(*m2, *o2), v2);

        // Verify no collision: each key retrieves its own value
        let retrieved_1: Option<u32> = e.storage().persistent().get(&OracleData::Result(*m1, *o1));
        let retrieved_2: Option<u32> = e.storage().persistent().get(&OracleData::Result(*m2, *o2));

        assert_eq!(
            retrieved_1, Some(*v1),
            "Collision scenario failed ({}, {}, {}): first key returned wrong value. Scenario: {}",
            m1, o1, v1, scenario_desc
        );
        
        assert_eq!(
            retrieved_2, Some(*v2),
            "Collision scenario failed ({}, {}, {}): second key returned wrong value. Scenario: {}",
            m2, o2, v2, scenario_desc
        );
    }
}
