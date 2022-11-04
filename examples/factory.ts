import "dotenv/config";

import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { OraiswapFactoryClient } from "../contracts/oraiswap_factory/artifacts/ts/OraiswapFactory.client";

(async () => {
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(
    process.env.MNEMONIC,
    {
      prefix: process.env.PREFIX,
    }
  );
  const [firstAccount] = await wallet.getAccounts();
  const client = await SigningCosmWasmClient.connectWithSigner(
    process.env.RPC_URL,
    wallet
  );
  const factoryClient = new OraiswapFactoryClient(
    client,
    firstAccount.address,
    process.env.FACTORY_CONTRACT
  );

  const pairs = await factoryClient.pairs({ limit: 10 });

  console.log(pairs);
})();
