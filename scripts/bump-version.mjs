// Usage: npm run bump -- 0.2.0
// Updates the version in package.json, package-lock.json, src-tauri/tauri.conf.json
// and src-tauri/Cargo.toml (the updater compares against tauri.conf.json), then
// prints the commands to tag and publish the release.
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) {
  console.error("Usage: npm run bump -- <major.minor.patch>   e.g. npm run bump -- 0.2.0");
  process.exit(1);
}

function editJson(path, edit) {
  const raw = readFileSync(path, "utf8");
  const eol = raw.includes("\r\n") ? "\r\n" : "\n";
  const json = JSON.parse(raw);
  edit(json);
  writeFileSync(path, JSON.stringify(json, null, 2).replace(/\n/g, eol) + eol);
}

editJson("package.json", (j) => (j.version = version));
editJson("package-lock.json", (j) => {
  j.version = version;
  if (j.packages?.[""]) j.packages[""].version = version;
});
editJson("src-tauri/tauri.conf.json", (j) => (j.version = version));

const cargoPath = "src-tauri/Cargo.toml";
const cargo = readFileSync(cargoPath, "utf8");
const updated = cargo.replace(/^version = ".*"$/m, `version = "${version}"`);
if (updated === cargo && !cargo.includes(`version = "${version}"`)) {
  console.error("Could not find the package version in Cargo.toml");
  process.exit(1);
}
writeFileSync(cargoPath, updated);

console.log(`Version set to ${version}. Next:`);
console.log(`  cargo check --manifest-path src-tauri/Cargo.toml   # refreshes Cargo.lock`);
console.log(`  git commit -am "chore(release): v${version}"`);
console.log(`  git tag v${version} && git push origin develop v${version}`);
