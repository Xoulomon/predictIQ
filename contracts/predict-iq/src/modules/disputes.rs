use soroban_sdk::{Env, Address, String};
use crate::types::{MarketStatus, Market};
use crate::modules::markets;

pub fn file_dispute(e: &Env, disciplinarian: Address, market_id: u64) {
    disciplinarian.require_auth();

    let mut market = markets::get_market(e, market_id).expect("Market not found");
    
    if market.status != MarketStatus::PendingResolution {
        panic!("Market cannot be disputed in current state");
    }

    market.status = MarketStatus::Disputed;
    // Extend resolution deadline for voting period
    market.resolution_deadline += 86400 * 3; // 3 days extension

    markets::update_market(e, market);

    e.events().publish(
        (String::from_str(e, "market_disputed"), market_id),
        disciplinarian,
    );
}

pub fn resolve_market(e: &Env, market_id: u64, winning_outcome: u32) {
    // This would ideally be called by an admin or triggered by oracle/consensus
    let mut market = markets::get_market(e, market_id).expect("Market not found");
    
    market.status = MarketStatus::Resolved;
    market.winning_outcome = Some(winning_outcome);

    markets::update_market(e, market);

    e.events().publish(
        (String::from_str(e, "market_resolved"), market_id),
        winning_outcome,
    );
}
