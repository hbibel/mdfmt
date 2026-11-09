import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";

const rule=process.argv[process.argv.indexOf("--rule")+1];
assert.ok(["MD052","MD053"].includes(rule),"Use --rule MD052 or MD053");
const inert={names:["TST000"],description:"Populate the pinned parser cache",tags:["test"],parser:"micromark",function(){}};
const options=rule==="MD052"?[true,false,{severity:"warning"},{shortcut_syntax:true},{ignored_labels:[]},{ignored_labels:["MISSING","skip"]},{shortcut_syntax:true,ignored_labels:[]}]:[true,false,{severity:"warning"},{ignored_definitions:[]},{ignored_definitions:["ID","skip"]}];
let samples=[
  "","plain text\n","[label][missing]\n","![label][missing]\n","[missing][]\n","[missing]\n","![missing]\n",
  "[x]\n","[x][]\n","[label][x]\n","[X]\n","[label][skip]\n","[label][MISSING]\n",
  "[label][id]\n\n[id]: /url\n","[label][ID]\n\n[id]: /url\n","[id][]\n\n[id]: /url\n","[id]\n\n[id]: /url\n",
  "[ß]\n\n[SS]: /url\n","[label][ß]\n\n[SS]: /url\n","[label][ missing ]\n","[label][a  b]\n",
  "[label][m\\*issing]\n","[label][m&amp;issing]\n","[label][*missing*]\n","[label][`missing`]\n",
  "[a][b][c]\n","[a][][b][]\n","[a] [b]\n","[a]\n[b]\n","[outer [inner]][missing]\n",
  "\\[label][missing]\n","[label]\\[missing]\n","[label][mis\\]sing]\n","[label][missing\nlabel]\n",
  "[label](url)\n","![label](img)\n","[label]()\n","![label]()\n","<https://example.com>\n",
  "`[label][missing]`\n","```md\n[label][missing]\n```\n","    [label][missing]\n",
  "$[label][missing]$\n","$$\n[label][missing]\n$$\n","<div>\n[label][missing]\n</div>\n",
  "<span>[label][missing]</span>\n","<!-- [label][missing] -->\n","<!-- 😀 --> [label][missing]\n",
  "---\ntitle: [label][missing]\n---\n[label][missing]\n","\uFEFF[label][missing]\r\n",
  "[label][missing]\r\n","[label][missing]\r","[label][😀]\n","[label][\u00a0id\u00a0]\n\n[id]: /url\n",
  "[^missing]\n","[^id]\n\n[^id]: footnote\n","![ [label][missing] ](img)\n",
  "> [label][a\n> b]\n","> ![![ [label][missing] ](inner)](outer)\n",
  "[label][id]\n\n<!-- markdownlint-disable MD052 -->\n[id]: /url\n",
  "<!-- markdownlint-disable-next-line MD052 -->\n[label][missing]\n",
  "[label][missing] <!-- markdownlint-disable-line MD052 -->\n",
  "<!-- markdownlint-disable-file MD052 -->\n[label][missing]\n",
];
for(const body of ["[label][missing]","![label][missing]","[missing][]","[missing]","[label][m&amp;issing]","[label][*missing*]","[label][`missing`]","[a][b][c]","[a] [b]","[a\\]b][missing]","[a][b\nc]","[\nmissing\n]","[label]["+"a".repeat(999)+"]","[label]["+"a".repeat(1000)+"]"]){
  for(const wrap of [s=>`alpha ${s} omega\n`,s=>`> ${s.replaceAll("\n","\n> ")}\n`,s=>`- ${s.replaceAll("\n","\n  ")}\n`,s=>`> - ${s.replaceAll("\n","\n>   ")}\n`,s=>`-\t${s.replaceAll("\n","\n    ")}\n`,s=>`![${s}](img)\n`,s=>`| ${s.replaceAll("\n"," ")} |\n| --- |\n`])samples.push(wrap(body));
}
function findings(input,options,forced){return lint({strings:{input},...(forced?{customRules:[inert]}:{}),config:{default:false,[rule]:options,...(forced?{TST000:true}:{})}}).input.map(f=>({line:f.lineNumber,range:f.errorRange,context:f.errorContext?.toWellFormed()??null,detail:f.errorDetail?.toWellFormed()??null,severity:f.severity}));}
for(const body of ["[a\\\nb]","[a  \nb]","[outer [inner]][missing]","[[a]][b]","[ ][a\nb]","[a][ ]","[a][`x`]","[a][&amp;]","[a][__x__]","[a][~~x~~]","[a][$x$]","[a][<i>x</i>]","[a][b](u)","[b][a][b]","[m\\*id]","[m&amp;id]","[İ]","[ΣΣ]","[label][a\nb]","[a]![b\nc]","[a][b\\\nc]"]){
  for(const wrap of [s=>`${s}\n`,s=>`> ${s.replaceAll("\n","\n> ")}\n`,s=>`> - ${s.replaceAll("\n","\n>   ")}\n`,s=>`![${s}](img)\n`,s=>`[${s}](url)\n`,s=>`${s}\n\n[a]: /a\n[b]: /b\n[m\\*id]: /m\n[m&amp;id]: /m\n`])samples.push(wrap(body));
}
samples.push("\\![missing]\n","\\\\![missing]\n","[a]![b]\n","\\![[a]]\n");
if(rule==="MD053"){
  samples=samples.map(input=>input.replaceAll("MD052","MD053"));
  samples.push("[unused]: /url\n","[id]: /url\n[id]: /url\n\n[text][id]\n","[id]: /one \"first\"\n[id]: /two \"second\"\n","[//]: comment\n","[skip]: /url\n","[ID]: /url\n","[id]:\n  /url\n  \"title\"\n","<!-- 😀 -->\n[id]: /url\n","\uFEFF[id]: /url\r\n","[id]: /url\r","[a\nb]: /url\n","[^id]: footnote\n","[^id]: first\n[^id]: second\n","[^id]\n\n[^id]: footnote\n","[id]: /url\n\n<!-- markdownlint-disable MD053 -->\n[text][id]\n","<!-- markdownlint-disable-next-line MD053 -->\n[id]: /one\n[id]: /two\n","[text][m\\*id]\n\n[m\\*id]: /url\n","[text][m&amp;id]\n\n[m&amp;id]: /url\n");
  for(const definition of ["[id]: /url","[id]: /url\n[id]: /url","[A B]: /url\n[a  b]: /other","[m\\*id]: /url","[m&amp;id]: /url","[^id]: footnote","[id]: /url \""+"😀".repeat(20)+"\"","[id]: /url \"<!-- 😀 -->\""]){
    for(const wrap of [s=>`${s}\n`,s=>`> ${s.replaceAll("\n","\n> ")}\n`,s=>`- ${s.replaceAll("\n","\n  ")}\n`,s=>`> - ${s.replaceAll("\n","\n>   ")}\n`,s=>`-\t${s.replaceAll("\n","\n    ")}\n`,s=>`~~~md\n${s}\n~~~\n`,s=>`---\ntitle: ${s.replaceAll("\n"," ")}\n---\n${s}\n`])samples.push(wrap(definition));
  }
}
const rows=samples.flatMap((input,sample)=>options.map((options,configuration)=>({id:`${sample}-${configuration}`,input,options,direct:findings(input,options,false),parserForced:findings(input,options,true)})));
const path=new URL(`${rule.toLowerCase()}-observations.json`,import.meta.url),text="[\n"+rows.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(path,text);else assert.equal(text,readFileSync(path,"utf8"));
if(!process.argv.includes("--source-only")){
  const directory=mkdtempSync("/tmp/opencode/mdfmt-reference-"),configuration=directory+"/.markdownlint-cli2.jsonc",binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  try{for(const row of rows)for(const fix of [false,true]){
    writeFileSync(configuration,JSON.stringify({fix,config:{default:false,[rule]:row.options}}));
    const native=spawnSync(binary,["-"],{cwd:directory,input:row.input,encoding:"utf8"});assert.equal(native.stdout,row.input,`${row.id}: source`);
    const description=rule==="MD052"?"Reference links and images should use a label that is defined":"Link and image reference definitions should be needed";
    const pattern=new RegExp(`^-:(\\d+):(\\d+): (Error|Warning) ${rule}: ${description} \\[(.*)\\] \\[Context: "(.*)"\\]$`,"gm");
    const actual=[...native.stderr.matchAll(pattern)].map(m=>({line:Number(m[1]),column:Number(m[2]),severity:m[3].toLowerCase(),detail:m[4],context:m[5]}));
    const expected=row.parserForced.map(f=>({line:f.line,column:f.range?.[0]??1,severity:f.severity,detail:f.detail,context:f.context}));
    const order=(a,b)=>a.line-b.line||a.column-b.column||a.detail.localeCompare(b.detail);assert.deepEqual(actual.sort(order),expected.sort(order),`${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,`${row.id}: ${native.stderr}`);
    if(fix){const again=spawnSync(binary,["-"],{cwd:directory,input:native.stdout,encoding:"utf8"});assert.equal(again.stdout,native.stdout);assert.equal(again.stderr,native.stderr);assert.equal(again.status,native.status);}
  }}finally{unlinkSync(configuration);rmdirSync(directory);}
}
console.log(`PASS: ${rows.length} pinned ${rule} direct/parser-forced observations${process.argv.includes("--source-only")?" (native implementation not checked)":" with native diagnostics, exact source and idempotence comparisons"}`);
