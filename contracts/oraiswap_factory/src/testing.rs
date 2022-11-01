use oraiswap::asset::{AssetInfo, PairInfo};

use oraiswap::mock_app::MockApp;
use oraiswap::pair::DEFAULT_COMMISSION_RATE;

#[test]
fn create_pair() {
    let mut app = MockApp::new();
    app.set_token_contract(oraiswap_token::testutils::contract());
    app.set_oracle_contract(oraiswap_oracle::testutils::contract());

    app.set_factory_and_pair_contract(
        crate::testutils::contract(),
        oraiswap_pair::testutils::contract(),
    );

    let contract_addr1 = app.create_token("assetA");
    let contract_addr2 = app.create_token("assetB");

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: contract_addr1,
        },
        AssetInfo::Token {
            contract_addr: contract_addr2,
        },
    ];

    // create pair
    app.set_pair(asset_infos.clone());

    // should never change commission rate once deployed
    let pair_res = app.query_pair(asset_infos.clone()).unwrap();
    assert_eq!(
        pair_res,
        PairInfo {
            oracle_addr: app.oracle_addr,
            liquidity_token: "Contract #5".into(),
            contract_addr: "Contract #4".into(),
            asset_infos,
            commission_rate: DEFAULT_COMMISSION_RATE.into()
        }
    );
}
