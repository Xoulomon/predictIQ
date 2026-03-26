# Issue #261: Multi-Oracle Keying & Collision Prevention

## Overview

**Issue**: The oracle result storage previously used a hardcoded oracle_id (0), preventing multi-oracle aggregation and creating risks for key collisions.

**Status**: ✅ Implementation Complete (Issue #9 mitigation), Testing Gap Addressed (Issue #261)

**Area**: Oracle validation and storage isolation

## Problem Statement

### Original Issue (Issue #9)
The contract stored oracle results with a hardcoded sub-index:
```rust
// ❌ BEFORE: Hardcoded oracle_id
OracleData::Result(market_id, 0)  // All markets forced to use oracle_id=0
```

**Impact**:
- Only one oracle result per market forever
- No multi-oracle aggregation support
- No historical retrieval if a market is re-resolved

### Current Implementation (Issue #261 Tests)
The code now supports composite keys:
```rust
// ✅ AFTER: Composite (market_id, oracle_id) key
OracleData::Result(market_id, oracle_id)      // Per-oracle results
OracleData::LastUpdate(market_id, oracle_id)  // Per-oracle timestamps
```

**Remaining Risk**: Without comprehensive tests, potential issues could be:
1. **Key Collisions**: Different pairs hash to same storage location
2. **Data Isolation Failure**: Cross-contamination between oracle results
3. **Boundary Weaknesses**: Untested edge cases with large IDs

## Storage Key Structure

```rust
#[contracttype]
pub enum OracleData {
    Result(u64, u32),     // (market_id, oracle_id) -> outcome: u32
    LastUpdate(u64, u32), // (market_id, oracle_id) -> timestamp: u64
}
```

### Key Characteristics
- **market_id**: u64 - max `18,446,744,073,709,551,615` markets
- **oracle_id**: u32 - max `4,294,967,295` oracles per market
- **Composite Key**: Each combination must be unique in persistent storage

## Testing Strategy

### Test Coverage

| Test Function | Purpose | Scenarios | Guards Against |
|---|---|---|---|
| `test_multi_oracle_basic_storage_retrieval` | Sanity check | Store/retrieve one pair | Basic functionality |
| `test_multi_oracle_isolation_same_market` | Isolation within market | 4 different oracle_ids | Cross-oracle pollution |
| `test_multi_oracle_isolation_different_markets` | Isolation across markets | 5 different market_ids | Cross-market pollution |
| `test_multi_oracle_matrix_combinations` | Comprehensive matrix | 3 markets × 4 oracles | Statistical collisions |
| `test_multi_oracle_large_ids` | Boundary testing | MAX/MIN values | Overflow/wrapping issues |
| `test_multi_oracle_sequential_updates` | Write isolation | Sequential updates | Update side-effects |
| `test_multi_oracle_timestamp_independence` | Timestamp isolation | Independent timestamps | Timestamp cross-pollution |
| `test_multi_oracle_collision_mitigation` | Collision scenarios | 5 theoretical risks | Hash collision vulnerabilities |

### Total Test Coverage
- **8 test functions**
- **50+ test cases**
- **Coverage areas**: Sanity checks, isolation, boundaries, collisions

## Test Results

All tests verify that:
1. ✅ Data can be stored for any (market_id, oracle_id) pair
2. ✅ Multiple oracle_ids per market maintain independence
3. ✅ Same oracle_id in different markets maintain isolation
4. ✅ Large ID values don't cause collisions
5. ✅ Updating one oracle doesn't affect others
6. ✅ Timestamps are independent per oracle
7. ✅ Theoretical collision scenarios don't occur

### Running the Tests

All tests:
```bash
cargo test --lib modules::oracles_test
```

Multi-oracle specific tests:
```bash
# Isolation within market
cargo test --lib modules::oracles_test::test_multi_oracle_isolation_same_market -- --nocapture

# Isolation across markets
cargo test --lib modules::oracles_test::test_multi_oracle_isolation_different_markets -- --nocapture

# Matrix combinations (comprehensive)
cargo test --lib modules::oracles_test::test_multi_oracle_matrix_combinations -- --nocapture

# Large ID boundaries
cargo test --lib modules::oracles_test::test_multi_oracle_large_ids -- --nocapture

# Sequential updates (write isolation)
cargo test --lib modules::oracles_test::test_multi_oracle_sequential_updates -- --nocapture

# Collision mitigation verification
cargo test --lib modules::oracles_test::test_multi_oracle_collision_mitigation -- --nocapture
```

## Theoretical Collision Scenarios Tested

### Scenario 1: String Concatenation Collision
```
If keys were naively concatenated:
  (1, 0) → "1_0" 
  (10, 0) → "10_0"  ← Could collide with (1, 0) under simple concatenation
```
**Test**: `test_multi_oracle_collision_mitigation` - Scenario 1

### Scenario 2: Bit-Packing Overflow
```
If market_id overflow wasn't handled:
  (u32::MAX + 1, 0) could collide with (0, 0)
    if only lower 32 bits used
```
**Test**: `test_multi_oracle_large_ids` and `test_multi_oracle_collision_mitigation` - Scenario 2

### Scenario 3: Hash Collision
```
If composite key hashing was poor:
  Different (market_id, oracle_id) could hash to same slot
```
**Test**: `test_multi_oracle_matrix_combinations` - Statistical verification

### Scenario 4: Reversed Pair Collision
```
If key structure wasn't ordered:
  (1, 100) could be indistinguishable from (100, 1)
```
**Test**: `test_multi_oracle_collision_mitigation` - Scenario 5

### Scenario 5: Adjacent Value Collision
```
If only partial bits used:
  Adjacent oracle_ids or market_ids could collide
```
**Test**: `test_multi_oracle_collision_mitigation` - Scenarios 3-4

## Implementation Details

### Key Storage Mechanism
```rust
// Storage of oracle result
e.storage()
    .persistent()
    .set(&OracleData::Result(market_id, oracle_id), &outcome);

// Retrieval of oracle result
e.storage()
    .persistent()
    .get(&OracleData::Result(market_id, oracle_id))
```

### Soroban Storage Security
Soroban's storage system uses:
- **Composite key derivation**: Each enum variant creates unique key prefix
- **Type safety**: `#[contracttype]` ensures type integrity
- **Persistent storage**: No key collisions across different enum variants

This means:
- `OracleData::Result(1, 0)` has different storage from `OracleData::LastUpdate(1, 0)`
- The Soroban runtime handles proper key composition

## Acceptance Criteria Met

✅ **Multi-Oracle Support**: Code supports arbitrary oracle_ids per market
✅ **Key Isolation**: Tests verify no collisions across (market_id, oracle_id) pairs
✅ **Data Independence**: Tests ensure updates don't cross-pollinate
✅ **Boundary Testing**: Large ID values properly handled
✅ **Documentation**: Comprehensive test documentation with collision scenarios

## Next Steps

1. **Code Review**: Verify test coverage is comprehensive
2. **Continuous Integration**: Run tests on every commit
3. **Production Deployment**: Deploy with confidence in isolation guarantees
4. **Monitoring**: Track oracle aggregation in multi-oracle markets

## References

- **Related Issue**: Issue #9 - Global Outcome Key Collisions in Oracles
- **Related Issue**: Issue #260 - Confidence Threshold Rounding Tests
- **Mitigation Implemented**: Composite key structure `(market_id, oracle_id)`
- **Test File**: `contracts/predict-iq/src/modules/oracles_test.rs`
- **Code Under Test**: `contracts/predict-iq/src/modules/oracles.rs`

---

**Last Updated**: 2026-03-26
**Test Coverage**: 8 test functions, 50+ test cases
**Status**: ✅ Ready for Integration
