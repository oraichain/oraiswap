import { DownloadState, SimulateCosmWasmClient } from "@oraichain/cw-simulate";
import {
  STAKING_CONTRACT,
  USDT_CONTRACT,
  ATOM_ORAICHAIN_DENOM,
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

const hardcode_data = {
  old_asset: [
    "orai19q4qak2g3cj2xc2y3060t0quzn3gfhzx08rjlrdd3vqxhjtat0cq668phq",
    "orai19rtmkk6sn4tppvjmp5d5zj6gfsdykrl5rw2euu5gwur3luheuuusesqn49",
    "orai1gzvndtzceqwfymu2kqhta2jn6gmzxvzqwdgvjw",
    "orai12hzjxfh77wl572gdzct2fxv2arxcwh6gykc7qh",
    "ibc/4F7464EEE736CCFB6B444EB72DE60B3B43C0DD509FFA2B87E05D584467AAE8C8",
    "ibc/A2E2EEC9057A4A1C2C0A6A4C78B0239118DF5F278830F50B4A6BDD7A66506B78",
    "ibc/9C4DCD21B48231D0BC2AC3D1B74A864746B37E4292694C93C617324250D002FC",
    "ibc/9E4F68298EE0A201969E583100E5F9FAD145BAA900C04ED3B6B302D834D8E3C4",
    "orai1065qe48g7aemju045aeyprflytemx7kecxkf5m7u5h5mphd0qlcs47pclp",
    "orai10ldgzued6zjp0mkqwsv2mux3ml50l97c74x8sg",
    "orai1nd4r053e3kgedgld2ymen8l9yrw8xpjyaal7j5",
    "orai15un8msx3n5zf9ahlxmfeqd2kwa5wm0nrpxer304m9nd5q6qq0g6sku5pdd",
    "orai1c7tpjenafvgjtgm9aqwm7afnke6c56hpdms8jc6md40xs3ugd0es5encn0",
    "orai1l22k254e8rvgt5agjm3nn9sy0cmvhjmhd6ew6shacfmexkgzymhsyc2sr2",
    "orai1lus0f0rhx8s03gdllx2n6vhkmf0536dv57wfge",
    "orai1llsm2ly9lchj006cw2mmlu8wmhr0sa988sp3m5",
  ],
  lpArray: [
    "orai1hcjne0hmdj6pjrc3xuksucr0yplsa9ny7v047c34y8k8hfflq6yqyjapnn",
    "orai1slqw6gfvs6l2jgvh5ryjayf4g77d7sgfv6fumtyzcr06a6g9gnrq6c4rgg",
    "orai18ywllw03hvy720l06rme0apwyyq9plk64h9ccf",
    "orai1mav52eqhd07c3lwevcnqdykdzhh4733zf32jcn",
    "orai1ay689ltr57jt2snujarvakxrmtuq8fhuat5rnvq6rct89vjer9gqm2vde6",
    "orai1hxm433hnwthrxneyjysvhny539s9kh6s2g2n8y",
    "orai17rcfcrwltujfvx7w4l2ggyku8qrncy0hdvrzvc",
    "orai1e0x87w9ezwq2sdmvv5dq5ngzy98lt47tqfaf2m7zpkg49g5dj6fqred5d7",
    "orai1wgywgvumt5dxhm7vjpwx5es9ecrtl85qaqdspjqwx2lugy7vmw5qlwrn88",
    "orai1c4dzwmr73xfgdmazrf7zg3jqsxszrf9mccx46zw6tdduhxdyxfaqz4577u",
    "orai1qmy3uuxktflvreanaqph6yua7stjn6j65rur62",
    "orai1hesuzwmfuhuln0l6zss3gtpdwlmmlapp497k0a",
  ],
};

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

  it("should migrate first 10 old asset successfully", async () => {
    for (const old_asset of hardcode_data.old_asset.slice(0, 10)) {
      const asset_info = old_asset.startsWith("ibc/")
        ? { native_token: { denom: old_asset } }
        : { token: { contract_addr: old_asset } };
      await migrate_pool(client, asset_info, sender, codeId);
    }
  }, 6000000);

  it("should migrate rest old asset successfully", async () => {
    for (const old_asset of hardcode_data.old_asset.slice(10)) {
      const asset_info = old_asset.startsWith("ibc/")
        ? { native_token: { denom: old_asset } }
        : { token: { contract_addr: old_asset } };
      await migrate_pool(client, asset_info, sender, codeId);
    }
  }, 6000000);
  xit("should migrate stake contract with cw20 token succesfully", async () => {
    await migrate_pool(
      client,
      {
        token: {
          contract_addr: "orai1lus0f0rhx8s03gdllx2n6vhkmf0536dv57wfge",
        },
      },
      sender,
      codeId,
    );
  });

  xit("should migrate stake contract with cw20 token succesfully", async () => {
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
