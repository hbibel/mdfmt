import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
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
console.log(`PASS: ${observations.length} pinned MD023 observations (native implementation pending)`);
