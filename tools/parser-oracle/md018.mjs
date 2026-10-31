import assert from "node:assert/strict";
import {readFileSync,writeFileSync} from "node:fs";
import {lint} from "markdownlint/sync";
import {applyFixes} from "markdownlint";
const samples=[
  "#Heading\n","##Heading\n","#######Heading\n","# Heading\n","#\tHeading\n","#\n","##\n",
  "#Heading#\n","#Heading # \n","#Heading\\#\n","#️⃣ keycap\n","#😀 text\n","#é text\n",
  " #Heading\n","    #Heading\n","> #Heading\n","- #Heading\n","1. #Heading\n","text\n#Heading\n",
  "```\n#Heading\n```\n","~~~\n#Heading\n","    code\n#Heading\n","<div>\n#Heading\n</div>\n",
  "<!-- comment\n#Heading\n-->\n","<!-- comment -->\n#Heading\n","#<b>Heading</b>\n",
  "$$\n#Heading\n$$\n","$\n#Heading\n$\n","---\ntitle: hello\n---\n#Heading\n",
  "\uFEFF#Heading\r\n","#Heading\r#Second\r","#Heading\u00a0\n","#Heading#\u00a0\n",
  "<!-- markdownlint-disable-next-line MD018 -->\n#Heading\n#Second\n",
  "<!-- markdownlint-disable-file no-missing-space-atx -->\n#Heading\n","#Heading <!-- markdownlint-disable-line MD018 -->\n",
];
const configurations=[true,false,{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD018:options}}).input;
  const bom=input.startsWith("\uFEFF")?"\uFEFF":"";
  return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,column:f.errorRange[0],length:f.errorRange[1],context:f.errorContext,severity:f.severity})),
    output:bom+applyFixes(input.slice(bom.length),findings)};
}));
const receipt=new URL("md018-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
console.log(`PASS: ${observations.length} pinned MD018 observations (native implementation pending)`);
