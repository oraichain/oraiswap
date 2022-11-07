const codegen = require("@cosmwasm/ts-codegen").default;
const { execSync } = require("child_process");
const path = require("path");
const fs = require("fs");
const { TypescriptParser } = require("typescript-parser");

const genTS = async (contracts, outPath) => {
  fs.rmSync(outPath, { recursive: true, force: true });
  await codegen({
    contracts,
    outPath,

    // options are completely optional ;)
    options: {
      bundle: {
        bundleFile: "index.ts",
        scope: "contracts",
      },
      types: {
        enabled: true,
      },
      client: {
        enabled: true,
      },
      reactQuery: {
        enabled: process.argv.includes("--react-query"),
        optionalClient: true,
        version: "v4",
        mutations: true,
      },
      recoil: {
        enabled: false,
      },
      messageComposer: {
        enabled: false,
      },
    },
  });
  console.log("✨ all done!");
};

const isPrivateType = (type) => {
  return (
    type.endsWith("Response") ||
    type === "InstantiateMsg" ||
    type === "ExecuteMsg" ||
    type === "QueryMsg" ||
    type === "MigrateMsg"
  );
};

const fixTs = async (outPath) => {
  const parser = new TypescriptParser();
  const typeExt = ".types.ts";
  const typeData = {};
  for (const dir of fs.readdirSync(outPath)) {
    if (dir.endsWith(typeExt)) {
      const tsFile = path.join(outPath, dir);
      const tsData = fs.readFileSync(tsFile).toString();
      const parsed = await parser.parseSource(tsData);
      const modifiedTsData = [];
      const importData = [];
      for (let token of parsed.declarations) {
        const exportData = tsData.substring(token.start, token.end);
        if (!isPrivateType(token.name) && !typeData[token.name]) {
          typeData[token.name] = exportData;
          importData.push(token.name);
        } else {
          modifiedTsData.push(exportData);
        }
      }
      // import from types, and remove from client
      modifiedTsData.unshift(
        `import {${importData.join(", ")}} from "./types";`
      );
      fs.writeFileSync(tsFile, modifiedTsData.join("\n"));

      // update client file
      const clientName = path.basename(dir, typeExt);
      const clientFile = path.join(outPath, `${clientName}.client.ts`);
      const clientData = fs.readFileSync(clientFile).toString();

      fs.writeFileSync(
        clientFile,
        clientData.replace(
          new RegExp(
            `import\\s+\\{(.*?)\\}\\s+from\\s+"\\.\\/${clientName}\\.types";`
          ),
          (_, g1) => {
            const [clientImportData, typesImportData] = g1
              .trim()
              .split(/\s*,\s*/)
              .reduce(
                (ret, el) => {
                  ret[!importData.includes(el) ? 0 : 1].push(el);
                  return ret;
                },
                [[], []]
              );

            return `import {${typesImportData.join(
              ", "
            )}} from "./types";\nimport {${clientImportData.join(
              ", "
            )}} from "./${clientName}.types";`;
          }
        )
      );
    }
  }

  fs.writeFileSync(
    path.join(outPath, "types.ts"),
    Object.values(typeData).join("\n")
  );

  // add export from types
  const indexData = fs.readFileSync(path.join(outPath, "index.ts")).toString();
  if (indexData.indexOf('export * from "./types";') === -1) {
    fs.writeFileSync(
      path.join(outPath, "index.ts"),
      `${indexData}\nexport * from "./types";`
    );
  }
};

const force = process.argv.includes("--force") || process.argv.includes("-f");
const contractsFolder = path.resolve(__dirname, "contracts");
const contracts = [];
const tsFolder = path.resolve(__dirname, "build");

for (const dir of fs.readdirSync(contractsFolder)) {
  if (!dir.startsWith("oraiswap_")) continue;
  const package = path.resolve(contractsFolder, dir);

  const artifactsFolder = path.join(package, "artifacts");
  const schemaFolder = path.join(artifactsFolder, "schema");

  if (!fs.existsSync(artifactsFolder)) fs.mkdirSync(artifactsFolder);

  if (force) {
    const ret = execSync(`cargo run -q --bin schema`, {
      cwd: artifactsFolder,
    }).toString();
    console.log(ret);
  }

  const baseName = path.basename(package);
  const name = baseName.replace(/^.|_./g, (m) => m.slice(-1).toUpperCase());

  contracts.push({
    name,
    dir: schemaFolder,
  });
}

(async () => {
  await genTS(contracts, tsFolder);
  await fixTs(tsFolder);
})();
