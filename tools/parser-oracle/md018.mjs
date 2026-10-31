import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
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
  "$$\n```\n$$\n#Heading\n","$$\n<div>\n$$\n#Heading\n","$$ info\n#Hidden\n$$\n#Heading\n",
  "#A very long heading whose context must be truncated at thirty units\n",
  "$$$\n#Hidden\n$$\n#StillHidden\n$$$\n#Heading\n","$$ invalid$\n#Heading\n",
  "```\n$$\n#Hidden\n```\n#Heading\n","<div>\n$$\n#Hidden\n</div>\n\n#Heading\n",
  "- item\n  $$\n#Heading\n","- item\n  $$\n#Heading\n$$\n#After\n",
];
for(const prefix of ["","- item\n  ","> ","1. item\n   "])for(const fence of ["$$","$$$","$$ info","$$ invalid$","    $$"])
  samples.push(prefix+fence+"\n#Heading\n$$\n#After\n");
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
if(process.argv.includes("--source-only"))console.log(`PASS: ${observations.length} pinned MD018 observations`);
else{
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md018-oracle-"),configuration=directory+"/.markdownlint-cli2.jsonc";
  let differences=0;
  try{for(const row of observations){for(const fix of [false,true]){
    writeFileSync(configuration,JSON.stringify({fix,config:{default:false,MD018:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});
    assert.equal(native.stdout,row.input,row.id);
    const expected=row.findings.map(({length,...finding})=>finding);
    const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD018: No space after hash on atx style heading \[Context: "([^\r\n]*)"\]$/gm)]
      .map(m=>({line:Number(m[1]),column:Number(m[2]),context:m[4],severity:m[3].toLowerCase()}));
    assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,native.stderr);
    if(fix&&native.stdout!==row.output)differences++;
  }}}finally{unlinkSync(configuration);rmdirSync(directory);}
  console.log(`PASS: ${observations.length} pinned MD018/native diagnostics and source-preservation cases; ${differences} meaning-preserving upstream-fix differences`);
}
