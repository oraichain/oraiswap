import { DownloadState, SimulateCosmWasmClient } from "@oraichain/cw-simulate";
import {
  CONVERTER_CONTRACT,
  FACTORY_V2_CONTRACT,
  ORACLE_CONTRACT,
  STAKING_CONTRACT,
  ROUTER_V2_CONTRACT,
  ORAIX_CONTRACT,
  REWARDER_CONTRACT,
} from "@oraichain/oraidex-common";
import { resolve } from "path";

const listContracts = [
  CONVERTER_CONTRACT,
  FACTORY_V2_CONTRACT,
  ORACLE_CONTRACT,
  STAKING_CONTRACT,
  ROUTER_V2_CONTRACT,
  ORAIX_CONTRACT,
  REWARDER_CONTRACT,
];

const dataPath = resolve(__dirname, "../data");
const downloadState = new DownloadState("https://lcd.orai.io", dataPath);
const client = new SimulateCosmWasmClient({
  bech32Prefix: "orai",
  chainId: "Oraichain",
  metering: true,
});
// abitrary sender
const sender = "orai12p0ywjwcpa500r9fuf0hly78zyjeltakrzkv0c";

async function loadAllState() {
  const allState = await Promise.all(
    listContracts.map((contract) => downloadState.saveState(contract)),
  );
  await Promise.all(
    allState.map((state, i) =>
      downloadState.loadState(client, sender, listContracts[i], "label", state),
    ),
  );
  console.log("Load state done");
}

loadAllState();

export default client;
