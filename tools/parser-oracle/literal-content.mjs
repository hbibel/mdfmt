import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";

const rule=process.argv[process.argv.indexOf("--rule")+1];
assert.ok(["MD034","MD046"].includes(rule),"Use --rule MD034 or MD046 (MD044 remains unimplemented)");
const options=rule==="MD034"?[true,false,{severity:"warning"}]:[true,false,{style:"consistent"},{style:"fenced"},{style:"indented"},{severity:"warning",style:"fenced"}];
const samples=rule==="MD034"?["","text\n","go https://example.com now\n","[https://example.com](url)\n","![https://example.com](img)\n","<https://example.com>\n","<div>\nhttps://example.com\n</div>\n","- item\n<span>\nhttps://example.com\n","<!-- markdownlint-disable-next-line MD034 -->\nhttps://example.com\n","<!-- markdownlint-disable MD034 -->\nhttps://a.com\n<!-- markdownlint-enable MD034 -->\nhttps://b.com\n","<!-- https://example.com -->\n","---\ntitle: https://example.com\n---\nhttps://example.com\n"]:["","text\n","```\n```\n","```rs\nlet x = 1;\n```\n\n    code\n","    code\n\n~~~js\nsecond\n~~~\n","```\nunclosed\n","~~~\nunclosed\n","    ```\n    literal fence\n","\tcode\n","   \tcode\n"," \t```\n\tvalue\n","---\ntitle: code\n---\n    code\n","<div>\n```\nhtml\n```\n</div>\n","- item\n<span>\n```\ncode\n```\n","- item\n<span>\n  ```\n  html\n  ```\n","<!-- markdownlint-disable MD046 -->\n```\nx\n```\n<!-- markdownlint-enable MD046 -->\n\n    y\n","<!-- markdownlint-disable-next-line code-block-style -->\n    x\n\n```\ny\n```\n","<!-- markdownlint-disable-file MD046 -->\n    x\n"];
if(rule==="MD034")for(const url of ["https://example.com","http://example.com/a(b)","www.example.com","a@example.com","HTTPS://EXAMPLE.COM/a_b","https://ex.com/a*b","https://ex.com/a&amp;b","https://ex.com/a\\*b","https://ex.com/😀"+"a".repeat(30),"https://ex.com/界","https://a.com.","https://a.com,","https://a.com>"]){
  for(const wrap of [s=>s+"\n",s=>`> ${s}\n`,s=>`- ${s}\n`,s=>`**${s}**\n`,s=>`[${s}](url)\n`,s=>`![${s}](img)\n`,s=>`\`${s}\`\n`,s=>`$${s}$\n`,s=>`<span>${s}</span>\n`,s=>`<span>${s}\n`,s=>`<br>${s}\n`,s=>`<span><span>${s}</span></span>\n`,s=>`| a | b |\n| - | - |\n| ${s} | b |\n`,s=>`\\<${s}>\n`])samples.push(wrap(url));
}
if(rule==="MD046")for(const first of ["```\na\n```\n","~~~rs\na\n~~~\n","    a\n","\ta\n"]){
  for(const second of ["```\nb\n```\n","~~~\nb\n~~~\n","    b\n","\tb\n"]){
    const input=first+"\n"+second;
    for(const prefix of ["","> ","- ","> - "]){
      const continuation=prefix.replaceAll("- ","  ");
      samples.push(prefix+input.trimEnd().split("\n").map((line,index)=>index?continuation+line:line).join("\n")+"\n");
    }
  }
}
if(rule==="MD046")samples.push("-    ```sh\n  \t$ echo hi\n     ```\n",">     $ echo hi\n> >\n>     output\n",">     $ echo hi\n>\n> >\n>\n>     output\n");
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
    const pattern=rule==="MD046"?/^-:(\d+):(\d+): (Error|Warning) MD046: Code block style \[(.*)\]$/gm:/^-:(\d+):(\d+): (Error|Warning) MD034: Bare URL used \[Context: "(.*)"\]$/gm;
    const actual=[...result.stderr.matchAll(pattern)].map(m=>({line:Number(m[1]),column:Number(m[2]),severity:m[3].toLowerCase(),detail:rule==="MD046"?m[4]:null,context:rule==="MD034"?m[4]:null}));
    const expected=row.findings.map(f=>({line:f.line,column:f.range?.[0]??1,severity:f.severity,detail:f.detail,context:f.context}));
    assert.deepEqual(actual,expected,`${row.id}: ${result.stderr}`);assert.equal(result.status,expected.some(f=>f.severity==="error")?1:0);
    if(fix){const again=spawnSync(binary,["-"],{cwd:directory,input:result.stdout,encoding:"utf8",timeout:30000});assert.equal(again.stdout,result.stdout);assert.equal(again.stderr,result.stderr);assert.equal(again.status,result.status);}
  }}finally{unlinkSync(configuration);rmdirSync(directory);}
}
console.log(`PASS: ${rows.length} pinned ${rule} observations${process.argv.includes("--source-only")?" (native implementation not checked)":" with native diagnostics, exact source and idempotence comparisons"}`);
