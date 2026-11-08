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
function findings(input,options){return lint({strings:{input},config:{default:false,[rule]:options}}).input.map(f=>({
  line:f.lineNumber,range:f.errorRange,context:f.errorContext?.toWellFormed()??null,detail:f.errorDetail,severity:f.severity,
  ...(f.errorContext&&!f.errorContext.isWellFormed()?{contextUtf16:Array.from({length:f.errorContext.length},(_,i)=>f.errorContext.charCodeAt(i))}:{}),
}));}
const rows=samples.flatMap((input,sample)=>options.map((options,configuration)=>({id:`${sample}-${configuration}`,input,options,findings:findings(input,options)})));
const receipt=new URL(`${rule.toLowerCase()}-observations.json`,import.meta.url);
const serialized="[\n"+rows.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
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
