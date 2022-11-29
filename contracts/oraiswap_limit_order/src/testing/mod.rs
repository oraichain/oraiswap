mod contract_test;
mod orderbook_test;

#[macro_export]
macro_rules! jsonstr {
    ($arg:expr) => {
        String::from_utf8(cosmwasm_schema::schemars::_serde_json::to_vec_pretty(&$arg).unwrap())
            .unwrap()
    };
}
