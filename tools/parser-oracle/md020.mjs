import assert from "node:assert/strict";
import {readFileSync,writeFileSync} from "node:fs";
import {lint} from "markdownlint/sync";
import {applyFixes} from "markdownlint";
const samples=[
  "#Heading#\n","# Heading#\n","#Heading #\n","# Heading #\n","##Heading##\n","#######Heading#######\n",
  "#H#\n","# #\n","###\n","#\\#\n","# Heading\\##\n","# Heading \\##\n","# Heading \\###\n",
  "#Heading\\#\n","#Heading\\##\n","#Heading##\n","#Heading ### \t\n","# Heading#\u00a0\n",
  "#Heading#\u0085\n","#Heading#\uFEFF\n","#😀#\n","# 😀#\n","#😀 #\n","#️⃣#\n",
  " #Heading#\n","> #Heading#\n","- #Heading#\n","    #Heading#\n","```\n#Heading#\n```\n",
  "<div>\n#Heading#\n</div>\n","#<b>Heading</b>#\n","<!--\n#Heading#\n-->\n",
  "$$\n#Heading#\n$$\n","- item\n  $$\n#Heading#\n$$\n#After#\n",
  "\uFEFF#Heading#\r\n","#Heading#\r#Second#\r","---\ntitle: hello\n---\n#Heading#\n",
  "<!-- markdownlint-disable-next-line MD020 -->\n#Heading#\n#Second#\n",
  "<!-- markdownlint-disable-file no-missing-space-closed-atx -->\n#Heading#\n",
  "#Heading# <!-- markdownlint-disable-line MD020 -->\n",
  "#A long closed heading that must truncate context#\n","# A long closed heading that must truncate context#\n","#A long closed heading that must truncate context #\n",
];
const configurations=[true,false,{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD020:options}}).input;
  const bom=input.startsWith("\uFEFF")?"\uFEFF":"";
  return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,column:f.errorRange[0],length:f.errorRange[1],context:f.errorContext,severity:f.severity})),
    output:bom+applyFixes(input.slice(bom.length),findings)};
}));
const receipt=new URL("md020-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
console.log(`PASS: ${observations.length} pinned MD020 observations (native implementation pending)`);
