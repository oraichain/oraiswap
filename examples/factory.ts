import "dotenv/config";

import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { contracts } from "../build";

(async () => {
  const client = await CosmWasmClient.connect(process.env.RPC_URL);
  const factoryClient =
    new contracts.OraiswapFactory.OraiswapFactoryQueryClient(
      client,
      process.env.FACTORY_CONTRACT
    );

  const { pairs } = await factoryClient.pairs({ limit: 10 });

  const ret = await Promise.all(
    pairs.map((pair) => {
      const pairClient = new contracts.OraiswapPair.OraiswapPairQueryClient(
        client,
        pair.contract_addr
      );
      return pairClient.pool();
    })
  );
  console.log(JSON.stringify(ret, null, 2));
})();
