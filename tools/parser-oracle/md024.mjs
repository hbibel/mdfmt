import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
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
  "*A\nB*\n===\n\n# *A B*\n", "A\\\nB\n===\n\n# A\\ B\n", "A  \nB\n===\n\n# A   B\n",
  "# A\tB\n# A\tB\n", "# <span title=\"x\">Same</span>\n# Same\n",
  "# [<b>Same</b>](url)\n# [Same](url)\n", "# A\u00a0B\n# A B\n", "# A\u00a0B\n# A\u00a0B\n",
  "# Top\n<!-- markdownlint-disable-next-line MD024 -->\n## Child\n## Child\n",
  "# A\n## Same\n<!-- markdownlint-disable-next-line MD024 -->\n# B\n## Same\n",
  "# ~~<b>A</b>~~\n# ~~A~~\n", "# ~<b>A</b>~\n# ~A~\n", "# ~~<b>A</b>~~\n# ~~<b>A</b>~~\n",
  "> A  \n> B\n> ===\n\n# A   B\n", "> A  \n> B\n> ===\n\n# A   > B\n",
  "- A  \n  B\n  ===\n\n# A     B\n", "> A\\\n> B\n> ===\n\n# A\\ > B\n",
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
if (process.argv.includes("--source-only")) {
  console.log(`PASS: ${observations.length} pinned MD024 observations (native implementation pending)`);
} else {
  const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
  const directory = mkdtempSync("/tmp/opencode/mdfmt-md024-oracle-");
  const configuration = directory + "/.markdownlint-cli2.jsonc";
  try { for (const row of observations) {
    for (const fix of [false, true]) {
      writeFileSync(configuration, JSON.stringify({ fix, config: {default:false,MD024:row.options} }));
      const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
      assert.equal(native.stdout, row.input, row.id);
      const actual = [...native.stderr.matchAll(/^-:(\d+):1: (Error|Warning) MD024: Multiple headings with the same content(?: \[Context: "([^\r\n]*)"\])?$/gm)]
        .map(match => ({line:Number(match[1]),context:match[3] ?? null,severity:match[2].toLowerCase()}));
      assert.deepEqual(actual, row.findings, `${row.id}: fix ${fix}: ${native.stderr}`);
      assert.equal(native.status, row.findings.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
    }
  } } finally { unlinkSync(configuration); rmdirSync(directory); }
  console.log(`PASS: ${observations.length} pinned MD024/native diagnostic and source-preservation cases`);
}
