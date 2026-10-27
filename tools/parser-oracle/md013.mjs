import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";

const samples = [
  "one two three", "verylongtoken", "> verylongtoken", "- verylongtoken", "# verylongtoken",
  "alpha beta  ", "ab\u00a0verylongtoken", "ab\u000bverylongtoken", "ab\tverylongtoken", ">\u00a0verylongtoken",
  "a😀b", "éé alpha beta", "\uFEFFalpha beta gamma", "\uFEFF# alpha beta gamma",
  "---\ntitle: alpha beta gamma\n---\n\nalpha beta gamma\n", "+++\ntitle = 'alpha beta gamma'\n...\nalpha beta gamma",
  "---\r\ntitle: alpha beta gamma\r\n---\r\nalpha beta gamma\r\n",
  "# alpha beta gamma\n", "setext alpha beta\n----------------\n", "```text\nalpha beta gamma\n```\n", "    alpha beta gamma\n",
  "| long cell |\n| --------- |\n| data data |\n", "[id]: https://example.com/long\n    \"a long title\"\n",
  "[long label](https://example.com/long)", "![long label](https://example.com/long)", "[long label](url) [long label](url)",
  "> [long label](https://example.com/long)", "- [long label](https://example.com/long)", "[long label](url)\n[long label](url)\n",
  "[long label][id]\n\n[id]: https://example.com/long\n", "`long code value`", "[label](url)`long code value`",
  "&amp;[label](https://example.com/long)", "&unknown;[label](https://example.com/long)", "\\![label](https://example.com/long)",
  "&amp;[long label](https://example.com/long)", "&unknown;[long label](https://example.com/long)", "\\![long label](https://example.com/long)",
  "[label](https://example.com/long)  \n", "[label](url) .", "$$ alpha beta gamma $$", "::note[alpha beta gamma]",
  "<!-- markdownlint-disable-next-line MD013 -->\nalpha beta gamma\n\nalpha beta gamma\n",
  "<!-- markdownlint-disable-file MD013 -->\nalpha beta gamma\n",
];
const configurations = [
  { line_length: 10 }, { line_length: 10, strict: true }, { line_length: 10, stern: true },
  { line_length: 10, strict: true, headings: false, code_blocks: false, tables: false },
  { line_length: 10, strict: true, heading_line_length: 30, code_block_line_length: 30, severity: "warning" },
  { line_length: 2, strict: true }
];

function column(text, line, units) {
  const original = text.replace(/\r\n?/g, "\n").split("\n")[line - 1].replace(/^\uFEFF/u, "");
  let offset = 0;
  for (const ch of original) {
    if (offset + ch.length >= units) return offset + 1;
    offset += ch.length;
  }
  return offset + 1;
}

const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const findings = lint({ strings: { input }, config: { default: false, MD013: options } }).input;
  return { id: `${sample}-${configuration}`, input, options, findings: findings.map(finding => ({
    line: finding.lineNumber,
    column: column(input, finding.lineNumber, finding.errorRange[0]),
    detail: finding.errorDetail,
    severity: finding.severity
  })) };
}));
const receipt = new URL("md013-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));

const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
const directory = mkdtempSync("/tmp/opencode/mdfmt-md013-oracle-");
const configuration = directory + "/.markdownlint-cli2.jsonc";
try { for (const row of observations) {
  writeFileSync(configuration, JSON.stringify({ fix: false, config: { default: false, MD013: row.options } }));
  const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
  assert.equal(native.stdout, row.input, `${row.id}: fix false`);
  const findings = [...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD013: Line length \[([^\]]+)\]/gm)]
    .map(match => ({ line: Number(match[1]), column: Number(match[2]), detail: match[4], severity: match[3].toLowerCase() }));
  assert.deepEqual(findings, row.findings, `${row.id}: ${JSON.stringify(row.input)} ${JSON.stringify(row.options)}`);
  assert.equal(native.status, row.findings.some(f => f.severity === "error") ? 1 : 0, native.stderr);
} } finally { unlinkSync(configuration); rmdirSync(directory); }
console.log(`PASS: ${observations.length} native/pinned upstream MD013 diagnostic comparisons`);
