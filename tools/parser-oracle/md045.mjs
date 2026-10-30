import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";

const samples = [
  "![](image)\n", "![alt](image)\n", "![ ](image)\n", "![]()\n", "![](image \"title\")\n",
  "![][id]\n\n[id]: image\n", "![][missing]\n", "[![](image)](url)\n", "`![](image)`\n",
  "```md\n![](image)\n```\n", "| ![](image) |\n| --- |\n", "> ![](image)\n", "- ![](image)\n",
  "![](\nimage\n)\n", "![\n](image)\n", "\uFEFF😀 ![](image)\r\n",
  "<img src=\"image\">\n", "<IMG SRC='image' ALT='alt'>\n", "<img src=image alt=\"\">\n",
  "<img src=image alt>\n", "<img src=image aria-hidden=true>\n", "<img src=image aria-hidden='TRUE'>\n",
  "<img src=image aria-hidden=false>\n", "<img src=image data-alt=\"alt\">\n", "<img src=image alttext=\"alt\">\n",
  "<img\n src=image\n>\n", "alpha <img\n src=image> beta\n", "<img src=image />\n", "</img>\n",
  "<!-- <img src=image> -->\n", "<script>const s = '<img src=image>';</script>\n",
  "<style>/* <img src=image> */</style>\n", "<div><img src=image></div>\n", "<div>\n<img src=image>\n</div>\n",
  "> <img src=image>\n", "- <img src=image>\n", "<img src=image> and ![](image)\n",
  "<!-- markdownlint-disable-next-line MD045 -->\n![](image)\n", "<!-- markdownlint-disable-file accessibility -->\n<img src=image>\n",
  "<img src=image aria-hidden=\" true\">\n", "<img src=image ALT = \"alt\">\n", "<img src=image\u00a0alt=\"alt\">\n",
  "![![](inner)](outer)\n", "![<img src=image>](outer)\n", "![`<img src=image>`](outer)\n",
  "<img src=x data-note=\" alt=ignored\">\n", "<img src=x aria-hidden=\"TrUe\">\n",
  "<img src=x aria-hidden=\"true extra\">\n", "<img src=x aria-hidden=true/>\n",
  "<![CDATA[<img src=x>]]>\n", "<?xml text='<img src=x>'?>\n", "<textarea><img src=x></textarea>\n",
  "<pre>`<img src=x>`</pre>\n", "<img src=x>\n![](markdown)\n", "alpha <img src=x> ![](markdown)\n",
  "![![](inner)][outer]\n\n[outer]: image\n", "> ![![](inner)](outer)\n", "![![alt](inner)](outer)\n",
  "prefix <img src=x\n alt>\n", "<img src=x\u0085alt=\"alt\">\n", "<img src=x\u2003alt=\"alt\">\n",
  "![<img title=\"]\">](out)\n", "![<img title=\"[\">](out)\n",
  "<div>\n![<img src=x>](out)\n</div>\n", "<script>![<img src=x>](out)</script>\n",
  "> ![![](\n> inner\n> )](outer)\n", "> <div><img\n> src=x></div>\n",
  "> ![<img\n> src=x>](outer)\n", "> ![![][id]](outer)\n>\n> [id]:\n>  image\n",
];
const configurations = [true, false, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => ({
  id: `${sample}-${configuration}`, input, options,
  findings: lint({ strings: { input }, config: {default:false,MD045:options} }).input.map(finding => ({
    line:finding.lineNumber, range:finding.errorRange, severity:finding.severity,
  })),
})));
const receipt = new URL("md045-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
if (process.argv.includes("--source-only")) {
  console.log(`PASS: ${observations.length} pinned MD045 observations (native implementation pending)`);
} else {
  const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
  const directory = mkdtempSync("/tmp/opencode/mdfmt-md045-oracle-");
  const configuration = directory + "/.markdownlint-cli2.jsonc";
  try { for (const row of observations) {
    for (const fix of [false, true]) {
      writeFileSync(configuration, JSON.stringify({ fix, config: {default:false,MD045:row.options} }));
      const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
      assert.equal(native.stdout, row.input, row.id);
      const expected = row.findings.map(finding => ({line:finding.line,column:finding.range?.[0] || 1,severity:finding.severity}))
        .sort((left,right) => left.line-right.line || left.column-right.column);
      const actual = [...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD045: Images should have alternate text \(alt text\)$/gm)]
        .map(match => ({line:Number(match[1]),column:Number(match[2]),severity:match[3].toLowerCase()}));
      assert.deepEqual(actual, expected, `${row.id}: fix ${fix}: ${native.stderr}`);
      assert.equal(native.status, expected.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
    }
  } } finally { unlinkSync(configuration); rmdirSync(directory); }
  console.log(`PASS: ${observations.length} pinned MD045/native diagnostic and source-preservation cases`);
}
