import assert from "node:assert/strict";
import {readFileSync,writeFileSync} from "node:fs";
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
const configurations=[true,false,{style:"leading_and_trailing"},{style:"leading_only"},{style:"trailing_only"},{style:"no_leading_or_trailing"},{style:"per_table"},{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD055:options}}).input;
  return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,column:f.errorRange[0],length:f.errorRange[1],detail:f.errorDetail,severity:f.severity}))};
}));
const receipt=new URL("md055-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
console.log(`PASS: ${observations.length} pinned MD055 observations (native implementation pending)`);
