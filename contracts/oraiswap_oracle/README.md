# Concepts

Store all params managed by multisig wallet.

## Treasury: acts as the central bank

adjust demand through adjusting reward, using multisig

- Tax Rewards: Income generated from transaction fees (stability fee)
- Seigniorage Rewards: Amount of seignorage generated from Orai swaps to Orai ow20 that is destined for multisig rewards inside the Oracle rewards
- Total Staked Orai: total Orai that has been staked by users and bonded by their delegated validators.

#### State

- TaxRate: tax rate is constant and normally fixed at 0.003%
- TaxCap: map a denom to an Uint128 that that represents that maximum income that can be generated from taxes on a transaction in that denomination

## Exchange: provides the Oraiswap with an up-to-date and accurate price feed of exchange rates

- Using multisig to vote for exchange rate
- Reward for the whitelist in multisig contract
