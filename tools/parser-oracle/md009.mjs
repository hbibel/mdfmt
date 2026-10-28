import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  "alpha ", "alpha  ", "alpha   ", "alpha \n", "alpha  \n", "alpha   \nbeta\n", "alpha  \nbeta\n",
  "alpha\t\n", "alpha\u00a0\n", "alpha\uFEFF\n", "alpha\u2028\n", "\uFEFFalpha \r\n", "alpha \rbeta \r",
  "# heading \n", "# heading  \n", "# heading  ### \n", "heading \n--- \n", "*** \n", "***  \n",
  "```md \ncode \n``` \n", "```\ncode \n", "    code \n", "```\ncode  \n```\n",
  "| a | \n| - | \n| b | \n", "`alpha  \nbeta`\n", "alpha `code  \nmore` tail\n",
  "- item \n", "- item\n  \n  continuation \n", "- first\n  \n- second \n", "- item\n  \n",
  "> alpha \n> beta\n", "---\ntitle: Hi \n---\n\nalpha \n",
  "<!-- markdownlint-disable-next-line MD009 -->\nalpha \n", "<!-- markdownlint-disable-file whitespace -->\nalpha \n",
  "<!-- markdownlint-disable-next-line MD009 -->\nalpha beta gamma ",
];
const configurations = [true, false, {br_spaces:0}, {br_spaces:1}, {br_spaces:3}, {strict:true},
  {code_blocks:true}, {list_item_empty_lines:true}, {strict:true,list_item_empty_lines:true}, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const findings = lint({ strings: { input }, config: {default:false,MD009:options} }).input;
  const bom = input.startsWith("\uFEFF") ? "\uFEFF" : "";
  return { id: `${sample}-${configuration}`, input, options,
    findings: findings.map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0],
      length: finding.errorRange[1], detail: finding.errorDetail, severity: finding.severity })),
    output: bom + applyFixes(input.slice(bom.length), findings) };
}));
const receipt = new URL("md009-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
console.log(`PASS: ${observations.length} pinned MD009 observations (native implementation pending)`);
