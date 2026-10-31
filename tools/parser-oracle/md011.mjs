import assert from "node:assert/strict";
import {readFileSync,writeFileSync} from "node:fs";
import {lint} from "markdownlint/sync";
import {applyFixes} from "markdownlint";
const samples=[
  "(text)[url]\n","before (text)[url] after\n","(one)[url] and (two)[url2]\n","(one)[url](two)[url2]\n",
  "\\(text)[url]\n","\\\\(text)[url]\n","(text\\)[url]\n","(text)[url\\]\n","(text)[^note]\n",
  "(text)[]\n","()[url]\n","((text))[url]\n","(text)[url](other)\n","(😀)[https://example.com]\n",
  "`(text)[url]`\n","`a\n(text)[url]\nb`\n","``(text)[url]`` and (real)[url]\n",
  "$ (text)[url] $\n","$$ (text)[url] $$\n","$a\n(text)[url]\nb$\n","$$\n(text)[url]\n$$\n",
  "```\n(text)[url]\n```\n","    (text)[url]\n","<div>\n(text)[url]\n</div>\n",
  "<span>(text)[url]</span>\n","- item\n<span>\n(text)[url]\n","> (text)[url]\n","- (text)[url]\n",
  "\uFEFF(text)[url]\r\n","(text)[url]\r(next)[dest]\r","---\ntitle: (text)[url]\n---\n(real)[url]\n",
  "<!-- markdownlint-disable-next-line MD011 -->\n(text)[url]\n(real)[url]\n",
  "<!-- markdownlint-disable-file no-reversed-links -->\n(text)[url]\n",
];
const configurations=[true,false,{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD011:options}}).input;
  const bom=input.startsWith("\uFEFF")?"\uFEFF":"";
  return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,column:f.errorRange[0],length:f.errorRange[1],detail:f.errorDetail,severity:f.severity})),
    output:bom+applyFixes(input.slice(bom.length),findings)};
}));
const receipt=new URL("md011-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
console.log(`PASS: ${observations.length} pinned MD011 observations (native implementation pending)`);
