import { DownloadState, SimulateCosmWasmClient } from "@oraichain/cw-simulate";
import {
  STAKING_CONTRACT,
  USDT_CONTRACT,
  ATOM_ORAICHAIN_DENOM,
  parseRpcEvents,
} from "@oraichain/oraidex-common";
import { AssetInfo, OraiswapStakingClient } from "../build/contracts";
import { existsSync, readFileSync } from "fs";
import { resolve } from "path";
import { MigrateMsg } from "../build/contracts/OraiswapStaking.types";

const client = new SimulateCosmWasmClient({
  bech32Prefix: "orai",
  chainId: "Oraichain",
  metering: true,
});
const stakeAdmin = "orai1gkr56hlnx9vc7vncln2dkd896zfsqjn300kfq0";
const dataPath = resolve(__dirname, "../data");

const downloadState = new DownloadState("https://lcd.orai.io", dataPath);

async function migrate_pool(
  client: any,
  old_asset: AssetInfo,
  sender: string,
  codeId: number,
) {
  let next_staker: string;
  while (true) {
    const migrateMsg: MigrateMsg = {
      asset_info: old_asset,
      staker_after: next_staker,
      limit: 1000,
    };
    const tx = await client.migrate(
      sender,
      STAKING_CONTRACT,
      codeId,
      migrateMsg,
      "auto",
    );
    console.log("gasUsed: ", tx.gasUsed);
    next_staker = tx.events
      .find((e) => e.type === "wasm")
      .attributes.find((e) => e.key == "next_staker").value;
    console.log("next_staker", next_staker);
    if (!next_staker) {
      break;
    }
  }
}

describe("Simulate oraiswap contract test", () => {
  const sender = "orai12p0ywjwcpa500r9fuf0hly78zyjeltakrzkv0c";
  let stakeContract: OraiswapStakingClient;
  let codeId: number;

  beforeAll(async () => {
    ({ codeId } = await client.upload(
      stakeAdmin,
      readFileSync(resolve(__dirname, "../build/wasm/oraiswap_staking.wasm")),
      "auto",
    ));
    if (!existsSync(`${dataPath}/${STAKING_CONTRACT}.state`)) {
      await downloadState.saveState(STAKING_CONTRACT);
    }

    await downloadState.loadState(client, sender, STAKING_CONTRACT, "label");
    stakeContract = new OraiswapStakingClient(client, sender, STAKING_CONTRACT);
  }, 600000);

  it("should loadState successfully", async () => {
    const stakeInfo = await stakeContract.config();

    expect(stakeInfo).toBeDefined();
  });

  it("should migrate stake contract with cw20 token succesfully", async () => {
    await migrate_pool(
      client,
      {
        token: {
          contract_addr: USDT_CONTRACT,
        },
      },
      sender,
      codeId,
    );
  });

  it("should migrate stake contract with cw20 token succesfully", async () => {
    await migrate_pool(
      client,
      {
        native_token: {
          denom: ATOM_ORAICHAIN_DENOM,
        },
      },
      sender,
      codeId,
    );
  });
});
