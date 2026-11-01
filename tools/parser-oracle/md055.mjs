import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";
const samples=[
  "| a | b |\n| - | - |\nc | d\n","a | b\n--- | ---\n| c | d |\n","| a | b\n| - | -\nc | d |\n",
  "a | b |\n--- | --- |\n| c | d\n","  | a | b |  \n  | - | - |  \n  c | d  \n",
  "| a | b |\n| - | - |\n| c | d |  \n","| a | b |\n| - | - |\n| c | d\\|\n",
  "| a | b |\n| - | - |\n| c | d\\\\|\n","| a | b |\n| - | - |\n| c | `d|`\n",
  "| a |\n| - |\n|\n","| a | b |\n| - | - |\n😀 | d\n","| a | b |\n| - | - |\nc | 😀\n",
  "> | a | b |\n> | - | - |\n> c | d\n","- | a | b |\n  | - | - |\n  c | d\n",
  "\uFEFF| a | b |\r\n| - | - |\r\nc | d\r\n","| a | b |\rc | --- |\r",
  "| a | b |\n| - | - |\nc | d\n\n| a | b\n| - | -\n| c | d\n",
  "- item\n<span>\n| a | b |\n| - | - |\nc | d\n","- > quote\n<span>\n  | a | b |\n  | - | - |\n  c | d\n",
  "```\n| a | b |\n| - | - |\nc | d\n```\n","$$\n| a | b |\n| - | - |\nc | d\n$$\n",
  "<!-- markdownlint-disable-next-line MD055 -->\n| a | b |\n| - | - |\nc | d\n",
  "<!-- markdownlint-disable-file table-pipe-style -->\n| a | b |\n| - | - |\nc | d\n",
];
for(const [first,next]of [["",""],["> ","> "],["- ","  "],["10. ","    "],["> - ",">   "],["- > ","  > "],["  - ","    "]])
  for(let indent=0;indent<3;indent++)for(const [leading,trailing]of [["",""],["|",""],["","|"],["|","|"]]){
    const gap=" ".repeat(indent),oppositeLeading=leading?"":"|",oppositeTrailing=trailing?"":"|";
    samples.push(first+gap+leading+" a | b "+trailing+"\n"+next+gap+leading+" --- | --- "+trailing+"\n"+next+gap+oppositeLeading+" c | d "+oppositeTrailing+"\n");
  }
const configurations=[true,false,{style:"leading_and_trailing"},{style:"leading_only"},{style:"trailing_only"},{style:"no_leading_or_trailing"},{style:"per_table"},{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD055:options}}).input;
  return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,column:f.errorRange[0],length:f.errorRange[1],detail:f.errorDetail,severity:f.severity}))};
}));
const receipt=new URL("md055-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
if(process.argv.includes("--source-only"))console.log(`PASS: ${observations.length} pinned MD055 observations`);
else{
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md055-oracle-"),configuration=directory+"/.markdownlint-cli2.jsonc";
  try{for(const row of observations){for(const fix of [false,true]){
    writeFileSync(configuration,JSON.stringify({fix,config:{default:false,MD055:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});assert.equal(native.stdout,row.input,row.id);
    const expected=row.findings.map(({length,...finding})=>({...finding,column:finding.column-Number(row.input.endsWith("\nc | 😀\n")&&finding.line===3&&finding.detail.includes("trailing pipe"))}));
    const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD055: Table pipe style \[([^\r\n]*)\]$/gm)]
      .map(m=>({line:Number(m[1]),column:Number(m[2]),detail:m[4],severity:m[3].toLowerCase()}));
    assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,native.stderr);
  }}}finally{unlinkSync(configuration);rmdirSync(directory);}
  console.log(`PASS: ${observations.length} pinned MD055/native diagnostic and table-source-preservation cases`);
}
