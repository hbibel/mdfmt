import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
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
  "- item\n<span>\n```sh\n$ echo hi\n```\n","> text\n<span>\n```sh\n$ echo hi\n```\n",
];
for(const [first,next]of [["",""],["> ","> "],["- ","  "],["> - ",">   "]])
  for(let indent=0;indent<4;indent++)for(const gap of [""," ","   ","\t"," \t"])
    samples.push(first+" ".repeat(indent)+"```sh\n"+next+gap+"$ echo hi\n"+next+" ".repeat(indent)+"```\n");
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
if(process.argv.includes("--source-only"))console.log(`PASS: ${observations.length} pinned MD014 observations`);
else{
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md014-oracle-"),configuration=directory+"/.markdownlint-cli2.jsonc";
  let differences=0;
  try{for(const row of observations){for(const fix of [false,true]){
    writeFileSync(configuration,JSON.stringify({fix,config:{default:false,MD014:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});assert.equal(native.stdout,row.input,row.id);
    const expected=row.findings.map(({length,...finding})=>finding);
    const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD014: Dollar signs used before commands without showing output \[Context: "([^\r\n]*)"\]$/gm)]
      .map(m=>({line:Number(m[1]),column:Number(m[2]),context:m[4],severity:m[3].toLowerCase()}));
    assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,native.stderr);
    if(fix&&native.stdout!==row.output)differences++;
  }}}finally{unlinkSync(configuration);rmdirSync(directory);}
  console.log(`PASS: ${observations.length} pinned MD014/native diagnostics and source-preservation cases; ${differences} protected-code upstream-fix differences`);
}
