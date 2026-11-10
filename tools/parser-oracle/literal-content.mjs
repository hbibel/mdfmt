import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";

const rule=process.argv[process.argv.indexOf("--rule")+1];
assert.equal(rule,"MD046","Use --rule MD046 (MD034/MD044 remain unimplemented)");
const options=[true,false,{style:"consistent"},{style:"fenced"},{style:"indented"},{severity:"warning",style:"fenced"}];
const samples=["","text\n","```\n```\n","```rs\nlet x = 1;\n```\n\n    code\n","    code\n\n~~~js\nsecond\n~~~\n","```\nunclosed\n","~~~\nunclosed\n","    ```\n    literal fence\n","\tcode\n","   \tcode\n"," \t```\n\tvalue\n","---\ntitle: code\n---\n    code\n","<div>\n```\nhtml\n```\n</div>\n","- item\n<span>\n```\ncode\n```\n","- item\n<span>\n  ```\n  html\n  ```\n","<!-- markdownlint-disable MD046 -->\n```\nx\n```\n<!-- markdownlint-enable MD046 -->\n\n    y\n","<!-- markdownlint-disable-next-line code-block-style -->\n    x\n\n```\ny\n```\n","<!-- markdownlint-disable-file MD046 -->\n    x\n"];
for(const first of ["```\na\n```\n","~~~rs\na\n~~~\n","    a\n","\ta\n"]){
  for(const second of ["```\nb\n```\n","~~~\nb\n~~~\n","    b\n","\tb\n"]){
    const input=first+"\n"+second;
    for(const prefix of ["","> ","- ","> - "]){
      const continuation=prefix.replaceAll("- ","  ");
      samples.push(prefix+input.trimEnd().split("\n").map((line,index)=>index?continuation+line:line).join("\n")+"\n");
    }
  }
}
for(const input of [...samples].filter(Boolean))samples.push(input.replaceAll("\n","\r\n"),"\uFEFF"+input.replaceAll("\n","\r"));
const rows=samples.flatMap((input,index)=>options.map((options,configuration)=>({id:`${index}-${configuration}`,input,options,findings:lint({strings:{input},config:{default:false,[rule]:options}}).input.map(f=>({line:f.lineNumber,range:f.errorRange,context:f.errorContext,detail:f.errorDetail,severity:f.severity}))})));
const path=new URL(`${rule.toLowerCase()}-observations.json`,import.meta.url),text="[\n"+rows.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(path,text);else assert.equal(text,readFileSync(path,"utf8"));
if(!process.argv.includes("--source-only")){
  const directory=mkdtempSync("/tmp/opencode/mdfmt-literal-"),configuration=directory+"/.markdownlint-cli2.jsonc",binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  try{for(const row of rows)for(const fix of [false,true]){
    writeFileSync(configuration,JSON.stringify({fix,config:{default:false,[rule]:row.options}}));
    const result=spawnSync(binary,["-"],{cwd:directory,input:row.input,encoding:"utf8",timeout:30000});
    assert.equal(result.stdout,row.input,`${row.id}: exact source`);
    const actual=[...result.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD046: Code block style \[(.*)\]$/gm)].map(m=>({line:Number(m[1]),column:Number(m[2]),severity:m[3].toLowerCase(),detail:m[4]}));
    const expected=row.findings.map(f=>({line:f.line,column:f.range?.[0]??1,severity:f.severity,detail:f.detail}));
    assert.deepEqual(actual,expected,`${row.id}: ${result.stderr}`);assert.equal(result.status,expected.some(f=>f.severity==="error")?1:0);
    if(fix){const again=spawnSync(binary,["-"],{cwd:directory,input:result.stdout,encoding:"utf8",timeout:30000});assert.equal(again.stdout,result.stdout);assert.equal(again.stderr,result.stderr);assert.equal(again.status,result.status);}
  }}finally{unlinkSync(configuration);rmdirSync(directory);}
}
console.log(`PASS: ${rows.length} pinned ${rule} observations${process.argv.includes("--source-only")?" (native implementation not checked)":" with native diagnostics, exact source and idempotence comparisons"}`);
