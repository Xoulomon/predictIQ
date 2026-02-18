use soroban_sdk::{Env, String};
use crate::types::{ConfigKey, CircuitBreakerState};
use crate::modules::admin;

pub fn set_state(e: &Env, state: CircuitBreakerState) {
    admin::require_admin(e);
    e.storage().persistent().set(&ConfigKey::CircuitBreakerState, &state);

    e.events().publish(
        (String::from_str(e, "circuit_breaker_updated"),),
        state,
    );
}

pub fn get_state(e: &Env) -> CircuitBreakerState {
    e.storage().persistent().get(&ConfigKey::CircuitBreakerState).unwrap_or(CircuitBreakerState::Closed)
}

pub fn require_closed(e: &Env) {
    if get_state(e) == CircuitBreakerState::Open {
        panic!("Circuit breaker is OPEN");
    }
}
