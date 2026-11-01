import assert from "node:assert/strict";
import {readFileSync,writeFileSync} from "node:fs";
import {lint} from "markdownlint/sync";
import {applyFixes} from "markdownlint";
const samples=[
  "```sh\n$ echo hi\n$ pwd\n```\n","```sh\n$ echo hi\nhi\n```\n","```\n$ echo hi\n\n$ pwd\n```\n",
  "```\n$ echo hi\n   \n$ pwd\n```\n","```\n\n$ echo hi\n\n```\n","```\n```\n","```\n$\n```\n",
  "```\n$\techo hi\n```\n","```\n  $  echo hi\n```\n","```\n$\u00a0echo hi\n```\n","```\n$\u0085echo hi\n```\n",
  "    $ echo hi\n    $ pwd\n","    $ echo hi\n\n    $ pwd\n","    $ echo hi\n    output\n",
  "> ```sh\n> $ echo hi\n> $ pwd\n> ```\n","- ```sh\n  $ echo hi\n  $ pwd\n  ```\n",
  "1. ```sh\n   $ echo hi\n   $ pwd\n   ```\n",">     $ echo hi\n>     $ pwd\n",
  "  ```\n  $ echo hi\n    $ pwd\n  ```\n","```\n$ echo hi\n","```\n$ echo hi",
  "~~~\n$ echo hi\n~~~\n","$$\n$ echo hi\n$$\n","`$ echo hi`\n","$ echo hi\n",
  "\uFEFF```sh\r\n$ echo 😀\r\n```\r\n","```\r$ echo hi\r```\r",
  "<!-- markdownlint-disable-next-line MD014 -->\n```\n$ echo hi\n```\n",
  "```\n$ one\n$ two\n```\n<!-- markdownlint-disable-file commands-show-output -->\n",
  "```\n$ echo a very long command whose context is truncated\n```\n",
];
const configurations=[true,false,{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD014:options}}).input;
  const bom=input.startsWith("\uFEFF")?"\uFEFF":"";
  return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,column:f.errorRange[0],length:f.errorRange[1],context:f.errorContext,severity:f.severity})),
    output:bom+applyFixes(input.slice(bom.length),findings)};
}));
const receipt=new URL("md014-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
console.log(`PASS: ${observations.length} pinned MD014 observations (native implementation pending)`);
