use cosmwasm_schema::cw_serde;
use cosmwasm_std::{testing::mock_dependencies, Api, CanonicalAddr, StdResult, Storage};
use cosmwasm_storage::singleton;

use crate::state::read_config;

#[cw_serde]
pub struct ContractInfoOld {
    pub name: String,
    pub version: String,
    // admin can update the parameter, may be multisig
    pub admin: CanonicalAddr,
    pub commission_rate: String,
    pub reward_address: CanonicalAddr,
}

static CONTRACT_INFO: &[u8] = b"contract_info"; // contract info

pub fn store_config_old(storage: &mut dyn Storage, config: &ContractInfoOld) -> StdResult<()> {
    singleton(storage, CONTRACT_INFO).save(config)
}

#[test]
fn test_migrate_contract_info() {
    // fixture
    let mut deps = mock_dependencies();
    let contract_info = ContractInfoOld {
        name: "foo".to_string(),
        version: "1".to_string(),
        admin: deps.api.addr_canonicalize("admin").unwrap(),
        commission_rate: "1".to_string(),
        reward_address: deps.api.addr_canonicalize("reward").unwrap(),
    };
    store_config_old(deps.as_mut().storage, &contract_info).unwrap();

    let config = read_config(deps.as_ref().storage).unwrap();
    assert_eq!(config.name, "foo".to_string());
    assert_eq!(config.operator, None);
}
