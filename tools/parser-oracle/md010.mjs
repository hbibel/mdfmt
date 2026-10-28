import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  "alpha\tbeta\n", "alpha\t\tbeta\n", "alpha \t beta\n", "alpha\t\t\n", "alpha\t\nbeta\n", "\talpha\n",
  "`code\tvalue`\n", "`code`\toutside\n", "`code` \toutside\n", "alpha `code\n\tmore` end\n",
  "```js\n\tcode\n```\n", "```JS\ncode\tvalue\n```\n", "```js extra\n\tcode\n```\n",
  "```make\n\tcode\n```\n", "```\n\tcode\n```\n", "```j&#115;\n\tcode\n```\n", "```js\t\ncode\n```\t\n",
  "```js\n\tcode", "    code\tvalue\n", "| a\tb |\n| --- |\n| data |\n", "[label\tvalue](url)\n",
  "-\titem\n  - child\n", "> \talpha\n", "\uFEFF😀\talpha\r\n", "alpha\tbeta\rnext\tline\r",
  "---\ntitle:\tHi\n---\n\nalpha\tbeta\n", "<!-- markdownlint-disable-next-line MD010 -->\nalpha\tbeta\n",
  "<!-- markdownlint-disable-file hard_tab -->\nalpha\tbeta\n", "- * *\t\n",
];
const configurations = [true, false, {spaces_per_tab:0}, {spaces_per_tab:2}, {spaces_per_tab:4},
  {code_blocks:false}, {ignore_code_languages:["js"]}, {code_blocks:false,ignore_code_languages:["make"]},
  {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const findings = lint({ strings: { input }, config: {default:false,MD010:options} }).input;
  const bom = input.startsWith("\uFEFF") ? "\uFEFF" : "";
  return { id: `${sample}-${configuration}`, input, options,
    findings: findings.map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0],
      length: finding.errorRange[1], detail: finding.errorDetail, severity: finding.severity })),
    output: bom + applyFixes(input.slice(bom.length), findings) };
}));
const receipt = new URL("md010-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
console.log(`PASS: ${observations.length} pinned MD010 observations (native implementation pending)`);
