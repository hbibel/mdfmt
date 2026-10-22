import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const mm = require("micromatch");
assert.equal(require("micromatch/package.json").version, "4.0.8");
let seed = 0x6d64666d;
const pick = values => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return values[seed % values.length]; };
const atoms = ["a", "b", "ab", "é", "🦀", "a*", "?", "[ab]", "[!a]", "[^a]", "[[:digit:]]", "{a,b}", "{1..3}", "a..", "\\*", "\\?", "\\[a\\]", "a\\b", '"a*"'];
function body(depth) {
  if (depth === 0 || pick([0, 1, 2]) === 0) return pick(atoms);
  return `${pick(["@", "?", "+", "*", "!"])}(${body(depth - 1)}|${body(depth - 1)})`;
}
const patterns = new Set(["*.*", "**/*.*", "\\@(a|b).md", "!!(a).md", "!(a|!(b)).md", "!(*.test).md", "{é..ê}.md", "[🦀-🦞].md", "a.md\\", "a|b", "a{,b}", "{x,@(a,b)}.md", "[]a].md", "[[:bad:]].md", "[[:alpha:]--a].md", "+(+(a(a)+)a)a.md", "((aa+)+a)+a.md", "\\uD83E\\uDD80.md", "[\\uD83E\\uDD80][\\uD83E\\uDD80].md", '@(a"b").md']);
for (let i = 0; i < 768; i++) patterns.add(`${pick(["", "x", "*/", "**/", "{a,b}/"])}${body(2)}${pick(["", ".md", "?.md", "*.md", "/x.md", "**", ".{md,txt}"])}`);
const paths = ["", ".", "..", "a", "aa", "ab", "b", "a.", "a.md", "b.md", "ab.md", ".md", "a/b.md", "a/x.md", "a/b/x.md", "x.md", "foo.test.md", "foo.md", "!a.md", "é.md", "ê.md", "🦀.md", "🦀🦀.md", "*.md", "?.md", "[a].md", "a\n.md", "\na.md", "a.md\n", "a.md\r\n", "a.md\\", "1.md", "2.md", "a|b", "@a.md", "@b.md", "aaaa.md", 'a"b".md'];
const cases = [...patterns].map(pattern => {
  const expanded = mm.braces(pattern, { expand: true, nodupes: true, keepEscaping: true });
  const regexes = expanded.filter(Boolean).map(p => mm.makeRe(p, { dot: true, posix: true }));
  return { pattern, expanded, enumeration: paths.filter(path => regexes.some(regex => regex.test(path))), filtering: mm(paths, [pattern], { dot: true }) };
});
const output = { seed: "0x6d64666d", paths, cases };
const file = new URL("../../tests/fixtures/configuration/generated-globs.json", import.meta.url);
if (process.argv.includes("--record")) writeFileSync(file, JSON.stringify(output, null, 2) + "\n");
else assert.deepEqual(output, JSON.parse(readFileSync(file)));
console.log(`PASS: ${cases.length} generated pinned glob cases over ${paths.length} paths`);
