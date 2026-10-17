import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";

const version = JSON.parse(fs.readFileSync("package.json", "utf8")).version;
const directory = process.argv[2] ?? "artifacts";
const base = `https://github.com/sywyyhykkk/subbit/releases/download/v${version}`;
const files = {
  "darwin-aarch64": `Subbit_${version}_macos-arm64.app.tar.gz`,
  "darwin-x86_64": `Subbit_${version}_macos-x86_64.app.tar.gz`,
  "windows-x86_64": `Subbit_${version}_windows-x64-setup.exe`,
};
const platforms = Object.fromEntries(Object.entries(files).map(([target, file]) => {
  assert(fs.statSync(path.join(directory, file)).size > 0, `${target} update bundle is empty`);
  const signature = fs.readFileSync(path.join(directory, `${file}.sig`), "utf8").trim();
  assert(signature.length > 50, `${target} update signature is missing`);
  return [target, { url: `${base}/${file}`, signature }];
}));
fs.writeFileSync(path.join(directory, "latest.json"), JSON.stringify({
  version,
  notes: fs.readFileSync(`docs/releases/${version}.md`, "utf8"),
  pub_date: new Date().toISOString(),
  platforms,
}, null, 2) + "\n");
console.log(`Created updater manifest for ${version}: ${Object.keys(platforms).join(", ")}`);
