import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const profile = process.argv.includes("--release") ? "release" : "debug";
const binary = fileURLToPath(new URL(`../../target/${profile}/mdfmt`, import.meta.url));
for (const paragraphs of [100, 500, 1000]) {
  const prefix = '<!-- markdownlint-configure-file {"default":false,"MD013":{"line_length":10}} --> <!-- markdownlint-disable-line MD013 -->\n';
  const input = prefix +
    "alpha beta gamma delta epsilon zeta eta theta\n\n".repeat(paragraphs);
  const start = performance.now();
  const result = spawnSync(binary, ["-"], { input, encoding: "utf8", timeout: 10000 });
  assert.equal(result.error, undefined, `${paragraphs} paragraphs exceeded the 10-second smoke-test budget`);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, prefix +
    "alpha beta\ngamma\ndelta\nepsilon\nzeta eta\ntheta\n\n".repeat(paragraphs));
  console.log(`${paragraphs} paragraphs: ${(performance.now() - start).toFixed(0)} ms`);
}
