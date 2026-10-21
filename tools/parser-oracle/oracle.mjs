import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { lint } from "markdownlint/sync";

const cases = JSON.parse(readFileSync(new URL("../../tests/fixtures/parser/cases.json", import.meta.url)));
const observations = [];
for (const fixture of cases) {
  let captured;
  lint({ strings: { fixture: fixture.input }, noInlineConfig: true, config: { default: false, capture: true }, customRules: [{
    names: ["capture"], description: "Capture upstream parser regions", tags: ["observation"], parser: "micromark",
    function(params) {
      function flatten(tokens) {
        return tokens.flatMap(token => [{ type: token.type, text: token.text,
          start: [token.startLine, token.startColumn], end: [token.endLine, token.endColumn] }, ...flatten(token.children)]);
      }
      captured = { id: fixture.id, frontMatterLines: params.frontMatterLines,
        tokens: flatten(params.parsers.micromark.tokens) };
    }
  }] });
  assert.ok(captured, fixture.id);
  observations.push(captured);
}
if (process.argv.includes("--record")) {
  writeFileSync(new URL("observations.json", import.meta.url), JSON.stringify(observations, null, 2) + "\n");
} else {
  assert.deepEqual(observations, JSON.parse(readFileSync(new URL("observations.json", import.meta.url))));
}
console.log(`PASS: ${observations.length} pinned upstream parser observations`);
