import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";

const ruleIndex=process.argv.indexOf("--rule"),rule=ruleIndex<0?"MD022":process.argv[ruleIndex+1];
assert.ok(["MD022","MD031"].includes(rule),"Only MD022/MD031 are audited in this batch so far");
let samples=[
  "before\n# Heading\nafter\n", "# Heading\n", "# Heading", "", "\uFEFF# Heading\r\nnext\r\n",
  "before\n\n# Heading\n\nafter\n", "before\n\n\n# H\n\n\nafter\n",
  "# First\n## Second\n### Third\n", "text\nH\n---\nnext\n", "before\n\nH\n===\nnext\n",
  "# First\n\n## Second\n",
  "# # Literal\nnext\n", "# Closed #\nnext\n", "# H <!-- é -->\nafter\n",
  "before\n<!-- comment -->\n# H\n<!-- trailing -->\nafter\n", "before\n>\n# H\n>\nafter\n",
  "---\ntitle: T\n---\n# H\nnext\n", "+++\ntitle='T'\n+++\n# H\n",
  '{\n"title":"T"\n}\n# H\n', "\uFEFF---\ntitle: T\n---\n# H\nnext\n",
  "before\r# H\rafter\r", "before\r\n# H\nafter\r", "#H\nnext\n",
  "```\n# H\n```\n", "    # H\n", "$$\n# H\n$$\n", "<div>\n# H\n</div>\n",
  "- item\n<span>\nA\nB\n===\nnext\n", "<!--\n# H\n-->\n", "# H\n<!-- open\nnext\n",
  "<!-- markdownlint-disable-next-line MD022 -->\n# H\nnext\n",
  "<!-- markdownlint-disable-file blanks-around-headings -->\n# H\nnext\n",
  "# First <!-- markdownlint-disable-next-line MD026 -->\n# Second!\n",
  "# H\n<!-- markdownlint-disable MD022 -->\n# Other\nnext\n",
  "before\n# One\n## Two\n### Three\nafter\n",
];
for(const [first,next]of [["", ""],["> ","> "],["- ","  "],["> - ",">   "],["- > ","  > "],["1. ","   "],["-\t","  "],["-\t","    "]]) {
  for(const middle of ["# H\n","H\n===\n","## H ##\n"]) {
    samples.push(first+"before\n"+middle.split("\n").filter(Boolean).map(line=>next+line+"\n").join("")+next+"after\n");
  }
  samples.push(first+"# H\n"+next+"after\n");
}
for(let level=1;level<=6;level++) samples.push(`before\n${"#".repeat(level)} H\nafter\n`);
let options=[true,false,{severity:"warning"},{lines_above:0,lines_below:0},{lines_above:2,lines_below:2},{lines_above:-1,lines_below:-1},{lines_above:[2,0],lines_below:-1},{lines_above:[],lines_below:[0,2]},{include_front_matter:true}];
if(rule==="MD031"){
  samples=["before\n```rust\nx\n```\nafter\n","before\n~~~\nx\n~~~~\nafter\n","before\n\n```\nx\n```\n\nafter\n","```\nx\n```\n","```\nx\n```","```\n","before\n```\nx\n","before\n```\nx\n\n","before\n````\n```\nx\n````\nafter\n","    ```\n    x\n    ```\n","\t```\n\tx\n\t```\n","---\ntitle: T\n---\n```\nx\n```\nafter\n","+++\ntitle='T'\n+++\n```\nx\n```\n",'\uFEFF---\ntitle: T\n---\n```\nx\n```\n',"before\r\n```rust\r\nx\r\n```\r\nafter\r\n","before\r```\rx\r```\rafter\r","before\n```\nx\n```\r\nafter\n","<div>\n```\nx\n```\n</div>\n","$$\n```\nx\n```\n$$\n","<!--\n```\nx\n```\n-->\n","<!-- comment -->\n```\nx\n```\n<!-- after -->\n","before <!-- markdownlint-disable-next-line MD026 -->\n```rust\nx\n```\n","<!-- markdownlint-disable-next-line MD031 -->\n```\nx\n```\nafter\n","<!-- markdownlint-disable-file blanks-around-fences -->\nbefore\n```\nx\n```\nafter\n","before\n```rust\n```\nafter\n","before\n```a very long language info string with many words\nx\n```\nafter\n","- item\n<span>\n```\nx\n```\nafter\n","- before\n\n  ```rust\n  x\n  ```\n  after\n",""];
  for(const [first,next]of [["",""],["> ","> "],["- ","  "],["1. ","   "],["> - ",">   "],["- > ","  > "],["-\t","  "],["-\t","    "]]){
    for(const fence of ["```rust","~~~"]){
      const close=fence.startsWith("`")?"```":"~~~";
      samples.push(`${first}before\n${next}${fence}\n${next}x\n${next}${close}\n${next}after\n`);
      samples.push(`${first}${fence}\n${next}x\n${next}${close}\n${next}after\n`);
    }
  }
  samples.push("before\n   ```rust\n   x\n  ```\nafter\n","> before\n> ```rust\n> x\n> ```\n>\n","- > before\n\n  > ```rust\n  > x\n  > ```\n  > after\n","- > quote\n<span>\n  ```rust\n  x\n  ```\n  after\n","> - item\n<span>\n```rust\nx\n```\nafter\n");
  options=[true,false,{severity:"warning"},{list_items:false}];
}
const observations=samples.flatMap((input,sample)=>options.map((options,configuration)=>({id:`${sample}-${configuration}`,input,options,findings:findings(input,options)})));
function findings(input,options){return lint({strings:{input},config:{default:false,[rule]:options}}).input.map(f=>({line:f.lineNumber,range:f.errorRange,detail:f.errorDetail,context:f.errorContext,severity:f.severity}));}
function message(f){return (rule==="MD022"?"Headings should be surrounded by blank lines":"Fenced code blocks should be surrounded by blank lines")+" ["+[f.detail,f.context===null?null:`Context: "${f.context}"`].filter(x=>x!==null).join("; ")+"]";}
const receipt=new URL(rule.toLowerCase()+"-observations.json",import.meta.url),serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
if(!process.argv.includes("--source-only")){
  const directory=mkdtempSync("/tmp/opencode/mdfmt-block-separators-"),binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  try{for(const row of observations){for(const fix of [false,true]){
    writeFileSync(directory+"/.markdownlint-cli2.jsonc",JSON.stringify({fix,config:{default:false,[rule]:row.options}}));
    const native=spawnSync(binary,["-"],{cwd:directory,input:row.input,encoding:"utf8"});
    if(!fix)assert.equal(native.stdout,row.input,row.id);
    const actual=[...native.stderr.matchAll(new RegExp(`^-:(\\d+):1: (Error|Warning) ${rule}: (.*)$`,"gm"))].map(m=>({line:Number(m[1]),severity:m[2].toLowerCase(),message:m[3]}));
    const expected=findings(native.stdout,row.options).map(f=>({line:f.line,severity:f.severity,message:message(f)}));
    assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,row.id);
    const visible=input=>input.split(/\r\n|\r|\n/).filter(line=>!/^\s*>?[>\s]*$/u.test(line));
    assert.deepEqual(visible(native.stdout),visible(row.input),row.id);
    if(fix){const again=spawnSync(binary,["-"],{cwd:directory,input:native.stdout,encoding:"utf8"});assert.equal(again.stdout,native.stdout,`${row.id}: idempotence`);assert.equal(again.stderr,native.stderr,`${row.id}: stable findings`);}
  }}}finally{rmSync(directory,{recursive:true});}
}
console.log(`PASS: ${observations.length} pinned ${rule} observations${process.argv.includes("--source-only")?"":" with native diagnostics/preservation/idempotence comparisons"}`);
