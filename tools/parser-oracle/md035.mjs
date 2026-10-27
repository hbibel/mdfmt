import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { lint } from "markdownlint/sync";

const samples = [
  "***\n\n---\n", "---\n\n***\n", "***\n\n* * *\n", "  ***\n\n---\n", "***  \n\n---\n",
  "> ***\n>\n> ---\n", "- ***\n\n  ---\n", "alpha\n---\n\n***\n", "---\ntitle: Hi\n---\n\n***\n\n---\n",
  "```\n***\n---\n```\n", "| *** |\n| --- |\n| --- |\n", "\uFEFF***\r\n\r\n---\r\n",
  "<!-- markdownlint-disable-next-line MD035 -->\n***\n\n---\n", "<!-- markdownlint-disable-file MD035 -->\n***\n\n---\n",
  "*** <!-- markdownlint-disable-line MD035 -->\n\n---\n", "___\n\n- - -\n\n* * *\n", "***\n\n\t---\n",
];
const configurations = [true, false, {style:"---"}, {style:"* * *"}, {style:""}, {style:"   "},
  {style:"# heading"}, {style:"\u00a0---\u00a0"}, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => ({
  id: `${sample}-${configuration}`, input, options,
  findings: lint({ strings: { input }, config: {default:false,MD035:options} }).input.map(finding => ({
    line: finding.lineNumber, detail: finding.errorDetail, severity: finding.severity,
  })),
})));
const receipt = new URL("md035-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
console.log(`PASS: ${observations.length} pinned MD035 diagnostic observations (native implementation pending)`);
