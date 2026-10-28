import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
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
console.log(`PASS: ${observations.length} pinned MD040 observations (native implementation pending)`);
