import "dotenv/config";

import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { contracts } from "../build";

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
  const factoryClient = new contracts.OraiswapFactory.OraiswapFactoryClient(
    client,
    firstAccount.address,
    process.env.FACTORY_CONTRACT
  );

  const pairs = await factoryClient.pairs({ limit: 10 });

  console.log(pairs);
})();
