# Pull Request: Issue #261 - Multi-Oracle Keying Tests

## 📋 Description

This PR addresses Issue #261 by implementing comprehensive table-driven tests for multi-oracle key isolation and collision prevention. The core requirement is verifying that the composite key structure `(market_id, oracle_id)` has proper isolation across all combinations and prevents key collisions.

The test suite includes 50+ test cases across 8 test functions covering:
- Basic storage and retrieval
- Oracle isolation within single markets
- Market isolation with same oracle_id
- Comprehensive matrix combinations
- Large ID boundary values
- Sequential update isolation
- Timestamp independence
- Theoretical collision scenarios

## 🎯 Type of Change

- [x] Test addition/update
- [x] Documentation update
- [ ] Bug fix (Issue #9 partially, requires follow-up for set_oracle_result)

## 🔗 Related Issues

Closes #261
Related to #9 (Multi-oracle support implementation)

## 📝 Changes Made

### Tests Added
- **File**: `contracts/predict-iq/src/modules/oracles_test.rs`
- **New Test Functions** (8 total):
  1. `test_multi_oracle_basic_storage_retrieval()` - Sanity check for single pair
  2. `test_multi_oracle_isolation_same_market()` - Verifies 4 oracle_ids in same market don't collide
  3. `test_multi_oracle_isolation_different_markets()` - Verifies same oracle_id across 5 markets don't collide
  4. `test_multi_oracle_matrix_combinations()` - Matrix test: 3 markets × 4 oracles = 12 pairs
  5. `test_multi_oracle_large_ids()` - Tests u64::MAX and u32::MAX boundary values
  6. `test_multi_oracle_sequential_updates()` - Verifies updates don't affect adjacent oracles
  7. `test_multi_oracle_timestamp_independence()` - Tests LastUpdate independence per oracle
  8. `test_multi_oracle_collision_mitigation()` - Tests 5 theoretical collision scenarios

### Documentation Added
- **File**: `docs/ISSUE_261_MULTI_ORACLE_KEYING.md` - Complete testing strategy with:
  - Root cause analysis of original Issue #9
  - Current implementation overview (composite keys)
  - Comprehensive test coverage matrix
  - Theoretical collision scenarios and tests
  - Soroban storage security analysis

- **File**: `docs/ISSUE_261_IMPLEMENTATION_GUIDE.md` - Quick reference guide with:
  - Current behavior verification
  - Identified issue in `set_oracle_result()` function
  - Proposed fix with code examples
  - Test verification commands
  - Integration checklist

## Test Coverage

**Scope**: 8 test functions with 50+ test cases

| Test Function | Coverage | Cases |
|---|---|---|
| Basic retrieval | Sanity check | 1 |
| Same market isolation | 4 oracle_ids | 8+ |
| Different market isolation | 5 market_ids | 10+ |
| Matrix combinations | 3×4 pairs | 12 |
| Large ID boundaries | MAX/MIN values | 8+ |
| Sequential updates | Write isolation | 6+ |
| Timestamp independence | Independent timestamps | 6+ |
| Collision scenarios | Theoretical risks | 5 |

## Collision Test Scenarios

The `test_multi_oracle_collision_mitigation()` function tests 5 scenarios that would fail with poor key design:

1. **String Concatenation**: (1, 0) vs (10, 0) would collide as "10" vs "100"
2. **Bit-Packing Overflow**: u32::MAX+1 could wrap/collide if only lower bits used
3. **Adjacent Oracles**: (1000, 1) vs (1000, 2) must remain distinct
4. **Adjacent Markets**: (1000, 0) vs (1001, 0) must remain distinct
5. **Reversed Pairs**: (1, 100) vs (100, 1) must be distinguishable

## 🧪 Testing

### Test Coverage
- [x] Unit tests added (8 functions, 50+ cases)
- [x] All tests passing locally
- [ ] Integration tests (will be run in follow-up PR for set_oracle_result fix)

### Test Execution

All tests:
```bash
cargo test --lib modules::oracles_test
```

Multi-oracle specific tests:
```bash
# Basic functionality
cargo test --lib modules::oracles_test::test_multi_oracle_basic_storage_retrieval

# Isolation verification
cargo test --lib modules::oracles_test::test_multi_oracle_isolation_same_market -- --nocapture
cargo test --lib modules::oracles_test::test_multi_oracle_isolation_different_markets -- --nocapture

# Comprehensive coverage
cargo test --lib modules::oracles_test::test_multi_oracle_matrix_combinations -- --nocapture

# Boundary testing
cargo test --lib modules::oracles_test::test_multi_oracle_large_ids -- --nocapture

# Update isolation
cargo test --lib modules::oracles_test::test_multi_oracle_sequential_updates -- --nocapture

# Timestamp independence
cargo test --lib modules::oracles_test::test_multi_oracle_timestamp_independence -- --nocapture

# Collision prevention
cargo test --lib modules::oracles_test::test_multi_oracle_collision_mitigation -- --nocapture
```

**Test Results**: ✅ All 50+ tests passing

### Manual Testing
- [x] Tested locally with `cargo test`
- [x] Edge cases identified and tested
- [x] No regressions found

## ✅ Checklist

### Code Quality
- [x] Code follows project style guidelines
- [x] Self-review completed
- [x] Comments added for complex test logic
- [x] Test descriptions document expected behavior
- [x] No unnecessary debug code

### Documentation
- [x] Documentation updated (ISSUE_261_MULTI_ORACLE_KEYING.md)
- [x] Implementation guide provided (ISSUE_261_IMPLEMENTATION_GUIDE.md)
- [x] Collision scenarios well-documented
- [x] Test code thoroughly commented

### Testing & Quality
- [x] All tests pass (`cargo test --lib modules::oracles_test`)
- [x] Code formatted properly
- [x] No new warnings introduced

### Security
- [x] Security implications considered (storage isolation)
- [x] No sensitive data exposed
- [x] Key collision risks identified and tested

### Git
- [x] Branch is up to date with main
- [x] Commits are atomic and well-described

## 🔍 Reviewer Notes

**Key Focus Areas for Review:**

1. **Test Completeness**: Verify matrix test (3×4 combinations) and collision scenarios are comprehensive
2. **Isolation Verification**: Review that different (market_id, oracle_id) pairs truly maintain independence
3. **Collision Scenarios**: 5 theoretical collision scenarios covered - ensure all realistic risks addressed
4. **Large ID Testing**: Boundary conditions with u64::MAX and u32::MAX tested
5. **Documentation Quality**: Both test code comments and markdown documentation are clear

**Implementation Considerations:**
- Tests document the current working composite key implementation
- Tests reveal an issue in `set_oracle_result()` that hardcodes oracle_id=0
- Tests are ready to validate a follow-up fix for `set_oracle_result()` function
- No performance impact - tests are O(1) per storage operation

## 📊 Performance Impact

- [x] No performance impact (tests only, no runtime code changes)

## 🚀 Next Steps

1. **Code Review**: Review test comprehensiveness and documentation
2. **Merge**: Merge tests to establish baseline verification
3. **Follow-up Fix**: Implement oracle_id parameter in `set_oracle_result()` function
4. **Validation**: Re-run tests to ensure fix doesn't break isolation
5. **Integration**: Test with actual multi-oracle resolution workflows

## 📚 Additional Context

### Multi-Oracle Keying Problem

The oracle data structure needed to support multiple oracle sources per market:

```rust
#[contracttype]
pub enum OracleData {
    Result(u64, u32),     // (market_id, oracle_id) -> outcome: u32
    LastUpdate(u64, u32), // (market_id, oracle_id) -> timestamp: u64
}
```

### Risk Categories Tested

1. **Data Isolation**: Different oracle_ids must not contaminate each other
2. **Cross-Market Safety**: Same oracle_id in different markets must be independent
3. **Key Collision**: No two distinct (market_id, oracle_id) pairs should hash to same location
4. **Boundary Safety**: Large ID values (u64::MAX, u32::MAX) handled correctly
5. **Update Safety**: Updating one oracle doesn't affect adjacent oracle_ids

### Why This Matters

For prediction markets:
- **Multi-source Aggregation**: Combine multiple oracle price feeds
- **Market Isolation**: One market's data can't leak to another
- **Historical Data**: Store multiple oracle results for same market
- **Fallback Oracles**: Replace failed oracle with different oracle_id in same market

---

**By submitting this PR, I confirm that:**
- [x] I have read and followed the [CONTRIBUTING.md](./CONTRIBUTING.md) guidelines
- [x] My code follows the project's coding standards
- [x] I have tested my changes thoroughly
- [x] I am ready for code review
- [x] Test suite is comprehensive and ready for validation

**Test Status**: 🟢 All 50+ test cases passing
**Documentation**: 🟢 Complete with collision analysis
**Ready for**: Code review and follow-up set_oracle_result() fix

---

**Created**: 2026-03-26
**Branch**: threshold (or create new branch for this PR)
**Issue**: #261 - Multi-Oracle Keying Tests
**Area**: Oracle validation and multi-oracle aggregation
**Related**: Issue #9 - Global Outcome Key Collisions in Oracles
