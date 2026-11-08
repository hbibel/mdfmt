import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";

const rule=process.argv[process.argv.indexOf("--rule")+1];
assert.ok(["MD037","MD038","MD039"].includes(rule),"Use --rule MD037, MD038 or MD039");
let samples=[
  "","plain text\n","alpha * beta * omega\n","alpha _ beta _ omega\n",
  "alpha ** beta ** omega\n","alpha __ beta __ omega\n",
  "alpha *** beta *** omega\n","alpha ___ beta ___ omega\n",
  "alpha **** beta **** omega\n","alpha * beta ** omega\n","alpha * beta _ omega\n",
  "alpha * beta * gamma * omega\n","alpha * beta * gamma * delta * omega\n",
  "alpha _* beta_ gamma * omega\n","alpha _* beta * gamma_ omega\n",
  "alpha * ` beta *` omega\n","alpha * [ beta *](url) omega\n",
  "alpha * ![ beta *](img) omega\n","alpha \\* beta * omega\n",
  "alpha *beta* omega\n","alpha **beta** omega\n","alpha _beta_ omega\n",
  "alpha * beta* omega\n","alpha *beta * omega\n","alpha *  beta  * omega\n",
  "alpha *\tbeta\t* omega\n","alpha *\u00a0beta\u00a0* omega\n",
  "alpha * 😀 * omega\n","alpha * beta\n* omega\n","alpha *\nbeta * omega\n",
  "alpha *\n* omega\n","alpha *     * omega\n","alpha * * omega\n",
  "alpha * <!-- c --> beta * omega\n","<!-- 😀 --> alpha * beta * omega\n",
  "alpha <!-- * beta * --> omega\n","alpha <!-- unfinished\n* beta * omega\n",
  "`alpha * beta * omega`\n","```md\nalpha * beta * omega\n```\n",
  "    alpha * beta * omega\n","$alpha * beta * omega$\n","$$\nalpha * beta * omega\n$$\n",
  "<div>\nalpha * beta * omega\n</div>\n","<span>alpha * beta * omega</span>\n",
  "- item\n<span>\nalpha * beta * omega\n",
  "[alpha * beta * omega](url)\n","![alpha * beta * omega](img)\n",
  "![[alpha][*id*]](img)\n\n[*id*]: /u\n\nalpha * beta * omega\n",
  "> ![![alpha * beta\n> * omega](inner)](outer)\n",
  "| alpha * beta * omega |\n| --- |\n",
  "> _`a`*\n> *`b`_\n","> _`a`*\n>   *`b`_\n","> _`a`*\n>     *`b`_\n","> - _`a`*\n>   *`b`_\n","> [`a`*\n> *`b`](url)\n","> ![`a`*\n> *`b`](img)\n","> _&amp;*\n> *&amp;_\n","> _\\!*\n> *\\!_\n",
  "---\ntitle: alpha * beta * omega\n---\nalpha * beta * omega\n",
  "\uFEFFalpha * beta * omega\r\n","alpha * beta * omega\r","alpha * beta * omega\r\n",
  `<!-- markdownlint-disable-next-line ${rule} -->\nalpha * beta * omega\n`,
  `alpha * beta * omega <!-- markdownlint-disable-line ${rule} -->\n`,
  `<!-- markdownlint-disable-file ${rule} -->\nalpha * beta * omega\n`,
];
for(const marker of ["*","**","***","_","__","___"]){
  for(const [first,next]of [["",""],["> ","> "],["- ","  "],["> - ",">   "],["- > ","  > "],["-\t","    "]]){
    samples.push(`${first}alpha ${marker} beta ${marker} omega\n`);
    samples.push(`${first}alpha ${marker} beta\n${next}${marker} omega\n`);
  }
}
const options=[true,false,{severity:"warning"}];
if(rule==="MD037")for(const body of ["* *","*  *","*`x`*","`a`*\n*`b`","&amp;*\n*&amp;","&bogus;*\n*&bogus;","\\!*\n*\\!","*_*","**_**","* ** *","* *** *","* ~~x~~ *","* [x](u) *","* <i>x</i> *","* a * b * c *","*** a ***","_ a _","*\u2003x\u2003*","* 😀 *","*\t x \t*","**** a ****"]){
  for(const wrap of [s=>`alpha ${s} omega\n`,s=>`alpha _${s}_ omega\n`,s=>`alpha **${s}** omega\n`,s=>`[${s}](u)\n`,s=>`![${s}](i)\n`,s=>`> _${s.replaceAll("\n","\n>   ")}_\n`,s=>`- _${s.replaceAll("\n","\n  ")}_\n`,s=>`| ${s.replaceAll("\n"," ")} |\n| --- |\n`])samples.push(wrap(body));
}
if(rule==="MD038"){
  for(const body of ["`text`","` text `","`  text  `","`  text`","`text  `","` `","`   `","` \t `","`\tfoo\t`","`\u00a0foo\u00a0`","`\u2003foo\u2003`","`` `x` ``","``  `x`  ``","`` `x``","``  `x``","``x` ``","``x`  ``","` first\nlast `","`  first\nlast  `","` first\n last `","`\ntext\n`","`\ntext `","` text\n`","`  a\nb`","`a\nb  `","`  "+"😀".repeat(20)+"  `","```  text  ```","\\`  text  `","`  text  \\`","``  a`b  ``","`  a | b  `"]){
    for(const wrap of [s=>`alpha ${s} omega\n`,s=>`[${s}](u)\n`,s=>`![${s}](i)\n`,s=>`> ${s.replaceAll("\n","\n> ")}\n`,s=>`- ${s.replaceAll("\n","\n  ")}\n`,s=>`> - ${s.replaceAll("\n","\n>   ")}\n`,s=>`| ${s.replaceAll("\n"," ")} |\n| --- |\n`,s=>`<!-- 😀 --> ${s}\n`])samples.push(wrap(body));
  }
  samples.push("\uFEFF`  text  `\r\n","> `  first\r\n> last  `\r\n","![![`  first\n  last  `](inner)](outer)\n","$`  text  `$\n","<span>`  text  `</span>\n","<div>\n`  text  `\n</div>\n","---\ntitle: `  text  `\n---\n`  text  `\n",`<!-- markdownlint-disable-next-line ${rule} -->\n\`  text  \`\n`,`> \`  first <!-- markdownlint-disable-next-line ${rule} -->\n> last  \`\n`);
}
if(rule==="MD038")for(const input of ["`a\n  b  `\n","`  a\n  b`\n","> `a\n>   b  `\n","- `a\n    b  `\n","> - `a\n>     b  `\n","-\t`a\n    b  `\n","\t> `  a  `\n"])samples.push(input);
if(rule==="MD039"){
  for(const label of ["label"," label ","  label  "," label","label ","\tlabel\t","\u00a0label\u00a0","\u2003label\u2003","\u0085label\u0085","\ufefflabel\ufeff","\nlabel\n","\n label\n"," \n label\n "," ","\n"," **label** "," ` label ` "," ![ alt ](img) "," <i>label</i> "," [literal] "," \\[label\\] "," &amp; "," "+"😀".repeat(20)+" "]){
    for(const wrap of [s=>`[${s}](u)\n`,s=>`![${s}](i)\n`,s=>`[${s}][id]\n\n[id]: /u\n`,s=>`[${s}][]\n\n[${s.replaceAll("\n"," ")}]: /u\n`,s=>`![ [${s}](u) ](i)\n`,s=>`> [${s.replaceAll("\n","\n> ")}](u)\n`,s=>`- [${s.replaceAll("\n","\n  ")}](u)\n`,s=>`| [${s.replaceAll("\n"," ")}](u) |\n| --- |\n`])samples.push(wrap(label));
  }
  samples.push("[ label ][missing]\n","[ label ]\n\n[label]: /u\n","![![ [ label ][id] ](inner)](outer)\n\n[id]: /u\n","> ![![ [ label\n> end ](u) ](inner)](outer)\n","\uFEFF[ label ](u)\r\n","[\r\n label\r\n](u)\r\n","<span>[ label ](u)</span>\n","<div>\n[ label ](u)\n</div>\n","---\ntitle: [ label ](u)\n---\n[ label ](u)\n",`<!-- markdownlint-disable-next-line ${rule} -->\n[ label ](u)\n`,`[ label ](u) <!-- markdownlint-disable-line ${rule} -->\n`,`[ label <!-- markdownlint-disable-next-line ${rule} -->\nend ](u)\n`);
}
function findings(input,options){return lint({strings:{input},config:{default:false,[rule]:options}}).input.map(f=>({
  line:f.lineNumber,range:f.errorRange,context:f.errorContext?.toWellFormed()??null,detail:f.errorDetail,severity:f.severity,
  ...(f.errorContext&&!f.errorContext.isWellFormed()?{contextUtf16:Array.from({length:f.errorContext.length},(_,i)=>f.errorContext.charCodeAt(i))}:{}),
}));}
const failures=[];
const builtin=rule==="MD039"?(await import("./node_modules/markdownlint/lib/md039.mjs")).default:null;
const rows=samples.flatMap((input,sample)=>options.flatMap((options,configuration)=>{
  const id=`${sample}-${configuration}`;
  try{return [{id,input,options,findings:findings(input,options)}];}catch(error){
    if(rule!=="MD039"||error.message!=="Value of 'lineNumber' passed to onError by 'MD039' is incorrect for 'input'.")throw error;
    const raw=[];lint({strings:{input},customRules:[{...builtin,names:["TST039"],function(params){builtin.function(params,f=>raw.push(f));}}],config:{default:false,TST039:options}});
    failures.push({id,input,options,error:error.message,raw});return [];
  }
}));
const receipt=new URL(`${rule.toLowerCase()}-observations.json`,import.meta.url);
const serialized="[\n"+rows.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
if(rule==="MD039"){
  const path=new URL("md039-invalid-lines.json",import.meta.url),text=JSON.stringify(failures,null,2)+"\n";
  if(process.argv.includes("--record"))writeFileSync(path,text);else assert.equal(text,readFileSync(path,"utf8"));
  const policy=["\u00a0","\u2003","\ufeff"].flatMap(space=>[`[${space}label](u)\n`,`[label${space}](u)\n`]).map(input=>{
    try{return {input,findings:findings(input,true)};}catch(error){return {input,error:error.message};}
  });
  const policyPath=new URL("md039-unicode-policy.json",import.meta.url),policyText=JSON.stringify(policy,null,2)+"\n";
  if(process.argv.includes("--record"))writeFileSync(policyPath,policyText);else assert.equal(policyText,readFileSync(policyPath,"utf8"));
}
if(rule==="MD038"){
  const builtin=(await import("./node_modules/markdownlint/lib/md038.mjs")).default;
  const failures=["`\n a \n`\n","`\n  a  \n`\n","> `\n>  a \n> `\n"].map(input=>{
    let error;try{findings(input,true);}catch(thrown){error=thrown.message;}
    assert.equal(error,"Value of 'range' passed to onError by 'MD038' is incorrect for 'input'.");
    const raw=[];lint({strings:{input},customRules:[{...builtin,names:["TST038"],function(params){builtin.function(params,f=>raw.push(f));}}],config:{default:false,TST038:true}});
    return {input,error,raw};
  });
  const path=new URL("md038-invalid-ranges.json",import.meta.url),text=JSON.stringify(failures,null,2)+"\n";
  if(process.argv.includes("--record"))writeFileSync(path,text);else assert.equal(text,readFileSync(path,"utf8"));
}
if(!process.argv.includes("--source-only")){
  const directory=mkdtempSync("/tmp/opencode/mdfmt-inline-whitespace-"),configuration=directory+"/.markdownlint-cli2.jsonc";
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const description={MD037:"Spaces inside emphasis markers",MD038:"Spaces inside code span elements",MD039:"Spaces inside link text"}[rule];
  try{for(const row of rows)for(const fix of [false,true]){
    writeFileSync(configuration,JSON.stringify({fix,config:{default:false,[rule]:row.options}}));
    const native=spawnSync(binary,["-"],{cwd:directory,input:row.input,encoding:"utf8"});
    assert.equal(native.stdout,row.input,`${row.id}: protected source`);
    const pattern=new RegExp(`^-:(\\d+):(\\d+): (Error|Warning) ${rule}: ${description} \\[Context: "(.*)"\\]$`,"gm");
    const actual=[...native.stderr.matchAll(pattern)].map(m=>({line:Number(m[1]),column:Number(m[2]),severity:m[3].toLowerCase(),context:m[4]}));
    const expected=row.findings.map(f=>({line:f.line,column:f.range?.[0]??1,severity:f.severity,context:f.context}));
    const order=(a,b)=>a.line-b.line||a.column-b.column||a.context.localeCompare(b.context);
    assert.deepEqual(actual.sort(order),expected.sort(order),`${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,`${row.id}: ${native.stderr}`);
    if(fix){const again=spawnSync(binary,["-"],{cwd:directory,input:native.stdout,encoding:"utf8"});assert.equal(again.stdout,native.stdout);assert.equal(again.stderr,native.stderr);assert.equal(again.status,native.status);}
  }}finally{unlinkSync(configuration);rmdirSync(directory);}
}
console.log(`PASS: ${rows.length} pinned ${rule} observations${process.argv.includes("--source-only")?" (native implementation not checked)":" with native diagnostics, exact source and idempotence comparisons"}`);
