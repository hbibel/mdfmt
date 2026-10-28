import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";

const samples = [
  "```\ncode\n```\n", "```js\ncode\n```\n", "```JS\ncode\n```\n", "~~~rust\ncode\n~~~\n",
  "```js extra\ncode\n```\n", "``` js extra \ncode\n```\n", "```js\tmeta\ncode\n```\n",
  "``` \ncode\n```\n", "```\t\ncode\n```\n", "```j&#115;\ncode\n```\n", "```js\u00a0extra\ncode\n```\n",
  "```js\u2003extra\ncode\n```\n", "```js\u000bextra\ncode\n```\n", "```js\ncode", "```\ncode",
  "  ```\n  code\n  ```\n", "> ```\n> code\n> ```\n", "- ```\n  code\n  ```\n",
  "> - ```js extra\n>   code\n>   ```\n", "    ```\n    code\n    ```\n", "\t```\n\tcode\n\t```\n",
  "\uFEFF```\r\ncode\r\n```\r\n", "~~~\rcode\r~~~\r", "---\ntitle: x\n---\n```\ncode\n```\n",
  "<!-- markdownlint-disable-next-line MD040 -->\n```\ncode\n```\n",
  "<!-- markdownlint-disable-file language -->\n```\ncode\n```\n",
  "``````````````````````````````````````\ncode\n``````````````````````````````````````\n",
  "```JS extra\ncode\n```\n", "```js   \ncode\n```\n", "~~~js `meta`\ncode\n~~~\n",
  ">\t```\n>\tcode\n>\t```\n", "-\t```\n\tcode\n\t```\n",
  "  > -  ```js\tmeta \n  >    code\n  >    ```\n", "- item\n\n   ```\n   code\n   ```\n",
  "~~~js\u2028extra\ncode\n~~~\n", "~~~js\u000cextra\ncode\n~~~\n",
  "```js\\ extra\ncode\n```\n", "~~~j&#115; extra\ncode\n~~~\n",
];
const configurations = [true, false, {language_only:true}, {allowed_languages:["js","rust"]},
  {allowed_languages:["JS"]}, {language_only:true,allowed_languages:["js"]}, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => ({
  id: `${sample}-${configuration}`, input, options,
  findings: lint({ strings: { input }, config: {default:false,MD040:options} }).input.map(finding => ({
    line:finding.lineNumber, detail:finding.errorDetail, context:finding.errorContext, severity:finding.severity,
  })),
})));
const receipt = new URL("md040-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
if (process.argv.includes("--source-only")) {
  console.log(`PASS: ${observations.length} pinned MD040 observations (native implementation pending)`);
} else {
  const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
  const directory = mkdtempSync("/tmp/opencode/mdfmt-md040-oracle-");
  const configuration = directory + "/.markdownlint-cli2.jsonc";
  try { for (const row of observations) {
    for (const fix of [false, true]) {
      writeFileSync(configuration, JSON.stringify({ fix, config: {default:false,MD040:row.options} }));
      const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
      assert.equal(native.stdout, row.input, row.id);
      const expected = row.findings.map(finding => ({line:finding.line,severity:finding.severity,
        message: finding.context === null ? finding.detail : `Context: "${finding.context}"`}));
      const actual = [...native.stderr.matchAll(/^-:(\d+):1: (Error|Warning) MD040: Fenced code blocks should have a language specified \[([^\r\n]*)\]$/gm)]
        .map(match => ({ line: Number(match[1]), severity: match[2].toLowerCase(), message:match[3] }));
      assert.deepEqual(actual, expected, `${row.id}: fix ${fix}: ${native.stderr}`);
      assert.equal(native.status, expected.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
    }
  } } finally { unlinkSync(configuration); rmdirSync(directory); }
  console.log(`PASS: ${observations.length} pinned MD040/native diagnostic and source-preservation cases`);
}
