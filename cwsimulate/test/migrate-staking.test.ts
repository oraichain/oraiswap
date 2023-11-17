import {
  DownloadState,
  SimulateCosmWasmClient,
  SortedMap,
  BufferCollection,
  compare,
} from "@oraichain/cw-simulate";
import { STAKING_CONTRACT } from "@oraichain/oraidex-common";
import { OraiswapStakingClient } from "../build/contracts";
import { existsSync, readFileSync } from "fs";
import { resolve } from "path";

const client = new SimulateCosmWasmClient({
  bech32Prefix: "orai",
  chainId: "Oraichain",
  metering: true,
});
const stakeAdmin = "orai1gkr56hlnx9vc7vncln2dkd896zfsqjn300kfq0";
const dataPath = resolve(__dirname, "../data");

const downloadState = new DownloadState("https://lcd.orai.io", dataPath);
describe("Simulate oraiswap contract test", () => {
  const sender = "orai12p0ywjwcpa500r9fuf0hly78zyjeltakrzkv0c";
  let stakeContract: OraiswapStakingClient;
  let codeId;

  beforeAll(async () => {
    ({ codeId } = await client.upload(
      stakeAdmin,
      readFileSync(resolve(__dirname, "../build/wasm/oraiswap_staking.wasm")),
      "auto",
    ));
    if (!existsSync(`${dataPath}/${STAKING_CONTRACT}.state`)) {
      await downloadState.saveState(STAKING_CONTRACT);
    }

    // const buffer = readFileSync(`${dataPath}/${STAKING_CONTRACT}.state`);
    // // @ts-ignore
    // const state = SortedMap.rawPack(new BufferCollection(buffer), compare);
    //
    // await client.loadContract(
    //   STAKING_CONTRACT,
    //   { codeId, label: "label", admin: sender, creator: sender, created: 1 },
    //   state,
    // );
    await downloadState.loadState(client, sender, STAKING_CONTRACT, "label");
    stakeContract = new OraiswapStakingClient(client, sender, STAKING_CONTRACT);
  }, 600000);

  it("should loadState successfully", async () => {
    const stakeInfo = await stakeContract.config();

    expect(stakeInfo).toBeDefined();
  });

  it("should migrate stake contract succesfully", async () => {
    // const oldTotalAssetKey = await stakeContract.totalAssetKey();
    // console.log("oldTotalAssetKey", oldTotalAssetKey);

    const tx = await client.migrate(
      sender,
      STAKING_CONTRACT,
      codeId,
      {},
      "auto",
    );
    console.log("GasUsed", tx.gasUsed);

    const newTotalAssetKey = await stakeContract.totalAssetKey();
    console.log("newTotalAssetKey", newTotalAssetKey);

    const result = await Promise.allSettled(
      newTotalAssetKey.map(async (key) => {
        return stakeContract.poolInfo({ stakingToken: key });
      }),
    );

    result.forEach((poolInfo, i) => {
      console.log(newTotalAssetKey[i]);
      console.log({ poolInfo });
    });

    // expect(oldTotalAssetKey.length).toEqual(newTotalAssetKey.length);
  });
});
