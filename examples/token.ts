import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import "dotenv/config";
import { OraiswapTokenQueryClient } from "../contracts/oraiswap_token/artifacts/ts/OraiswapToken.client";

(async () => {
  const client = await CosmWasmClient.connect(process.env.RPC_URL);
  const tokenClient = new OraiswapTokenQueryClient(
    client,
    process.env.ORAIX_CONTRACT
  );

  const accounts = await tokenClient.allAccounts({ limit: 10 });

  console.log(accounts);
})();
