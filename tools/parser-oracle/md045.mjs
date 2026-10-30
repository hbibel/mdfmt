import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
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
console.log(`PASS: ${observations.length} pinned MD045 observations (native implementation pending)`);
