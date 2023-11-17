import { SimulateCosmWasmClient } from "@oraichain/cw-simulate";
import {
  CONVERTER_CONTRACT,
  FACTORY_V2_CONTRACT,
  ORACLE_CONTRACT,
  STAKING_CONTRACT,
  ROUTER_V2_CONTRACT,
  ORAIX_CONTRACT,
  REWARDER_CONTRACT,
} from "@oraichain/oraidex-common";
import loadAllState from "../src/loadAllState";
import {
  OraiswapConverterClient,
  OraiswapFactoryClient,
  OraiswapRewarderClient,
  OraiswapRouterClient,
  OraiswapStakingClient,
  OraiswapTokenClient,
} from "../build/contracts";

const listContracts = [
  CONVERTER_CONTRACT,
  FACTORY_V2_CONTRACT,
  ORACLE_CONTRACT,
  STAKING_CONTRACT,
  ROUTER_V2_CONTRACT,
  ORAIX_CONTRACT,
  REWARDER_CONTRACT,
];

const client = new SimulateCosmWasmClient({
  bech32Prefix: "orai",
  chainId: "Oraichain",
  metering: true,
});

describe("Simulate oraiswap contract test", () => {
  const sender = "orai12p0ywjwcpa500r9fuf0hly78zyjeltakrzkv0c";
  let converterContract: OraiswapConverterClient;
  let factoryContract: OraiswapFactoryClient;
  let oraiXContract: OraiswapTokenClient;
  let rewarderContract: OraiswapRewarderClient;
  let stakeContract: OraiswapStakingClient;
  let routerContract: OraiswapRouterClient;

  beforeAll(async () => {
    await loadAllState(client, listContracts, sender, "../data");
    converterContract = new OraiswapConverterClient(
      client,
      sender,
      CONVERTER_CONTRACT,
    );
    factoryContract = new OraiswapFactoryClient(
      client,
      sender,
      FACTORY_V2_CONTRACT,
    );
    // oracleContract = new OraiswapOracleClient(client, sender, ORACLE_CONTRACT);

    oraiXContract = new OraiswapTokenClient(client, sender, ORAIX_CONTRACT);

    rewarderContract = new OraiswapRewarderClient(
      client,
      sender,
      REWARDER_CONTRACT,
    );
    stakeContract = new OraiswapStakingClient(client, sender, STAKING_CONTRACT);
    routerContract = new OraiswapRouterClient(
      client,
      sender,
      ROUTER_V2_CONTRACT,
    );
    oraiXContract = new OraiswapTokenClient(client, sender, ORAIX_CONTRACT);
  }, 600000);

  it("should loadState successfully", async () => {
    const converterInfo = await converterContract.config();
    const factoryInfo = await factoryContract.config();
    const oraiXInfo = await oraiXContract.tokenInfo();
    const stakeInfo = await stakeContract.config();
    const routerInfo = await routerContract.config();
    const rewarderInfo = await rewarderContract.config();

    expect(converterInfo).toBeDefined();
    expect(factoryInfo).toBeDefined();
    expect(oraiXInfo).toBeDefined();
    expect(stakeInfo).toBeDefined();
    expect(routerInfo).toBeDefined();
    expect(rewarderInfo).toBeDefined();
  });
});
