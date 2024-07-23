// @generated
// This file is @generated by prost-build.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Memo {
    #[prost(message, optional, tag="1")]
    pub user_swap: ::core::option::Option<memo::UserSwap>,
    /// string because the minimum receive may be very high due to decimal points
    #[prost(string, tag="2")]
    pub minimum_receive: ::prost::alloc::string::String,
    #[prost(uint64, tag="3")]
    pub timeout_timestamp: u64,
    #[prost(message, optional, tag="4")]
    pub post_swap_action: ::core::option::Option<memo::PostAction>,
    #[prost(string, tag="5")]
    pub recovery_addr: ::prost::alloc::string::String,
}
/// Nested message and enum types in `Memo`.
pub mod memo {
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
    pub struct SwapExactAssetIn {
        #[prost(string, tag="1")]
        pub offer_amount: ::prost::alloc::string::String,
        #[prost(message, repeated, tag="2")]
        pub operations: ::prost::alloc::vec::Vec<SwapOperation>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
    pub struct SmartSwapExactAssetIn {
        #[prost(message, repeated, tag="1")]
        pub routes: ::prost::alloc::vec::Vec<Route>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Route {
        #[prost(string, tag="1")]
        pub offer_amount: ::prost::alloc::string::String,
        #[prost(message, repeated, tag="2")]
        pub operations: ::prost::alloc::vec::Vec<SwapOperation>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
    pub struct SwapOperation {
        #[prost(string, tag="1")]
        pub pool_id: ::prost::alloc::string::String,
    }
    /// if none is provided -> error, if more than one attributes are provided ->
    /// error
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
    pub struct UserSwap {
        #[prost(message, optional, tag="1")]
        pub swap_exact_asset_in: ::core::option::Option<SwapExactAssetIn>,
        #[prost(message, optional, tag="2")]
        pub smart_swap_exact_asset_in: ::core::option::Option<SmartSwapExactAssetIn>,
    }
    /// Can possibly have both? -> if both then always contract_call first then ibc
    /// transfer
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PostAction {
        #[prost(message, optional, tag="1")]
        pub ibc_transfer_msg: ::core::option::Option<IbcTransfer>,
        #[prost(message, optional, tag="2")]
        pub contract_call: ::core::option::Option<ContractCall>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IbcTransfer {
        #[prost(string, tag="1")]
        pub source_channel: ::prost::alloc::string::String,
        #[prost(string, tag="2")]
        pub source_port: ::prost::alloc::string::String,
        #[prost(string, tag="3")]
        pub receiver: ::prost::alloc::string::String,
        #[prost(string, tag="4")]
        pub memo: ::prost::alloc::string::String,
        #[prost(string, tag="5")]
        pub recover_address: ::prost::alloc::string::String,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ContractCall {
        #[prost(string, tag="1")]
        pub contract_address: ::prost::alloc::string::String,
        #[prost(string, tag="2")]
        pub msg: ::prost::alloc::string::String,
    }
}
// @@protoc_insertion_point(module)
