import assert from "node:assert/strict";
import {mkdtempSync,mkdirSync,readFileSync,writeFileSync,cpSync,rmSync,existsSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";

const option=(name,fallback)=>{const index=process.argv.indexOf(name);return index<0?fallback:path.resolve(process.argv[index+1]);};
const modules=option("--modules",fileURLToPath(new URL("node_modules/",import.meta.url)));
const binary=option("--binary",fileURLToPath(new URL("../../target/release/mdfmt",import.meta.url)));
if(process.platform==="win32"){console.log("SKIP: POSIX executable-wrapper isolation probe; Windows formatter transport is separately qualified");process.exit(0);}
for(const [alias,version] of [["prettier-v2","2.8.8"],["prettier-v3","3.6.2"]]){
 const source=path.join(modules,alias);assert.equal(JSON.parse(readFileSync(path.join(source,"package.json"))).version,version);
 const root=mkdtempSync("/tmp/opencode/mdfmt-prettier-isolation-");
 try{
  const project=path.join(root,"project"),installed=path.join(project,"node_modules"),tool=path.join(installed,"prettier");mkdirSync(installed,{recursive:true});cpSync(source,tool,{recursive:true});
  const plugin=path.join(installed,"prettier-plugin-probe"),marker=path.join(root,"loaded"),calls=path.join(root,"calls");mkdirSync(plugin);writeFileSync(path.join(plugin,"package.json"),JSON.stringify({name:"prettier-plugin-probe",version:"1.0.0",main:"index.js"}));writeFileSync(path.join(plugin,"index.js"),`require("node:fs").appendFileSync(${JSON.stringify(marker)},"loaded\\n");module.exports={};\n`);
  const metadata=JSON.parse(readFileSync(path.join(tool,"package.json"))),entry=typeof metadata.bin==="string"?metadata.bin:metadata.bin.prettier;
  const bin=path.join(root,"bin");mkdirSync(bin);writeFileSync(path.join(bin,"prettier"),`#!${process.execPath}\nrequire("node:fs").appendFileSync(${JSON.stringify(calls)},"call\\n");require(${JSON.stringify(path.join(tool,entry))});\n`,{mode:0o755});
  writeFileSync(path.join(project,".markdownlint-cli2.jsonc"),JSON.stringify({config:{default:false,MD013:{strict:true,code_block_line_length:20}}}));
  const input='```json\n{"alpha":1,"beta":2}\n```\n',expected='```json\n{\n  "alpha": 1,\n  "beta": 2\n}\n```\n';writeFileSync(path.join(project,"doc.md"),input);
  const invoke=()=>spawnSync(binary,["--format-code","doc.md"],{cwd:project,env:{...process.env,PATH:bin},encoding:"utf8",timeout:15000});
  const first=invoke();assert.equal(first.status,0,first.stderr);assert.equal(first.stdout,expected);assert.equal(existsSync(marker),false,`${version}: neighboring plugin executed`);writeFileSync(path.join(project,"doc.md"),first.stdout);
  const second=invoke();assert.equal(second.status,0,second.stderr);assert.equal(second.stdout,expected);assert.equal(second.stderr,first.stderr);assert.equal(existsSync(marker),false,`${version}: neighboring plugin executed on repeat`);assert.equal(readFileSync(calls,"utf8"),"call\n".repeat(4));
  console.log(`PASS: actual Prettier ${version}: exact JSON output, four stability invocations, no neighboring plugin execution`);
 }finally{rmSync(root,{recursive:true,force:true});}
}
