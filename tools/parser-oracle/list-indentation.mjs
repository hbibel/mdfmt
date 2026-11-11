import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";
import {applyFixes} from "markdownlint";

const rule=process.argv.includes("--rule")?process.argv[process.argv.indexOf("--rule")+1]:"MD005";
assert.ok(["MD005","MD007"].includes(rule));
const samples=["","- a\n - b\n"," - a\n- b\n","  - a\n - b\n- c\n","1. a\n 2. b\n"," 1. a\n2. b\n"," 9. a\n10. b\n11. c\n"," 9. a\n10. b\n 11. c\n"," 9. a\n10. b\n 2. c\n","1. a\n 2. b\n3. c\n","1) a\n 2) b\n","- a\n  - child\n   - child\n- b\n","-\n -\n- b\n","1.\n 2.\n3. b\n","- a\n\n - b\n","- a\n - b\n   continuation\n","- a\n - b\n\n       code\n","- a\n - b\n\n   | a |\n   | - |\n   | b |\n","- a\n - b\n\n   [id]: /url\n","- a\n - b [link](url)\n","- a\n - # Heading!\n","- a\n - ```text\n   code\n   ```\n","- a\n - > quote\n   > more\n","- a\n \t- b\n","- a\n\t- b\n","-\ta\n -\tb\n","1.\ta\n 2.\tb\n","> - a\n>  - b\n",">  - a\n> - b\n",">  9. a\n> 10. b\n>  11. c\n","- > - a\n  >  - b\n","> > - a\n> >  - b\n","[^a]:\n    - a\n     - b\n","---\ntitle: - a\n---\n- a\n - b\n","$$\n- a\n - b\n$$\n","```text\n- a\n - b\n```\n","    - a\n     - b\n","<div>\n- a\n - b\n</div>\n","- item\n<span>\n\n- a\n - b\n","> item\n<span>\n>\n> - a\n>  - b\n","text\n2. a\n 3. b\n","9.\ntext\n2. a\n 3. b\n"];
samples.push(`<!-- markdownlint-disable-next-line ${rule} -->\n- a\n - b\n`,`- a\n - b <!-- markdownlint-disable-line ${rule} -->\n`,`<!-- markdownlint-disable-file ${rule} -->\n- a\n - b\n`);
for(const a of [0,1,2,3])for(const b of [0,1,2,3])samples.push(`${" ".repeat(a)}- a\n${" ".repeat(b)}- b\n`,`${" ".repeat(a)}9. a\n${" ".repeat(b)}10. b\n 11. c\n`);
for(const input of [...samples].filter(Boolean))samples.push(input.replaceAll("\n","\r\n"),"\uFEFF"+input.replaceAll("\n","\r"));
const options=rule==="MD005"?[true,false,{severity:"warning"}]:[true,false,{indent:4},{start_indented:true},{start_indented:true,start_indent:3,indent:2},{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>options.map((options,configuration)=>{
 const findings=lint({strings:{input},config:{default:false,[rule]:options}}).input;const bom=input.startsWith("\uFEFF")?"\uFEFF":"";
 return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,column:f.errorRange[0],length:f.errorRange[1],detail:f.errorDetail,severity:f.severity})),output:bom+applyFixes(input.slice(bom.length),findings)};
}));
const receipt=new URL(`${rule.toLowerCase()}-observations.json`,import.meta.url),serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
function boundaries(input){
 let result=[];const types=new Set(["blockQuote","listOrdered","listUnordered","listItemPrefix","paragraph","atxHeading","setextHeading","codeFenced","codeIndented","gfmTable","definition","gfmFootnoteDefinition","codeFlowValue","codeFencedFenceInfo","codeFencedFenceMeta"]);
 lint({strings:{input},config:{default:false,capture_indent:true},customRules:[{names:["capture_indent"],description:"Capture list ownership and protected values",tags:["observation"],parser:"micromark",function(params){
  const visit=tokens=>{for(const token of tokens){if(types.has(token.type)){const parents=[];for(let parent=token.parent;parent;parent=parent.parent)parents.push(parent.type);const value=token.type==="listOrdered"?token.children.find(child=>child.type==="listItemPrefix")?.children.find(child=>child.type==="listItemValue"):null;result.push({type:token.type,parents,start:value?Number(value.text):null,literal:["codeFlowValue","codeFencedFenceInfo","codeFencedFenceMeta"].includes(token.type)?token.text:null});}visit(token.children);}};visit(params.parsers.micromark.tokens);
 }}]});return result;
}
if(process.argv.includes("--source-only"))console.log(`PASS: ${observations.length} pinned ${rule} source-only observations (native not checked)`);
else{
 const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url)),directory=mkdtempSync("/tmp/opencode/mdfmt-indent-"),configuration=directory+"/.markdownlint-cli2.jsonc";let differences=0;
 try{for(const row of observations)for(const fix of [false,true]){
  writeFileSync(configuration,JSON.stringify({fix,config:{default:false,[rule]:row.options}}));const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory,timeout:10000});
  if(!fix)assert.equal(native.stdout,row.input,row.id);const expected=lint({strings:{input:native.stdout},config:{default:false,[rule]:row.options}}).input.map(f=>({line:f.lineNumber,column:f.errorRange[0],detail:f.errorDetail,severity:f.severity}));
  const actual=[...native.stderr.matchAll(new RegExp(`^-:(\\d+):(\\d+): (Error|Warning) ${rule}: [^\\r\\n]* \\[(Expected: [^\\r\\n]*)\\]$`,"gm"))].map(m=>({line:Number(m[1]),column:Number(m[2]),detail:m[4],severity:m[3].toLowerCase()}));
  assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,native.stderr);
  if(fix){if(native.stdout!==row.output)differences++;assert.deepEqual(boundaries(native.stdout),boundaries(row.input),`${row.id}: protected values/ownership`);const second=spawnSync(binary,["-"],{input:native.stdout,encoding:"utf8",cwd:directory,timeout:10000});assert.equal(second.stdout,native.stdout,`${row.id}: idempotence`);assert.equal(second.stderr,native.stderr,row.id);assert.equal(second.status,native.status);}
 }}finally{unlinkSync(configuration);rmdirSync(directory);}
 console.log(`PASS: ${observations.length} ${rule} native diagnostics, ownership/code/idempotence cases; ${differences} intentional upstream-fix differences`);
}
