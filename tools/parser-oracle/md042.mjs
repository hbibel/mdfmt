import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
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
  "[label][id]\n\n[id]: #)\n", "[label][id]\n\n[id]: #(x)\n", "[label][ID]\n\n[ID]: #\n",
  "[a  b]\n\n[a  b]: #\n", "[a b]\n\n[a  b]: #\n", "[a\\]b]\n\n[a\\]b]: #\n",
  "[`a]b`]()\n", "[`a[b`]()\n", "[![alt](image)]()\n", "[a\\]b]()\n", "[a\\[b]()\n",
  "[a](<\u00a0#\u00a0>)\n", "[a](\u00a0#\u00a0)\n", "[a](#\\))\n",
  "> [a\n> b]()\n", "[a\r\nb]()\r\n", "---\ntitle: x\n---\n[label]()\n",
  " > 1.\t---\n >  \t[label]()\n", "😀\t[label]()\n",
  "> [a](\n> )\n", "> [a](\n> <>\n> )\n", "![[a]()](image)\n", "![[a](#)](image)\n",
  "![prefix [a]() suffix](image)\n", "![`[a]()`](image)\n", "![[a](url)](image)\n",
  "![[a][id]](image)\n\n[id]: #\n", "![[a][ID]](image)\n\n[id]: #\n",
  "![![inner [a]()](nested)](image)\n", "> ![[a]()](image)\n", "> > [a](\n> > <>\n> > )\n",
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
const sourceErrors = observations.filter(row => row.error).length;
if (process.argv.includes("--source-only")) {
  console.log(`PASS: ${observations.length} pinned MD042 records, including ${sourceErrors} upstream runtime errors (native implementation pending)`);
} else {
  const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
  const directory = mkdtempSync("/tmp/opencode/mdfmt-md042-oracle-");
  const configuration = directory + "/.markdownlint-cli2.jsonc";
  try { for (const row of observations) {
    for (const fix of [false, true]) {
      writeFileSync(configuration, JSON.stringify({ fix, config: {default:false,MD042:row.options} }));
      const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
      assert.equal(native.stdout, row.input, row.id);
      if (row.error) {
        assert.equal(native.status, row.options.severity === "warning" ? 0 : 1, native.stderr);
        assert.match(native.stderr, /MD042: No empty links \[Context: "/, row.id);
        assert.doesNotMatch(native.stderr, /Processing|panic|Invalid/, row.id);
      } else {
        const expected = row.findings.map(finding => ({line:finding.line,column:finding.column,
          severity:finding.severity,context:finding.context}));
        const actual = [...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD042: No empty links \[Context: "([^\r\n]*)"\]$/gm)]
          .map(match => ({line:Number(match[1]),column:Number(match[2]),severity:match[3].toLowerCase(),context:match[4]}));
        assert.deepEqual(actual, expected, `${row.id}: fix ${fix}: ${native.stderr}`);
        assert.equal(native.status, expected.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
      }
    }
  } } finally { unlinkSync(configuration); rmdirSync(directory); }
  console.log(`PASS: ${observations.length - sourceErrors} pinned MD042/native comparisons; ${sourceErrors} upstream runtime-error cases handled safely; source preserved`);
}
