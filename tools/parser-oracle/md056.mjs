import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";
const samples=[
  "| a | b |\n| - | - |\n| c |\n","| a | b |\n| - | - |\n| c | d | e |\n","a | b\n- | -\nc | d\n",
  "a | b\n- | -\nc\n","a | b\n- | -\nc | d | e\n","| a |\n| - |\n| b | c |\n","| a | b |\n| - | - |\n|\n",
  "| a | b |\n| - | - |\n|  |\n","| a | b |\n| - | - |\n| c |  d  |  e  |\n",
  "| a | b |\n| - | - |\n| c\\|d | e |\n","| a | b |\n| - | - |\n| `c|d` | e |\n","| a | b |\n| - | - |\n| `c\\|d` | e |\n",
  "| a | b |\n| - | - |\n| c\\\\|d | e |\n","| a | b |\n| - | - |\n| 😀 |\n","| a | b |\n| - | - |\n| c | d | 😀 |\n",
  "> | a | b |\n> | - | - |\n> | c |\n","- | a | b |\n  | - | - |\n  | c | d | e |\n",
  "\uFEFF| a | b |\r\n| - | - |\r\n| c |\r\n","| a | b |\r| - | - |\r| c |\r",
  "```\n| a | b |\n| - | - |\n| c |\n```\n","$$\n| a | b |\n| - | - |\n| c |\n$$\n",
  "- item\n<span>\n| a | b |\n| - | - |\n| c |\n","<div>\n| a | b |\n| - | - |\n| c |\n</div>\n",
  "| a | b |\n| - | - |\n| c |\n\n| a |\n| - |\n| b | c |\n",
  "<!-- markdownlint-disable-next-line MD056 -->\n| a | b |\n| - | - |\n| c |\n",
  "| a | b |\n| - | - |\n| c | <!-- markdownlint-disable-line MD056 -->\n",
  "<!-- markdownlint-disable-file table-column-count -->\n| a | b |\n| - | - |\n| c |\n",
  "| a | b |\n| - | - |\n|😀\n",
  "a | b\n--- | ---\nc\n","a | b\n--- | ---\nc | d | e\n",
  "- > quote\n<span>\n  | a | b |\n  | - | - |\n  | c |\n","- > quote\n<span>\n  owned\n| a | b |\n| - | - |\n| c |\n",
  "> quote\n> <span>\n  | x | y |\n  | - | - |\n  | z |\n\n| a | b |\n| - | - |\n| c |\n",
  "10. > quote\n<span>\n    > | a | b |\n    > | - | - |\n    > | c |\n",
  "- ```\n  code\n  ```\n<span>\n| a | b |\n| - | - |\n| c |\n","- # heading\n<span>\n| a | b |\n| - | - |\n| c |\n","-\n<span>\n| a | b |\n| - | - |\n| c |\n",
];
for(let expected=1;expected<=4;expected++)for(let actual=0;actual<=6;actual++)for(const [leading,trailing]of [["",""],["|",""],["","|"],["|","|"]]){
  const row=Array.from({length:actual},(_,index)=>index===1?" `c|d` ":" c\\|d ").join("|");
  samples.push("|"+Array(expected).fill(" h ").join("|")+"|\n|"+Array(expected).fill(" - ").join("|")+"|\n"+leading+row+trailing+"\n");
}
const configurations=[true,false,{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD056:options}}).input;
  return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,column:f.errorRange[0],length:f.errorRange[1],detail:f.errorDetail,severity:f.severity}))};
}));
const receipt=new URL("md056-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
if(process.argv.includes("--source-only"))console.log(`PASS: ${observations.length} pinned MD056 observations`);
else{
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md056-oracle-"),configuration=directory+"/.markdownlint-cli2.jsonc";
  try{for(const row of observations){for(const fix of [false,true]){
    writeFileSync(configuration,JSON.stringify({fix,config:{default:false,MD056:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});assert.equal(native.stdout,row.input,row.id);
    const expected=row.findings.map(({length,...finding})=>({...finding,column:finding.column-Number(row.input.endsWith("\n|😀\n"))}));
    const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD056: Table column count \[([^\r\n]*)\]$/gm)]
      .map(m=>({line:Number(m[1]),column:Number(m[2]),detail:m[4],severity:m[3].toLowerCase()}));
    assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,native.stderr);
  }}}finally{unlinkSync(configuration);rmdirSync(directory);}
  console.log(`PASS: ${observations.length} pinned MD056/native diagnostic and table-source-preservation cases`);
}
