import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { lint } from "markdownlint/sync";

const samples = [
  "> one\n\n> two\n", "> one\n\n\n> two\n", "> one\n \n> two\n", "> one\n\t\n> two\n",
  "> one\n>\n> two\n", "> one\n\ntext\n\n> two\n", "> one\n\n<!-- comment -->\n\n> two\n",
  "> one\n\n", "\n> one\n", "> one\n\u00a0\n> two\n", "> one\n\u2003\n> two\n",
  "> > one\n>\n> > two\n", "> > one\n> \n> > two\n", "> > one\n\n> > two\n",
  "- > one\n\n  > two\n", "- > one\n\n- > two\n", "- > one\n  \n  > two\n",
  "> - > one\n>\n>   > two\n", "> - > one\n>\n> - > two\n", " > one\n\n  > two\n",
  "> ```\n> one\n\n> two\n", "```\n> one\n\n> two\n```\n", "    > one\n\n    > two\n",
  "\uFEFF> one\r\n\r\n> two\r\n", "> one\r\r> two\r", ">\n\n>\n",
  "<!-- markdownlint-disable-next-line MD028 -->\n\n> one\n\n> two\n",
  "> one\n<!-- markdownlint-disable-next-line MD028 -->\n\n> two\n",
  "<!-- markdownlint-disable-file blockquote -->\n> one\n\n> two\n",
  "> one\n\n> two\n\n> three\n", "> > > one\n> >\n> > > two\n",
];
const configurations = [true, false, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => ({
  id: `${sample}-${configuration}`, input, options,
  findings: lint({ strings: { input }, config: {default:false,MD028:options} }).input.map(finding => ({
    line:finding.lineNumber, severity:finding.severity,
  })),
})));
const receipt = new URL("md028-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
console.log(`PASS: ${observations.length} pinned MD028 observations (native implementation pending)`);
