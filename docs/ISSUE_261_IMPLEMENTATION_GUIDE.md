# Issue #261 Quick Implementation Reference

## Problem
Multi-oracle support requires ensuring no key collisions between `(market_id, oracle_id)` pairs in persistent storage.

## Test Files
- **Primary**: `contracts/predict-iq/src/modules/oracles_test.rs`
- **Code Under Test**: `contracts/predict-iq/src/modules/oracles.rs`
- **Documentation**: `docs/ISSUE_261_MULTI_ORACLE_KEYING.md`

## Current Behavior ✅

The implementation already supports composite keys:
```rust
#[contracttype]
pub enum OracleData {
    Result(u64, u32),     // (market_id, oracle_id) -> outcome
    LastUpdate(u64, u32), // (market_id, oracle_id) -> timestamp
}
```

Functions properly use oracle_id parameter:
```rust
pub fn get_oracle_result(e: &Env, market_id: u64, oracle_id: u32) -> Option<u32> {
    e.storage()
        .persistent()
        .get(&OracleData::Result(market_id, oracle_id))
}

pub fn resolve_with_pyth(
    e: &Env,
    market_id: u64,
    oracle_id: u32,  // ✅ Correctly uses oracle_id
    config: &OracleConfig,
) -> Result<u32, ErrorCode> {
    // ... validation ...
    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, oracle_id), &outcome);
    Ok(outcome)
}
```

## Known Issue ⚠️

`set_oracle_result()` still hardcodes oracle_id=0:
```rust
pub fn set_oracle_result(e: &Env, market_id: u64, outcome: u32) -> Result<(), ErrorCode> {
    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, 0), &outcome);  // ← Hardcoded!
    // ...
}
```

**Fix**: Add oracle_id parameter:
```rust
pub fn set_oracle_result(
    e: &Env, 
    market_id: u64, 
    oracle_id: u32,  // ← Add this parameter
    outcome: u32
) -> Result<(), ErrorCode> {
    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, oracle_id), &outcome);  // ✅ Use it
    e.storage().persistent().set(
        &OracleData::LastUpdate(market_id, oracle_id),
        &e.ledger().timestamp(),
    );
    // ...
}
```

## Test Verification After Fix

1. **All current tests pass**:
   ```bash
   cargo test --lib modules::oracles_test
   ```

2. **Multi-oracle isolation verified**:
   ```bash
   cargo test --lib modules::oracles_test::test_multi_oracle_isolation_same_market -- --nocapture
   cargo test --lib modules::oracles_test::test_multi_oracle_isolation_different_markets -- --nocapture
   ```

3. **No collisions detected**:
   ```bash
   cargo test --lib modules::oracles_test::test_multi_oracle_collision_mitigation -- --nocapture
   ```

4. **Edge cases handled**:
   ```bash
   cargo test --lib modules::oracles_test::test_multi_oracle_large_ids -- --nocapture
   ```

## Key Test Cases Covered

### Isolation Tests
- ✅ 4+ oracle_ids per market with independent outcomes
- ✅ Same oracle_id across 5+ different markets with independent outcomes  
- ✅ 12-pair matrix (3 markets × 4 oracles) with 100% retrieval accuracy
- ✅ Large ID values (u64::MAX, u32::MAX)
- ✅ Sequential updates don't affect adjacent oracle_ids
- ✅ Timestamp independence per oracle
- ✅ 5 theoretical collision scenarios verified as prevented

### Edge Cases
- Market ID = u64::MAX
- Market ID = u64::MAX - 1
- Oracle ID = u32::MAX
- Oracle ID = u32::MAX - 1
- Reversed pairs: (1, 100) vs (100, 1)
- Adjacent values: (1000, 1) vs (1000, 2)

## Integration Checklist

- [ ] Modification compiles without errors
- [ ] All `oracles_test` tests pass
- [ ] No performance regression
- [ ] Related tests pass:
  - [ ] `modules::markets_test`
  - [ ] `modules::resolution_test`
- [ ] Integration tests pass
- [ ] Code review completed
- [ ] PR description references Issue #261

## Fix Validation

To verify `set_oracle_result()` fix:

1. Update function signature
2. Update all call sites to pass oracle_id
3. Run tests:
   ```bash
   cargo test --lib modules::oracles_test::test_multi_oracle_isolation_same_market
   ```
4. Verify all assertions pass

## Measurement Points

**Before Fix**:
- `set_oracle_result` hardcoded oracle_id=0
- Multi-oracle support limited for this function

**After Fix**:
- `set_oracle_result` accepts oracle_id parameter
- Full multi-oracle support enabled
- All tests pass
- No collisions detected

## Performance Notes

- Storage operations: O(1) per access
- No performance impact from multi-oracle support
- Composite key hashing handled by Soroban runtime

## References

- **Issue**: #261 - Multi-Oracle Keying Tests
- **Related Issue**: #9 - Global Outcome Key Collisions
- **Area**: Oracle validation and multi-oracle aggregation
- **Algorithm**: Composite key storage with (market_id, oracle_id) pairs

---

**Last Updated**: 2026-03-26
**Status**: ✅ Tests Ready, Implementation Complete (except set_oracle_result)
