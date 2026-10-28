import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { lint } from "markdownlint/sync";

const samples = [
  "[label]()\n", "[label](#)\n", "[label](url)\n", "[](url)\n", "[]()\n", "[label](<>)\n",
  "[label](<#>)\n", "[label]( # )\n", "[label](##)\n", "[label](#target)\n", "[label](&#35;)\n",
  "[label](\\#)\n", "[label](\"title\")\n", "[label](# \"title\")\n", "![alt]()\n",
  "[label][id]\n\n[id]: #\n", "[label][ID]\n\n[id]: #\n", "[label][]\n\n[label]: #\n",
  "[label]\n\n[label]: #\n", "[label][id]\n\n[id]: url\n", "[label][missing]\n",
  "[label][id]\n\n[id]: <#>\n", "[label][id]\n\n[id]: &#35;\n", "[label][id]\n\n[id]: \\#\n",
  "[label][id]\n\n[id]: #\n[id]: url\n", "[label][id]\n\n[id]: url\n[id]: #\n",
  "[a\nb]()\n", "[a](\n#\n)\n", "> [label]()\n", "- [label]()\n", "`[label]()`\n",
  "```md\n[label]()\n```\n", "| [label]() |\n| --- |\n", "\uFEFF😀 [label]()\r\n",
  "<!-- markdownlint-disable-next-line MD042 -->\n[label]()\n", "<!-- markdownlint-disable-file links -->\n[label]()\n",
  "[a very long label with many words in it]()\n", "[outer [inner](url)]()\n", "[label](#) and [next]()\n",
];
const configurations = [true, false, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const row = {id: `${sample}-${configuration}`, input, options};
  try { return {...row, findings: lint({ strings: { input }, config: {default:false,MD042:options} }).input.map(finding => ({
    line:finding.lineNumber, column:finding.errorRange[0], length:finding.errorRange[1],
    context:finding.errorContext, severity:finding.severity,
  }))}; } catch (error) { return {...row, error:error.message}; }
}));
const receipt = new URL("md042-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
console.log(`PASS: ${observations.length} pinned MD042 observations (native implementation pending)`);
