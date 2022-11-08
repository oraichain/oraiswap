const codegen = require("@cosmwasm/ts-codegen").default;
const { exec } = require("child_process");
const path = require("path");
const fs = require("fs");
const util = require("util");
const { TypescriptParser } = require("typescript-parser");
const execAsync = util.promisify(exec);

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

const buildSchema = async (package) => {
  const artifactsFolder = path.join(package, "artifacts");
  if (!fs.existsSync(artifactsFolder)) fs.mkdir(artifactsFolder);

  const ret = await execAsync(`cargo run -q --bin schema`, {
    cwd: artifactsFolder,
  });
  // print err or out
  console.log(ret.stderr || ret.stdout);
};

const force = process.argv.includes("--force") || process.argv.includes("-f");
const contractsFolder = path.resolve(__dirname, "contracts");
const tsFolder = path.resolve(__dirname, "build");

const packages = fs
  .readdirSync(contractsFolder)
  .map((dir) => path.resolve(contractsFolder, dir))
  .filter((package) => fs.existsSync(path.join(package, "Cargo.toml")));

(async () => {
  if (force) {
    await Promise.all(packages.map(buildSchema));
  }

  const contracts = packages.map((package) => {
    const baseName = path.basename(package);
    return {
      name: baseName.replace(/^.|_./g, (m) => m.slice(-1).toUpperCase()),
      dir: path.join(package, "artifacts", "schema"),
    };
  });
  await genTS(contracts, tsFolder);
  await fixTs(tsFolder);
})();
