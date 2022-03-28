const { compile, compileFromFile } = require("json-schema-to-typescript");
const { execSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const package = process.argv[2];
const noCompile = process.argv.includes("--no-compile");
if (package) {
  const artifactsFolder = path.join(package, "artifacts");
  const schemaFolder = path.join(artifactsFolder, "schema");
  if (!noCompile || !fs.existsSync(schemaFolder)) {
    const ret = execSync(`cargo run -q --example schema`, {
      cwd: package,
    }).toString();
    console.log(ret);
  }

  const typesFolder = path.join(artifactsFolder, "types");
  if (!fs.existsSync(typesFolder)) {
    fs.mkdirSync(typesFolder);
  }
  const dirs = fs.readdirSync(schemaFolder);
  for (const file of dirs) {
    if (file.endsWith("json")) {
      // compile from file
      compileFromFile(path.join(schemaFolder, file)).then((ts) => {
        fs.writeFileSync(
          path.join(typesFolder, file.replace(/json$/, "d.ts")),
          ts
        );
      });
    }
  }
}
