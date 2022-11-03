const codegen = require("@cosmwasm/ts-codegen").default;
const { execSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const genTS = async (name, dir, outPath) => {
  await codegen({
    contracts: [
      {
        name,
        dir,
      },
    ],
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

const package = process.argv[2];
if (package) {
  const artifactsFolder = path.join(package, "artifacts");
  const schemaFolder = path.join(artifactsFolder, "schema");

  if (
    !fs.existsSync(schemaFolder) ||
    process.argv.includes("--force") ||
    process.argv.includes("-f")
  ) {
    const ret = execSync(`cargo run -q --bin schema`, {
      cwd: artifactsFolder,
    }).toString();
    console.log(ret);
  }
  const tsFolder = path.join(artifactsFolder, "ts");
  const baseName = path.basename(package);
  const name = baseName.replace(/^.|_./g, (m) => m.slice(-1).toUpperCase());
  fs.rmSync(tsFolder, { recursive: true, force: true });
  genTS(name, schemaFolder, tsFolder);
}
