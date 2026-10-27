import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  "# Hello", "#  Hello", "#\tHello", "# \tHello", "#\t Hello", "#\t\tHello", "#  Hello!  ###",
  "# Hello ###", "#  Hello ###", "# Hello  ###", "#  Hello  ##  ", "  ##  Hello", "> #  Hello", "- #  Hello",
  "#", "# ", "#   ", "#  ##", "#  #  ", "# ##", "# ###  ###", "#foo\n---\n", "#foo!\n---\n",
  "#  Hello\\###", "#  Hello \\###", "#  Hello \\#", "#  Hello # text", "####### Hello",
  "\uFEFF#  😀Hello  ###\r\n", "#  Hello\u00a0###", "#\u00a0 Hello", "```\n#  Hello\n```", "    #  Hello",
  "<!-- markdownlint-disable-next-line MD019 -->\n#  Hello", "<!-- markdownlint-disable-next-line MD021 -->\n#  Hello  ###",
  "#  Hello <!-- markdownlint-disable-line MD019 -->", "<!-- markdownlint-disable-file headings -->\n#  Hello  ###",
  "#  <!-- markdownlint-disable-line MD026 -->",
];
const configurations = [
  { default: false, MD019: true, MD021: true }, { default: false, MD019: true },
  { default: false, MD021: true }, { default: false, MD019: { severity: "warning" }, MD021: { severity: "warning" } },
  { default: false },
];
const observations = samples.flatMap((input, sample) => configurations.map((config, configuration) => {
  const findings = lint({ strings: { input }, config }).input;
  const bom = input.startsWith("\uFEFF") ? "\uFEFF" : "";
  return { id: `${sample}-${configuration}`, input, config,
    findings: findings.map(finding => ({ rule: finding.ruleNames[0], line: finding.lineNumber,
      column: finding.errorRange[0], length: finding.errorRange[1], severity: finding.severity })),
    output: bom + applyFixes(input.slice(bom.length), findings) };
}));
const receipt = new URL("heading-spaces-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
const directory = mkdtempSync("/tmp/opencode/mdfmt-heading-spaces-oracle-");
const configuration = directory + "/.markdownlint-cli2.jsonc";
try { for (const row of observations) {
  for (const fix of [false, true]) {
    writeFileSync(configuration, JSON.stringify({ fix, config: row.config }));
    const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
    const expected = (fix ? [] : row.findings).map(({ length, ...finding }) => finding);
    const actual = [...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) (MD019|MD021): /gm)]
      .map(match => ({ rule: match[4], line: Number(match[1]), column: Number(match[2]), severity: match[3].toLowerCase() }));
    assert.equal(native.stdout, fix ? row.output : row.input, `${row.id}: fix ${fix}: ${native.stderr}`);
    assert.deepEqual(actual, expected, `${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status, expected.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
  }
} } finally { unlinkSync(configuration); rmdirSync(directory); }
console.log(`PASS: ${observations.length} pinned MD019/MD021 observations and native diagnostic/fix comparisons`);
