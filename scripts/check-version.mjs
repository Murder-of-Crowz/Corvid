import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const root = new URL("../", import.meta.url);
const desktop = new URL("desktop/", root);
const readJson = (name, base) =>
  JSON.parse(readFileSync(new URL(name, base), "utf8"));

const manifest = readJson("package.json", desktop);
const lock = readJson("package-lock.json", desktop);
const toolchain = readFileSync(new URL("rust-toolchain.toml", root), "utf8");
const [, rustVersion, rustHost] = toolchain.match(
  /^channel = "([0-9.]+)-([^"]+)"$/m,
);

function check(label, current, expected) {
  const matches = current === expected;
  console.log(
    `${matches ? "OK" : "FAIL"}  ${label}: ${current ?? "unavailable"} (expected ${expected})`,
  );
  if (!matches) process.exitCode = 1;
}

function rustTool(command, args) {
  const result = spawnSync(command, args, { cwd: desktop, encoding: "utf8" });
  if (result.status === 0) return result.stdout;
  console.error(result.error?.message ?? result.stderr?.trim() ?? `${command} failed`);
  return "";
}

check("Node.js", process.versions.node, manifest.engines.node);
check("npm", process.env.npm_config_user_agent?.match(/^npm\/([^ ]+)/)?.[1], manifest.engines.npm);

const rustc = rustTool("rustc", ["--version", "--verbose"]);
check("rustc", rustc.match(/^release: (.+)$/m)?.[1], rustVersion);
check("Rust host", rustc.match(/^host: (.+)$/m)?.[1], rustHost);
check("Cargo", rustTool("cargo", ["--version"]).match(/^cargo ([^ ]+)/)?.[1], rustVersion);

for (const [name, expected] of Object.entries({
  ...manifest.dependencies,
  ...manifest.devDependencies,
})) {
  const packagePath = `node_modules/${name}`;
  const locked = lock.packages[packagePath]?.version;
  if (locked !== expected) check(`${name} lockfile`, locked, expected);

  let installed;
  try {
    installed = readJson(`${packagePath}/package.json`, desktop).version;
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  check(name, installed, expected);
}
