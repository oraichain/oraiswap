import codegen from "@cosmwasm/ts-codegen";
import { exec } from "child_process";
import { join, basename, resolve as _resolve } from "path";
import * as fs from "fs";
import {
  TypescriptParser,
  ClassDeclaration,
  InterfaceDeclaration,
  File,
} from "typescript-parser";

const {
  existsSync,
  promises: { readdir, readFile, writeFile, rm, mkdir },
} = fs;

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

const fixNested = async (
  clientName: string,
  ext: string,
  nestedResponses: { [key: string]: [string, string, string[]] },
  outPath: string
) => {
  const clientFile = join(outPath, `${clientName}.${ext}`);
  let clientData = (await readFile(clientFile)).toString();
  Object.entries(nestedResponses).forEach(([key, [name, inputType]]) => {
    clientData = clientData
      .replace(
        `${name}: () => Promise<${key}>;`,
        `${name}: (input: ${inputType}) => Promise<${key}>;`
      )
      .replace(
        `${name} = async (): Promise<${key}> => {`,
        `${name} = async (input:${inputType}): Promise<${key}> => {`
      )
      .replace(`${name}: {}`, `${name}: input`);
  });
  await writeFile(clientFile, clientData);
};

const fixNestedReactQuery = async (
  clientName: string,
  ext: string,
  nestedResponses: { [key: string]: [string, string, string[]] },
  outPath: string
) => {
  const clientFile = join(outPath, `${clientName}.${ext}`);
  let clientData = (await readFile(clientFile)).toString();
  Object.entries(nestedResponses).forEach(([key, [name, inputType]]) => {
    clientData = clientData
      .replace(
        `export interface Oraiswap${inputType}<TData> extends ${clientName}ReactQuery<${key}, TData> {}`,
        `export interface Oraiswap${inputType}<TData> extends ${clientName}ReactQuery<${key}, TData> {input: ${inputType}}`
      )
      .replace(
        `\n}: Oraiswap${inputType}<TData>) {`,
        `,\n\tinput\n}: Oraiswap${inputType}<TData>) {`
      )
      .replace(`client.${name}()`, `client.${name}(input)`);
  });
  await writeFile(clientFile, clientData);
};

