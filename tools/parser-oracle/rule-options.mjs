import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const index = process.argv.indexOf("--modules");
const modules = index < 0 ? fileURLToPath(new URL("node_modules/", import.meta.url)) : path.resolve(process.argv[index + 1]);
assert.equal(JSON.parse(readFileSync(path.join(modules, "markdownlint/package.json"))).version, "0.42.0");
const schema = JSON.parse(readFileSync(path.join(modules, "markdownlint/schema/markdownlint-config-schema.json")));
function constraints(value) {
  const result = {};
  for (const [key, constraint] of Object.entries(value)) {
    if (["type", "enum", "minimum", "maximum", "pattern", "minItems", "maxItems"].includes(key)) result[key] = constraint;
    else if (key === "items") result[key] = constraints(constraint);
  }
  return result;
}
const entries = Object.entries(schema.properties).filter(([key]) => /^MD\d+$/.test(key)).map(([id, rule]) => [id, Object.fromEntries(Object.entries(rule.oneOf.find(option => option.type === "object").properties).map(([key, value]) => [key, constraints(value)]))]);
const text = "{\n" + entries.map(([id, options]) => `  ${JSON.stringify(id)}: ${JSON.stringify(options)}`).join(",\n") + "\n}\n";
const output = new URL("../../src/config/option-schema.json", import.meta.url);
if (process.argv.includes("--record")) writeFileSync(output, text);
else assert.equal(text, readFileSync(output, "utf8"));
console.log(`PASS: ${entries.length} pinned rule option schemas`);
