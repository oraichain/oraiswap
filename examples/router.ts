import "dotenv/config";
import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { contracts } from "../build";

(async () => {
  const client = await CosmWasmClient.connect(process.env.RPC_URL);

  const routerClient = new contracts.OraiswapRouter.OraiswapRouterQueryClient(
    client,
    process.env.ROUTER_CONTRACT
  );

  const ret = await routerClient.simulateSwapOperations({
    offerAmount: "10000000",
    operations: [
      {
        orai_swap: {
          offer_asset_info: {
            native_token: { denom: process.env.DENOM },
          },
          ask_asset_info: {
            token: {
              contract_addr: process.env.ORAIX_CONTRACT,
            },
          },
        },
      },
    ],
  });

  console.log(ret);
})();
