import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  ">  text\n", ">   text\n", ">    text\n", ">     code\n", "> text\n", " >  text\n", ">  >  text\n",
  ">  \n", ">   \n", ">\t text\n", "> \ttext\n", ">\t\ttext\n", ">\u00a0 text\n",
  ">  - item\n", "> - item\n>    continuation\n", "> 1. item\n>     continuation\n",
  "- >  item\n", "- > text\n  >   more\n", "> - >  text\n", ">  - >  text\n",
  ">  # Heading\n", ">  Heading\n> ---\n", ">  ```js\n>   code\n>  ```\n",
  "> ```\n>   code\n> ```\n", ">  ```\n>     code\n>  ```\n", ">    code\n>    more\n",
  ">     code\n>      more\n", "```\n>  literal\n```\n", "    >  literal\n",
  "\uFEFF>  😀 text\r\n", ">  one\r>  two\r", "> one\n\n>  two\n",
  "<!-- markdownlint-disable-next-line MD027 -->\n>  text\n", "<!-- markdownlint-disable-file blockquote -->\n>  text\n",
  ">  | a |\n> | - |\n> | b |\n", ">  text with a rather long context that will be truncated\n",
];
const configurations = [true, false, {list_items:false}, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const findings=lint({strings:{input},config:{default:false,MD027:options}}).input;
  const bom=input.startsWith("\uFEFF")?"\uFEFF":"";
  return {id:`${sample}-${configuration}`,input,options,
    findings:findings.map(finding=>({line:finding.lineNumber,column:finding.errorRange[0],length:finding.errorRange[1],
      context:finding.errorContext,severity:finding.severity})),
    output:bom+applyFixes(input.slice(bom.length),findings)};
}));
const receipt = new URL("md027-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
console.log(`PASS: ${observations.length} pinned MD027 observations (native implementation pending)`);
