import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { lint } from "markdownlint/sync";

const samples = [
  "```js\ncode\n```\n", "~~~js\ncode\n~~~\n", "```\na\n```\n\n~~~\nb\n~~~\n",
  "~~~\na\n~~~\n\n```\nb\n```\n", "````js\ncode\n`````\n", "~~~~js\ncode\n~~~~~\n",
  "~~~\n```\n~~~\n", "```\n~~~\n```\n", "~~~\nalpha ``` beta\n~~~\n",
  "~~~js `meta`\ncode\n~~~\n", "~~~js\ncode", "```js\ncode", "   ~~~js\n   code\n   ~~~\n",
  "> ~~~js\n> code\n> ~~~\n", "- ~~~js\n  code\n  ~~~\n", "> - ~~~js\n>   code\n>   ~~~\n",
  "    ~~~js\n    code\n    ~~~\n", "\uFEFF~~~js\r\ncode\r\n~~~\r\n", "~~~js\rcode\r~~~\r",
  "~~~ js extra \ncode\n~~~  \n", "~~~js\tmeta\ncode\n~~~\n", "| ~~~ |\n| --- |\n",
  "<!-- markdownlint-disable-next-line MD048 -->\n~~~\na\n~~~\n\n```\nb\n```\n",
  "<!-- markdownlint-disable-file code-fence-style -->\n~~~\ncode\n~~~\n",
  "~~~\ncode\n<!-- markdownlint-disable MD048 -->\n~~~\n",
  ">\t~~~js\n>\tcode\n>\t~~~\n", "-\t~~~js\n\tcode\n\t~~~\n",
  "---\ntitle: x\n---\n~~~\ncode\n~~~\n", "~~~\ncode\n~~~~~~\n",
];
const configurations = [true, false, {style:"backtick"}, {style:"tilde"}, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => ({
  id: `${sample}-${configuration}`, input, options,
  findings: lint({ strings: { input }, config: {default:false,MD048:options} }).input.map(finding => ({
    line:finding.lineNumber, detail:finding.errorDetail, severity:finding.severity,
  })),
})));
const receipt = new URL("md048-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
console.log(`PASS: ${observations.length} pinned MD048 observations (native implementation pending)`);
