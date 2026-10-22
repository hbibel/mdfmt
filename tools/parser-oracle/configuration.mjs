import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const modulesIndex = process.argv.indexOf("--modules");
const modules = modulesIndex < 0 ? fileURLToPath(new URL("node_modules/", import.meta.url)) : path.resolve(process.argv[modulesIndex + 1]);
const require = createRequire(path.join(modules, "../package.json"));
const cliFile = path.join(modules, "markdownlint-cli2/markdownlint-cli2.mjs");
const lintFile = path.join(modules, "markdownlint/lib/markdownlint.mjs");
assert.equal(JSON.parse(readFileSync(path.join(modules, "markdownlint-cli2/package.json"))).version, "0.24.0");
assert.equal(JSON.parse(readFileSync(path.join(modules, "markdownlint/package.json"))).version, "0.42.0");
const lintSource = readFileSync(lintFile, "utf8");
const helpers = require(path.join(modules, "markdownlint/helpers/helpers.cjs"));
const { default: parseConfiguration } = await import(pathToFileURL(path.join(modules, "markdownlint/lib/parse-configuration.mjs")));
const { default: rules } = await import(pathToFileURL(path.join(modules, "markdownlint/lib/rules.mjs")));
const { default: parsers } = await import(pathToFileURL(path.join(modules, "markdownlint-cli2/parsers/parsers.mjs")));
function extract(name) {
  const start = lintSource.indexOf(`function ${name}(`);
  assert.ok(start >= 0);
  return lintSource.slice(start, lintSource.indexOf("\n}\n", start) + 3);
}
const enabledPerLine = new Function("helpers", "parseConfiguration", `${extract("getEffectiveConfig")}\n${extract("getEnabledRulesPerLineNumber")}\nreturn getEnabledRulesPerLineNumber;`)(helpers, parseConfiguration);
const aliases = {};
for (const rule of rules) {
  for (const name of [...rule.names, ...rule.tags]) {
    (aliases[name.toUpperCase()] ||= []).push(rule.names[0]);
  }
}

// Keep actual cli2 discovery, inheritance, overrides and filtering; observe only
// its lint-call boundary and disable writes in this development-only harness.
globalThis.__mdfmtConfigCalls = [];
let cliSource = readFileSync(cliFile, "utf8").replace("import { lint, extendConfig, readConfig }", "import { lint as originalLint, extendConfig, readConfig }");
function resolveSpecifier(specifier) {
  if (specifier.startsWith("node:")) return specifier;
  return specifier.startsWith(".") ? new URL(specifier, pathToFileURL(cliFile)).href : pathToFileURL(require.resolve(specifier)).href;
}
cliSource = cliSource.replace(/(from\s+|import\(\s*)(["'])([^"']+)\2/g, (_, prefix, quote, specifier) => `${prefix}${quote}${resolveSpecifier(specifier)}${quote}`);
cliSource += "\nconst lint = (options) => { globalThis.__mdfmtConfigCalls.push(options); return originalLint(options); };\n";
const { main } = await import(`data:text/javascript;base64,${Buffer.from(cliSource).toString("base64")}`);
const cases = JSON.parse(readFileSync(new URL("../../tests/fixtures/configuration/cases.json", import.meta.url)));
const observations = [];
for (const fixture of cases) {
  const root = mkdtempSync("/tmp/opencode/mdfmt-configuration-");
  try {
    for (const [name, text] of Object.entries(fixture.files)) {
      const file = path.join(root, name); mkdirSync(path.dirname(file), { recursive: true }); writeFileSync(file, text);
    }
    const argv = [...fixture.argv];
    if (fixture.explicit) argv.push("--config", fixture.explicit);
    if (fixture.pointer) argv.push("--configPointer", fixture.pointer);
    globalThis.__mdfmtConfigCalls = [];
    await main({ directory: path.join(root, fixture.invocation), argv, optionsOverride: { fix: false, outputFormatters: [] } });
    const calls = globalThis.__mdfmtConfigCalls;
    const files = calls.flatMap(call => call.files).map(file => path.relative(root, file)).sort();
    const probes = fixture.probes.map(probe => {
      const call = calls.find(call => call.files.includes(path.join(root, probe.file)));
      assert.ok(call, `${fixture.id}: ${probe.file}`);
      const text = fixture.files[probe.file].replace(/^\uFEFF/, "");
      const state = enabledPerLine(rules, text.split(/\r\n|\r|\n/), Boolean(call.noInlineConfig), call.config || {}, parsers, aliases);
      return { ...probe, enabled: state.enabledRulesPerLineNumber[probe.line].get(probe.rule), severity: state.rulesSeverity.get(probe.rule), options: state.effectiveConfig[probe.rule] };
    });
    observations.push({ id: fixture.id, files, probes });
  } finally { rmSync(root, { recursive: true, force: true }); }
}
const output = new URL("../../tests/fixtures/configuration/observations.json", import.meta.url);
if (process.argv.includes("--record")) writeFileSync(output, JSON.stringify(observations, null, 2) + "\n");
else assert.deepEqual(observations, JSON.parse(readFileSync(output)));
console.log(`PASS: ${observations.length} pinned upstream configuration/selection observations`);
