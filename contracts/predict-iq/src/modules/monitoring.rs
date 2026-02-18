use soroban_sdk::{Env, String, contracttype};
use crate::modules::circuit_breaker;
use crate::types::CircuitBreakerState;

#[contracttype]
pub enum DataKey {
    ErrorCount,
    LastObservation,
}

pub fn track_error(e: &Env) {
    let mut count: u32 = e.storage().instance().get(&DataKey::ErrorCount).unwrap_or(0);
    count += 1;
    e.storage().instance().set(&DataKey::ErrorCount, &count);

    if count > 10 { // Threshold for automatic trigger
        // Automatically open the circuit breaker
        e.storage().persistent().set(&crate::types::ConfigKey::CircuitBreakerState, &CircuitBreakerState::Open);
        
        e.events().publish(
            (String::from_str(e, "automatic_circuit_breaker_trigger"),),
            count,
        );
    }
}

pub fn reset_monitoring(e: &Env) {
    e.storage().instance().set(&DataKey::ErrorCount, &0u32);
}
