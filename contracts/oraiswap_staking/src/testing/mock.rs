use std::marker::PhantomData;

use bech32::{FromBase32, ToBase32};
use cosmwasm_std::{
    testing::{MockQuerier, MockStorage},
    Addr, Api, CanonicalAddr, Empty, OwnedDeps, StdError, StdResult,
};

const SHUFFLES_ENCODE: usize = 10;
const SHUFFLES_DECODE: usize = 2;

// MockPrecompiles zero pads all human addresses to make them fit the canonical_length
// it trims off zeros for the reverse operation.
// not really smart, but allows us to see a difference (and consistent length for canonical adddresses)
#[derive(Copy, Clone)]
pub struct MockApi {
    /// Length of canonical addresses created with this API. Contracts should not make any assumptions
    /// what this value is.
    pub canonical_length: usize,
}

impl Default for MockApi {
    fn default() -> Self {
        Self {
            canonical_length: 20,
        }
    }
}

impl Api for MockApi {
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        let canonical = self.addr_canonicalize(input)?;
        let normalized = self.addr_humanize(&canonical)?;
        if input != normalized {
            return Err(StdError::generic_err(
                "Invalid input: address not normalized",
            ));
        }

        Ok(Addr::unchecked(input))
    }

    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        match bech32::decode(input) {
            Ok((_, canon, _)) => Ok(Vec::from_base32(&canon).unwrap().into()),
            Err(error) => Err(StdError::generic_err(format!(
                "addr_canonicalize errored: {}",
                error
            ))),
        }
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        match bech32::encode(
            "orai",
            canonical.to_vec().to_base32(),
            bech32::Variant::Bech32,
        ) {
            Ok(human) => Ok(Addr::unchecked(human)),
            Err(error) => Err(StdError::generic_err(format!(
                "addr_humanize errored: {}",
                error
            ))),
        }
    }

    fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, cosmwasm_std::VerificationError> {
        todo!()
    }

    fn secp256k1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_param: u8,
    ) -> Result<Vec<u8>, cosmwasm_std::RecoverPubkeyError> {
        todo!()
    }

    fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, cosmwasm_std::VerificationError> {
        todo!()
    }

    fn ed25519_batch_verify(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<bool, cosmwasm_std::VerificationError> {
        todo!()
    }

    fn debug(&self, message: &str) {
        todo!()
    }
}

pub fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, MockQuerier, Empty> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::default(),
        custom_query_type: PhantomData,
    }
}
