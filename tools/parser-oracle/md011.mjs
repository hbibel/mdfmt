import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
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
  "(a `b)[c]`\n","`(a`)[b]\n","$ (a $)[b]\n",
  "`a`(text)[url]\n","$a$(text)[url]\n","`start\n`(text)[url]`\nend`\n","(text)[url]`a`\n",
];
for(const prefix of ["- item\n","> text\n"])for(const literal of ["`(text)[url]`","$(text)[url]$","``(text)[url]``"])
  samples.push(prefix+"<span>\n"+literal+"\n(real)[url]\n");
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
if(process.argv.includes("--source-only"))console.log(`PASS: ${observations.length} pinned MD011 observations`);
else{
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md011-oracle-"),configuration=directory+"/.markdownlint-cli2.jsonc";
  let differences=0;
  try{for(const row of observations){for(const fix of [false,true]){
    writeFileSync(configuration,JSON.stringify({fix,config:{default:false,MD011:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});assert.equal(native.stdout,row.input,row.id);
    const expected=row.findings.map(({length,...finding})=>finding);
    const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD011: Reversed link syntax \[([^\r\n]*)\]$/gm)]
      .map(m=>({line:Number(m[1]),column:Number(m[2]),detail:m[4],severity:m[3].toLowerCase()}));
    assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,native.stderr);
    if(fix&&native.stdout!==row.output)differences++;
  }}}finally{unlinkSync(configuration);rmdirSync(directory);}
  console.log(`PASS: ${observations.length} pinned MD011/native diagnostics and source-preservation cases; ${differences} meaning-preserving upstream-fix differences`);
}
