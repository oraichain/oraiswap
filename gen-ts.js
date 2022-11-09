const codegen = require("@cosmwasm/ts-codegen").default;
const { exec } = require("child_process");
const path = require("path");
const {
  existsSync,
  promises: { readdir, readFile, writeFile, rm, mkdir },
} = require("fs");
const util = require("util");
const { TypescriptParser } = require("typescript-parser");
const execAsync = util.promisify(exec);

const genTS = async (contracts, outPath, enabledReactQuery = false) => {
  await rm(outPath, { recursive: true, force: true });
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
        enabled: enabledReactQuery,
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

const fixImport = async (clientName, ext, typeData, outPath) => {
  // react-query.ts
  const clientFile = path.join(outPath, `${clientName}.${ext}`);
  const clientData = await readFile(clientFile);

  await writeFile(
    clientFile,
    clientData
      .toString()
      .replace(
        new RegExp(
          `import\\s+\\{(.*?)\\}\\s+from\\s+"\\.\\/${clientName}\\.types";`
        ),
        (_, g1) => {
          const [clientImportData, typesImportData] = g1
            .trim()
            .split(/\s*,\s*/)
            .reduce(
              (ret, el) => {
                ret[!typeData[el] ? 0 : 1].push(el);
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
};

const fixTs = async (outPath, enabledReactQuery = false) => {
  const parser = new TypescriptParser();
  const typeExt = ".types.ts";
  const typeData = {};
  const dirs = (await readdir(outPath)).filter((dir) => dir.endsWith(typeExt));

  await Promise.all(
    dirs.map(async (dir) => {
      const tsFile = path.join(outPath, dir);
      const tsData = (await readFile(tsFile)).toString();
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
      await writeFile(tsFile, modifiedTsData.join("\n"));

      // update client file
      const clientName = path.basename(dir, typeExt);
      await fixImport(clientName, "client.ts", typeData, outPath);
      if (enabledReactQuery) {
        await fixImport(clientName, "react-query.ts", typeData, outPath);
      }
    })
  );

  await writeFile(
    path.join(outPath, "types.ts"),
    Object.values(typeData).join("\n")
  );

  // add export from types
  const indexData = (await readFile(path.join(outPath, "index.ts"))).toString();
  if (indexData.indexOf('export * from "./types";') === -1) {
    await writeFile(
      path.join(outPath, "index.ts"),
      `${indexData}\nexport * from "./types";`
    );
  }
};

const buildSchema = async (package) => {
  const artifactsFolder = path.join(package, "artifacts");
  if (!existsSync(artifactsFolder)) await mkdir(artifactsFolder);

  const ret = await execAsync(`cargo run -q --bin schema`, {
    cwd: artifactsFolder,
  });
  // print err or out
  console.log(ret.stderr || ret.stdout);
};

const force = process.argv.includes("--force") || process.argv.includes("-f");
const enabledReactQuery = process.argv.includes("--react-query");
const contractsFolder = path.resolve(__dirname, "contracts");
const tsFolder = path.resolve(__dirname, "build");

(async () => {
  const packages = (await readdir(contractsFolder))
    .map((dir) => path.resolve(contractsFolder, dir))
    .filter((package) => existsSync(path.join(package, "Cargo.toml")));

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
  await genTS(contracts, tsFolder, enabledReactQuery);
  await fixTs(tsFolder, enabledReactQuery);
})();
