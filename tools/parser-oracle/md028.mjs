import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
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
  "> one\n> <!-- markdownlint-disable-next-line MD028 -->\n\n> two\n",
  "> > one\n>\t\n> > two\n", "-\t> one\n\n\t> two\n", "> - > one\n>  \n>   > two\n",
  "1. > one\n\n   > two\n", "- one\n\n  > a\n\n  > b\n", "- > a\n\n  text\n\n  > b\n",
  "> a\n\n---\n\n> b\n", "> a\n\n# Heading\n\n> b\n", "> > a\n> \u00a0\n> > b\n",
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
if (process.argv.includes("--source-only")) {
  console.log(`PASS: ${observations.length} pinned MD028 observations (native implementation pending)`);
} else {
  const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
  const directory = mkdtempSync("/tmp/opencode/mdfmt-md028-oracle-");
  const configuration = directory + "/.markdownlint-cli2.jsonc";
  try { for (const row of observations) {
    for (const fix of [false, true]) {
      writeFileSync(configuration, JSON.stringify({ fix, config: {default:false,MD028:row.options} }));
      const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
      assert.equal(native.stdout, row.input, row.id);
      const actual = [...native.stderr.matchAll(/^-:(\d+):1: (Error|Warning) MD028: Blank line inside blockquote$/gm)]
        .map(match => ({line:Number(match[1]),severity:match[2].toLowerCase()}));
      assert.deepEqual(actual, row.findings, `${row.id}: fix ${fix}: ${native.stderr}`);
      assert.equal(native.status, row.findings.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
    }
  } } finally { unlinkSync(configuration); rmdirSync(directory); }
  console.log(`PASS: ${observations.length} pinned MD028/native diagnostic and source-preservation cases`);
}
