#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, IntoVal, Symbol, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAdminChange {
    pub new_admin: Address,
    pub effective_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Contract-isolated storage namespace root (#826).
    NamespaceRoot,
    Admin,
    Contracts,
}

#[contract]
pub struct AdminRotationCoordinator;

#[contractimpl]
impl AdminRotationCoordinator {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Contracts, &Vec::<Address>::new(&env));
    }

    pub fn register_contract(env: Env, admin: Address, contract: Address) {
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        if stored_admin != admin {
            panic!("unauthorized");
        }
        admin.require_auth();
        let mut contracts: Vec<Address> = env.storage().instance().get(&DataKey::Contracts).unwrap_or(Vec::new(&env));
        if !contracts.iter().any(|existing| existing == contract) {
            contracts.push_back(contract);
            env.storage().instance().set(&DataKey::Contracts, &contracts);
        }
    }

    pub fn batch_propose_admin_change(env: Env, admin: Address, new_admin: Address) {
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        if stored_admin != admin {
            panic!("unauthorized");
        }
        admin.require_auth();
        let contracts: Vec<Address> = env.storage().instance().get(&DataKey::Contracts).unwrap_or(Vec::new(&env));
        for contract in contracts.iter() {
            let _: () = env.invoke_contract(&contract, &Symbol::new(&env, "propose_admin_change"), (admin.clone(), new_admin.clone()).into_val(&env));
        }
    }

    pub fn get_registered_contracts(env: Env) -> Vec<Address> {
        env.storage().instance().get(&DataKey::Contracts).unwrap_or(Vec::new(&env))
    }
}

// ---------------------------------------------------------------------------
// Tests (#1262)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_batch_propose_admin_change_integration() {
        let env = Env::default();
        env.mock_all_auths();

        // Set up coordinator
        let coordinator_admin = Address::generate(&env);
        let coordinator = env.register_contract(None, AdminRotationCoordinator);

        // Initialize coordinator
        let mut client = AdminRotationCoordinatorClient::new(&env, &coordinator);
        client.initialize(&coordinator_admin);

        // Create 3 test contracts
        let contract1 = env.register_contract_wasm(None, b"contract1");
        let contract2 = env.register_contract_wasm(None, b"contract2");
        let contract3 = env.register_contract_wasm(None, b"contract3");

        // Register contracts
        client.register_contract(&coordinator_admin, &contract1);
        client.register_contract(&coordinator_admin, &contract2);
        client.register_contract(&coordinator_admin, &contract3);

        // Verify all contracts are registered
        let registered = client.get_registered_contracts();
        assert_eq!(registered.len(), 3);

        // Perform batch admin change
        let new_admin = Address::generate(&env);
        client.batch_propose_admin_change(&coordinator_admin, &new_admin);
    }

    #[test]
    fn test_register_contract_prevents_duplicates() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let coordinator = env.register_contract(None, AdminRotationCoordinator);

        let mut client = AdminRotationCoordinatorClient::new(&env, &coordinator);
        client.initialize(&admin);

        let contract = env.register_contract_wasm(None, b"test");

        // Register same contract twice
        client.register_contract(&admin, &contract);
        client.register_contract(&admin, &contract);

        // Should only be in the list once
        let registered = client.get_registered_contracts();
        assert_eq!(registered.len(), 1);
    }

    #[test]
    fn test_get_registered_contracts_returns_all() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let coordinator = env.register_contract(None, AdminRotationCoordinator);

        let mut client = AdminRotationCoordinatorClient::new(&env, &coordinator);
        client.initialize(&admin);

        // Register 3 contracts
        let contract1 = env.register_contract_wasm(None, b"c1");
        let contract2 = env.register_contract_wasm(None, b"c2");
        let contract3 = env.register_contract_wasm(None, b"c3");

        client.register_contract(&admin, &contract1);
        client.register_contract(&admin, &contract2);
        client.register_contract(&admin, &contract3);

        let registered = client.get_registered_contracts();
        assert_eq!(registered.len(), 3);
    }
}
