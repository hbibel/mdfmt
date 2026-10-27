import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  "# Hello!", "## Hello.,;:!。，；：！", "# Hello?", "# Hello! !", "# ! ###", "# !!!",
  "# Hello!  ###  ", "  # Hello!", "###\tHello!\t###\t", "# Hello\\!", "# Hello\\!!",
  "# Hello &amp;", "# Hello &unknown;", "# Hello &x;", "# Hello &#123;", "# Hello &#xabc;",
  "# Hello &amp;!", "# Hello :smile:", "# Hello :smile:!", "# Hello :+1:", "# Hello :invalid9:",
  "# Hello :clock1234:", "# Hello :foo_bar:", "# Hello :clock١:", "# Hello &#١;",
  "# Hello\u00a0!", "# Hello!\u00a0", "# Hello!\u000b", "# 😀Hello !\r\n", "\uFEFF# Hello!\r\n",
  "Hello!\n---\n", "Hello! \n---\n", "Hello!\t\n---\n", "First\nsecond!\n---\n",
  "> # Hello! ###", "- # Hello!", "> First\n> second!\n> ---\n", "- First\n  second!\n  ---\n",
  "# *Hello!*", "# `Hello!`", "# [Hello!](url)", "# Hello! <!-- comment -->",
  "Hello!", "```md\n# Hello!\n```", "    # Hello!", "| # Hello! |\n| --- |\n| data |",
  "---\ntitle: Hello!\n---\n\n# Hello!\n", "<!-- markdownlint-disable-next-line MD026 -->\n# Hello!",
  "<!-- markdownlint-disable-file headings -->\n# Hello!", "# Hello!\n\n# World!\n",
  "#foo!\n---\n", "#foo\n---\n", "#foo\n===\n", "# Hi! ###!", "# Hi! # !",
];
const configurations = [true, false, { punctuation: "" }, { punctuation: "?" }, { punctuation: ".,;:" },
  { severity: "warning" }, { punctuation: "!*]\\-" }, { punctuation: "o" }];

const observe = (input, options) => lint({ strings: { input }, config: { default: false, MD026: options } }).input;
function settle(input, options) {
  for (let count = 0; count < 100; count++) {
    // Lint columns exclude the BOM; applyFixes expects that same stripped input.
    const bom = input.startsWith("\uFEFF") ? "\uFEFF" : "";
    const next = bom + applyFixes(input.slice(bom.length), observe(input, options));
    if (next === input) return next;
    input = next;
  }
  throw new Error("upstream did not settle");
}
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => ({
  id: `${sample}-${configuration}`, input, options,
  findings: observe(input, options).map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0],
    length: finding.errorRange[1], detail: finding.errorDetail, severity: finding.severity })),
  output: settle(input, options),
})));
const crashInput = "First\n!\n---\n";
assert.throws(() => observe(crashInput, true), /Value of 'range'/);
const receipt = new URL("md026-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));

const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
const directory = mkdtempSync("/tmp/opencode/mdfmt-md026-oracle-");
const configuration = directory + "/.markdownlint-cli2.jsonc";
try { for (const row of observations) {
  for (const fix of [false, true]) {
    writeFileSync(configuration, JSON.stringify({ fix, config: { default: false, MD026: row.options } }));
    const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
    assert.equal(native.stdout, fix ? row.output : row.input, `${row.id}: fix ${fix}: ${native.stderr}`);
    const findings = [...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD026: Trailing punctuation in heading \[(.*)\]$/gm)]
      .map(match => ({ line: Number(match[1]), column: Number(match[2]), detail: match[4], severity: match[3].toLowerCase() }));
    const expected = (fix ? observe(row.output, row.options) : observe(row.input, row.options)).map(finding => ({
      line: finding.lineNumber, column: finding.errorRange[0], detail: finding.errorDetail, severity: finding.severity }));
    assert.deepEqual(findings, expected, `${row.id}: ${JSON.stringify(row.input)} ${JSON.stringify(row.options)}`);
    assert.equal(native.status, expected.some(f => f.severity === "error") ? 1 : 0, native.stderr);
  }
} } finally { unlinkSync(configuration); rmdirSync(directory); }
console.log(`PASS: ${observations.length} pinned MD026 observations, native findings and converged fixes`);
