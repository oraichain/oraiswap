import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import "dotenv/config";
import { contracts } from "../build";

(async () => {
  const client = await CosmWasmClient.connect(process.env.RPC_URL);

  const tokenClient = new contracts.OraiswapToken.OraiswapTokenQueryClient(
    client,
    process.env.ORAIX_CONTRACT
  );

  const accounts = await tokenClient.allAccounts({ limit: 10 });

  console.log(accounts);
})();
