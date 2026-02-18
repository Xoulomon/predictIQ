use soroban_sdk::{Env, Address};
use crate::types::ConfigKey;

pub fn set_admin(e: &Env, admin: Address) {
    e.storage().persistent().set(&ConfigKey::Admin, &admin);
}

pub fn get_admin(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&ConfigKey::Admin)
}

pub fn require_admin(e: &Env) {
    let admin: Address = get_admin(e).expect("Admin not set");
    admin.require_auth();
}

pub fn set_market_admin(e: &Env, admin: Address) {
    require_admin(e);
    e.storage().persistent().set(&ConfigKey::MarketAdmin, &admin);
}

pub fn get_market_admin(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&ConfigKey::MarketAdmin)
}

pub fn set_fee_admin(e: &Env, admin: Address) {
    require_admin(e);
    e.storage().persistent().set(&ConfigKey::FeeAdmin, &admin);
}

pub fn get_fee_admin(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&ConfigKey::FeeAdmin)
}
