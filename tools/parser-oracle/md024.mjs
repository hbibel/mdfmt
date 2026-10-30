import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { lint } from "markdownlint/sync";

const samples = [
  "# Same\n# Same\n", "# Same\n## Same\n", "# Same\n# same\n", "# A  B\n# A B\n",
  "# Parent A\n## Child\n# Parent B\n## Child\n", "# A\n## Child\n### Leaf\n## Child\n",
  "### Child\n## Child\n### Child\n", "# A\n#### Child\n## Other\n#### Child\n",
  "> ## Same\n\n## Same\n", "- ## Same\n\n## Same\n", "# Same\n\nSame\n====\n",
  "A\nB\n===\n\n# A B\n", "> A\n> B\n> ===\n\n# A B\n",
  "# <b>Same</b>\n# Same\n", "# *<b>Same</b>*\n# *Same*\n", "# `Same`\n# Same\n",
  "# [Same](one)\n# [Same](two)\n", "# A &amp; B\n# A & B\n", "# A\\*B\n# A*B\n",
  "# Same #\n# Same\n", "#\n##\n", "\uFEFF# 😀 Same\r\n# 😀 Same\r\n",
  "# This heading has a rather long duplicate context\n# This heading has a rather long duplicate context\n",
  "<!-- markdownlint-disable-next-line MD024 -->\n# Same\n# Same\n",
  "# Same\n<!-- markdownlint-disable-next-line MD024 -->\n# Same\n# Same\n",
  "<!-- markdownlint-disable-file headings -->\n# Same\n# Same\n", "```\n# Same\n```\n# Same\n",
  "---\ntitle: Same\n---\n# Same\n# Same\n", " > 1.\t---\n >  \t# Same\n# Same\n",
  "# ![alt](image)\n# ![alt](image)\n", "# A <!-- comment --> B\n# A  B\n",
];
const configurations = [true, false, {siblings_only:true}, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => ({
  id: `${sample}-${configuration}`, input, options,
  findings: lint({ strings: { input }, config: {default:false,MD024:options} }).input.map(finding => ({
    line:finding.lineNumber, context:finding.errorContext, severity:finding.severity,
  })),
})));
const receipt = new URL("md024-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
console.log(`PASS: ${observations.length} pinned MD024 observations (native implementation pending)`);
