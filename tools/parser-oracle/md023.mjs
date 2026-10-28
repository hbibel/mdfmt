import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  " # Heading\n", "  ## Heading\n", "   ### Heading\n", "    # Code\n", "\t# Code\n",
  "# Heading\n", " # Heading #\n", "  Heading\n---\n", "Heading\n  ---\n", "  Heading\n  ===\n",
  "> # Heading\n", ">  # Heading\n", ">   # Heading\n", " > # Heading\n", " >  # Heading\n",
  "> > # Heading\n", "> >  # Heading\n", "- # Heading\n", "-  # Heading\n", "-   # Heading\n",
  "- item\n\n   # Heading\n", "- item\n\n    # Heading\n", "1. # Heading\n", "1.  # Heading\n",
  "- >  # Heading\n", "> -  # Heading\n", ">  Heading\n> ---\n", "-  Heading\n   ---\n",
  "\uFEFF  # 😀 Heading\r\n", " # Heading\rnext\r", "```\n # Code\n```\n",
  "---\ntitle: x\n---\n # Heading\n", "<!-- markdownlint-disable-next-line MD023 -->\n # Heading\n",
  "<!-- markdownlint-disable-file heading-start-left -->\n # Heading\n",
  " # Heading!", " > 1.\t---\n >  \t# Code\n", "  # Heading\ntext\n",
  "   A\n   B\n---\n", ">   A\n>   B\n> ---\n",
  "   > > # Heading\n", "  >  > # Heading\n", "  - # Heading\n", "   -  # Heading\n",
  "- item\n\n  >  # Heading\n", "> - item\n>\n>    # Heading\n",
  " # This heading has a rather long context to truncate\n", " # 123456789012345678901234567😀end\n",
  ">\t # Heading\n", "-\t # Heading\n", "  text\nnot a heading\n", "# Heading\n  ---\n",
  "   >   A\n   > B\n   > ===\n", "- a\n\n  - b\n\n     # Nested\n",
  "-   \n    # H\n", "-\t\n   # H\n", "1. \t\n     # H\n", "> -   \n>     # H\n",
];
const configurations = [true, false, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const findings = lint({ strings: { input }, config: {default:false,MD023:options} }).input;
  const bom = input.startsWith("\uFEFF") ? "\uFEFF" : "";
  return { id: `${sample}-${configuration}`, input, options,
    findings: findings.map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0],
      length: finding.errorRange[1], context: finding.errorContext, severity: finding.severity })),
    output: bom + applyFixes(input.slice(bom.length), findings) };
}));
const receipt = new URL("md023-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
if (process.argv.includes("--source-only")) {
  console.log(`PASS: ${observations.length} pinned MD023 observations (native implementation pending)`);
} else {
  const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
  const directory = mkdtempSync("/tmp/opencode/mdfmt-md023-oracle-");
  const configuration = directory + "/.markdownlint-cli2.jsonc";
  let preservedDifferences = 0;
  try { for (const row of observations) {
    for (const fix of [false, true]) {
      writeFileSync(configuration, JSON.stringify({ fix, config: {default:false,MD023:row.options} }));
      const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
      if (!fix) assert.equal(native.stdout, row.input, row.id);
      const expected = lint({ strings: { input: native.stdout }, config: {default:false,MD023:row.options} }).input
        .map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0], context: finding.errorContext, severity: finding.severity }));
      const actual = [...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD023: Headings must start at the beginning of the line \[Context: "(.*)"\]$/gm)]
        .map(match => ({ line: Number(match[1]), column: Number(match[2]), context: match[4], severity: match[3].toLowerCase() }));
      assert.deepEqual(actual, expected, `${row.id}: fix ${fix}: ${native.stderr}`);
      assert.equal(native.status, expected.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
      if (fix) {
        if (native.stdout !== row.output) preservedDifferences++;
        const second = spawnSync(binary, ["-"], { input: native.stdout, encoding: "utf8", cwd: directory });
        assert.equal(second.stdout, native.stdout, `${row.id}: idempotence`);
        assert.equal(second.status, native.status, `${row.id}: stable status`);
      }
    }
  } } finally { unlinkSync(configuration); rmdirSync(directory); }
  console.log(`PASS: ${observations.length} pinned MD023/native diagnostic and idempotence cases; ${preservedDifferences} preserved upstream-fix differences`);
}
