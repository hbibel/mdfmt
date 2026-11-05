import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";
import {applyFixes} from "markdownlint";
const table="| a | b |\n| --- | --- |\n| c | d |\n";
const samples=[];
for(const before of ["","\n","text\n","# heading\n","<!-- comment -->\n","<!-- open\n","end -->\n","visible <!-- comment -->\n"])
  for(const after of ["","\n","# heading\n","<!-- comment -->\n","visible <!-- comment -->\n"])
    samples.push(before+table+after);
for(const [first,next]of [["> ","> "],["- ","  "],["> - ",">   "],["- > ","  > "],["10. ","    "]]){
  const nested=first+table.trimEnd().split("\n").join("\n"+next)+"\n";
  samples.push(nested+"# after\n");
  samples.push("# before\n"+nested);
  samples.push(nested+next.trimEnd()+"\n"+next+"# after\n");
}
samples.push("\uFEFF# before\r\n"+table.replaceAll("\n","\r\n")+"# after\r\n",
  "# before\r"+table.replaceAll("\n","\r")+"# after",
  "# before\n"+table.trimEnd(),
  "- item\n<span>\n"+table+"# after\n",
  "# before\n| a | b |\n| --- | --- |\nexample | f\n# after\n",
  "<!-- markdownlint-disable-next-line MD058 -->\n"+table+"# after\n",
  "<!-- markdownlint-disable-file blanks-around-tables -->\n# before\n"+table+"# after\n",
  "# before\n| 😀 | b |\n| --- | --- |\n| c | d |\n# after\n");
const configurations=[true,false,{severity:"warning"}];
for(const metadata of ["---\ntitle: x\n---\n","+++\ntitle = 'x'\n+++\n","{\n\"title\": \"x\"\n}\n"]){
  for(const bom of ["","\uFEFF"])samples.push(bom+metadata+table);
}
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD058:options}}).input;
  const bom=input.startsWith("\uFEFF")?"\uFEFF":"";
  return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,range:f.errorRange,context:f.errorContext,severity:f.severity})),
    output:bom+applyFixes(input.slice(bom.length),findings)};
}));
const receipt=new URL("md058-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
if(process.argv.includes("--source-only"))console.log(`PASS: ${observations.length} pinned MD058 observations`);
else{
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md058-oracle-"),configuration=directory+"/.markdownlint-cli2.jsonc";
  let differences=0;
  const withoutBlanks=input=>input.split(/\r\n|\r|\n/).filter(line=>!/^[>\s]*$/.test(line));
  try{for(const row of observations){for(const fix of [false,true]){
    writeFileSync(configuration,JSON.stringify({fix,config:{default:false,MD058:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});
    if(!fix)assert.equal(native.stdout,row.input,row.id);
    assert.deepEqual(withoutBlanks(native.stdout),withoutBlanks(row.input),row.id);
    const expected=lint({strings:{input:native.stdout},config:{default:false,MD058:row.options}}).input.map(f=>({line:f.lineNumber,context:f.errorContext,severity:f.severity}));
    const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD058: Tables should be surrounded by blank lines \[Context: "([^\r\n]*)"\]$/gm)]
      .map(m=>({line:Number(m[1]),context:m[4],severity:m[3].toLowerCase()}));
    assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,native.stderr);
    if(fix){
      if(native.stdout!==row.output)differences++;
      const second=spawnSync(binary,["-"],{input:native.stdout,encoding:"utf8",cwd:directory});assert.equal(second.stdout,native.stdout,row.id);assert.equal(second.stderr,native.stderr,row.id);
    }
  }}}finally{unlinkSync(configuration);rmdirSync(directory);}
  console.log(`PASS: ${observations.length} pinned MD058/native diagnostic, separator-preservation and idempotence cases; ${differences} preserved upstream-fix differences`);
}
