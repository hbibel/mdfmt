import assert from "node:assert/strict";
import {readFileSync,writeFileSync} from "node:fs";
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
];
const configurations=[true,false,{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD056:options}}).input;
  return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,column:f.errorRange[0],length:f.errorRange[1],detail:f.errorDetail,severity:f.severity}))};
}));
const receipt=new URL("md056-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
console.log(`PASS: ${observations.length} pinned MD056 observations (native implementation pending)`);
