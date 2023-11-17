import { DownloadState, SimulateCosmWasmClient } from "@oraichain/cw-simulate";
import { existsSync } from "fs";
import { resolve } from "path";

async function loadAllState(
  client: SimulateCosmWasmClient,
  listContracts: string[],
  sender: string,
  path: string,
) {
  // abitrary sender
  const dataPath = resolve(__dirname, path);
  const downloadState = new DownloadState("https://lcd.orai.io", dataPath);
  const allState = await Promise.all(
    listContracts.map((contract) => {
      if (existsSync(`${dataPath}/${contract}.state`)) {
        return Promise.resolve();
      }
      return downloadState.saveState(contract);
    }),
  );

  console.log("Load state ...");

  await Promise.all(
    allState.map((state, i) =>
      downloadState.loadState(client, sender, listContracts[i], "label", state),
    ),
  );

  console.log("Load state successfully");
  return client;
}

export default loadAllState;