const fixImport = async (
  clientName: string,
  ext: string,
  typeData: { [key: string]: string },
  nestedTypes: string[],
  outPath: string
) => {
  const clientFile = join(outPath, `${clientName}.${ext}`);
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
          )}} from "./types";\nimport {${[
            ...clientImportData,
            ...nestedTypes,
          ].join(", ")}} from "./${clientName}.types";`;
        }
      )
  );
};

const fixTs = async (outPath, enabledReactQuery = false) => {
  const parser = new TypescriptParser();
  const typeExt = ".types.ts";
  const typeData: { [key: string]: string } = {};
  const parsedData: { [key: string]: File } = {};
  const dirs = (await readdir(outPath)).filter((dir) => dir.endsWith(typeExt));

  await Promise.all(
    dirs.map(async (dir) => {
      const tsFile = join(outPath, dir);
      const tsData = (await readFile(tsFile)).toString();
      const parsed = await parser.parseSource(tsData);
      parsedData[dir] = parsed;

      for (let token of parsed.declarations) {
        if (!isPrivateType(token.name) && !typeData[token.name]) {
          typeData[token.name] = tsData.substring(token.start ?? 0, token.end);
        }
      }
    })
  );

  await Promise.all(
    dirs.map(async (dir) => {
      const tsFile = join(outPath, dir);
      const tsData = (await readFile(tsFile)).toString();
      const parsed = parsedData[dir];
      const modifiedTsData: string[] = [];
      const importData: string[] = [];

      for (let token of parsed.declarations) {
        if (typeData[token.name]) {
          importData.push(token.name);
        } else {
          modifiedTsData.push(tsData.substring(token.start ?? 0, token.end));
        }
      }

      // fix nested schema
      const contractName = basename(dir, typeExt);
      const nestedResponses = nestedMap[contractName];
      const nestedTypes: string[] = [];
      if (nestedResponses) {
        Object.entries(nestedResponses).forEach(([key, value]) => {
          nestedTypes.push(key);
          modifiedTsData.push(`export type ${key} = ${value[2].join(" | ")};`);
        });
      }

      // import from types, and remove from client
      modifiedTsData.unshift(
        `import {${importData.join(", ")}} from "./types";`
      );

      await writeFile(tsFile, modifiedTsData.join("\n"));

      // update client file

      const clientName = basename(dir, typeExt);
      await fixImport(clientName, "client.ts", typeData, nestedTypes, outPath);

      if (nestedResponses) {
        await fixNested(clientName, "client.ts", nestedResponses, outPath);
      }

      if (enabledReactQuery) {
        await fixImport(
          clientName,
          "react-query.ts",
          typeData,
          nestedTypes,
          outPath
        );
        if (nestedResponses) {
          await fixNestedReactQuery(
            clientName,
            "react-query.ts",
            nestedResponses,
            outPath
          );
        }
      }
    })
  );

  await writeFile(
    join(outPath, "types.ts"),
    Object.values(typeData).join("\n")
  );

  // add export from types
  const indexData = (await readFile(join(outPath, "index.ts"))).toString();
  if (indexData.indexOf('export * from "./types";') === -1) {
    await writeFile(
      join(outPath, "index.ts"),
      `${indexData}\nexport * from "./types";`
    );
  }
};

const buildSchema = async (packagePath: string) => {
  const artifactsFolder = join(packagePath, "artifacts");
  if (!existsSync(artifactsFolder)) await mkdir(artifactsFolder);

  const ret = await new Promise((resolve, reject) =>
    exec(
      `cargo run -q --bin schema`,
      {
        cwd: artifactsFolder,
      },
      (err, stdout, stderr) => {
        if (err) return reject(err);
        resolve(stderr || stdout);
      }
    )
  );
  // print err or out
  console.log(ret);
};

const fixNestedSchema = async (packagePath: string, update: boolean) => {
  const schemaFile = join(
    packagePath,
    "artifacts",
    "schema",
    basename(packagePath).replace(/_/g, "-") + ".json"
  );

  const schemaJSON = JSON.parse((await readFile(schemaFile)).toString());
  if (!schemaJSON.query.anyOf) return;
  const responses = {};
  schemaJSON.query.anyOf = schemaJSON.query.anyOf.map((item: any) => {
    const ref = update ? item.$ref : item.properties[item.required[0]].$ref;
    const matched = ref.match(/([A-Z][a-z]+)Query$/)[1];
    const name = matched.toLowerCase();
    const input = ref.split("/").pop();
    const subResponses = schemaJSON.query.definitions[input].oneOf.map(
      (item: any) => schemaJSON.responses[item.required[0]].title
    );

    responses[`${matched}Response`] = [name, input, subResponses];

    return update
      ? {
          type: "object",
          required: [name],
          properties: {
            [name]: item,
          },
          additionalProperties: false,
        }
      : item;
  });
  if (update) {
    await writeFile(schemaFile, JSON.stringify(schemaJSON, null, 2));
  }
  return responses;
};

const forceInd = process.argv.indexOf("--force");
const force = forceInd !== -1;
const enabledReactQuery = process.argv.includes("--react-query");
const contractsFolder = _resolve(__dirname, "contracts");
const tsFolder = _resolve(__dirname, "build");
const nestedMap: {
  [key: string]: { [key: string]: [string, string, string[]] };
} = {};

(async () => {
  const packages = (await readdir(contractsFolder))
    .map((dir) => _resolve(contractsFolder, dir))
    .filter((packagePath) => existsSync(join(packagePath, "Cargo.toml")));

  if (force) {
    // run custom packages or all
    let forcePackagesStr = process.argv[forceInd + 1];
    const forcePackages =
      forcePackagesStr && !forcePackagesStr.startsWith("--")
        ? forcePackagesStr.split(/\s*,\s*/)
        : packages;

    // can not run cargo in parallel
    for (const packagePath of forcePackages) {
      await buildSchema(packagePath);
    }
  }

  const contracts = await Promise.all(
    packages.map(async (packagePath) => {
      const baseName = basename(packagePath);
      // try fix nested schema if has
      const responses = await fixNestedSchema(packagePath, force);
      if (responses) {
        nestedMap[
          packagePath
            .split("/")
            .pop()!
            .replace(/(^\w|_\w)/g, (m, g1) => g1.slice(-1).toUpperCase())
        ] = responses;
      }

      return {
        name: baseName.replace(/^.|_./g, (m) => m.slice(-1).toUpperCase()),
        dir: join(packagePath, "artifacts", "schema"),
      };
    })
  );
  await genTS(contracts, tsFolder, enabledReactQuery);
  await fixTs(tsFolder, enabledReactQuery);
})();
