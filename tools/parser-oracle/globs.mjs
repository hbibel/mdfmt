import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const index = process.argv.indexOf("--modules");
const modules = index < 0 ? fileURLToPath(new URL("node_modules/", import.meta.url)) : path.resolve(process.argv[index + 1]);
const require = createRequire(path.join(modules, "../package.json"));
assert.equal(require(path.join(modules, "markdownlint-cli2/package.json")).version, "0.24.0");
const micromatch = require("micromatch");
const fixtures = JSON.parse(readFileSync(new URL("../../tests/fixtures/configuration/glob-cases.json", import.meta.url)));
const observations = fixtures.patterns.map(pattern => {
  const expanded = micromatch.braces(pattern, { expand: true, nodupes: true, keepEscaping: true });
  const regexes = expanded.map(p => micromatch.makeRe(p, { dot: true, posix: true }));
  return {
    pattern,
    expanded,
    enumeration: fixtures.paths.filter(p => regexes.some(regex => regex.test(p))),
    filtering: micromatch(fixtures.paths, [pattern], { dot: true })
  };
});
const output = new URL("../../tests/fixtures/configuration/glob-observations.json", import.meta.url);
if (process.argv.includes("--record")) writeFileSync(output, JSON.stringify(observations, null, 2) + "\n");
else assert.deepEqual(observations, JSON.parse(readFileSync(output)));
console.log(`PASS: ${observations.length} pinned enumeration/filter glob observations`);
