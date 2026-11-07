import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";
import stringWidth from "string-width";

assert.deepEqual([process.versions.node,process.versions.unicode,process.versions.icu],["24.19.0","17.0","78.3"],"MD060 width baseline changed; audit before updating receipts");
const widths=["","abc","界","😀","👩‍💻","👨‍👩‍👧‍👦","🏳️‍🌈","☀","☀️","☀︎","1️⃣","1⃣","#⃣","🇺🇸","🇦🇦","🇿🇿","🇦","é","́","\u200d","\t","\x1b[31mred\x1b[0m","\x1b]8;;https://example.test\x07link\x1b]8;;\x07","\u009b31mred\u009b0m","\x1b]unterminated","لا","क्ष","क्‍ष","का","각","ᅡ","ᄀ가","ᆨᆨ","ｶﾞ","ᄀ가","\u0600a","·","\u2028","\u2029","\u00ad","🫩","\u{1faea}","\u{1cc00}"];
const widthObservations=widths.map(input=>({input,width:stringWidth(input)}));
for(const input of ["\x1b[٣mred", "\x1b[31:2mred", "\u009d8;;url\u009clink\u009d8;;\u009c", "\x1b]open\x1b[31mred", "\x1b[12345mred", "👩\u200d☀", "☝🏻", "☝🏻́", "☀️́", "a\u093e", "\u302a", "\u302e", "\u115f", "\u1160", "\u{1f3f4}\u{e0067}\u{e0062}\u{e007f}"]) widthObservations.push({input,width:stringWidth(input)});
for(let a=0;a<26;a++)for(let b=0;b<26;b++) {
  const input=String.fromCodePoint(0x1f1e6+a,0x1f1e6+b);
  widthObservations.push({input,width:stringWidth(input)});
}
const samples=[
  "| a |\n| --- |\n| b |\n", "|a|\n|---|\n|b|\n", "| a   | b |\n| --- | --- |\n| c | d   |\n",
  "a | b\n--- | ---\nc | d\n", "| a | b |\n| --- | --- |\n| c |\n| d | e | f |\n",
  "| a | b |\n| --- | --- |\n||\n| | |\n|  |  |\n", "  | a | b |  \n  | --- | --- |  \n  | c | d |  \n",
  "| a | b |\n| --- | --- |\n| `c|d` | e |\n", "| a | b |\n| --- | --- |\n| c\\|d | e |\n",
  "| a | b |\n| --- | --- |\n| c\\\\|d | e |\n", "| a | b |\n| :--- | ---: |\n|c|d|\n",
  "| a | b |\n| --- | --- |\n| c | d\n", "a | b |\n--- | --- |\n| c | d\n",
  "<!-- markdownlint-disable-next-line MD060 -->\n| a |\n| --- |\n| b |\n",
  "| a |\n| --- |\n| b | <!-- markdownlint-disable-line MD060 -->\n",
  "<!-- markdownlint-disable-file table-column-style -->\n| a |\n| --- |\n| b |\n",
  "| <!-- 😀 --> a |\n| --- |\n| b |\n", "<!--\n| a |\n| --- |\n| b |\n-->\n",
  "```\n| a |\n| --- |\n| b |\n```\n", "$$\n| a |\n| --- |\n| b |\n$$\n",
  "| a |\r| --- |\r| b |", "\uFEFF| a |\r\n| --- |\r\n| b |\r\n",
  "---\ntitle: table\n---\n| a |\n| --- |\n| b |\n",
  "|\ta\t|\n| --- |\n|\tb\t|\n", "| a |\n| --- |\n|\t|\n",
];
for(const [first,next]of [["",""],["> ","> "],["- ","  "],["10. ","    "],["> - ",">   "],["- > ","  > "]]) {
  for(const [header,body]of [["| a | b |","| c | d |"],["a | b","c|d"],["| a | b |","`c | d`"],["| a | b |","| | |	"]]) {
    samples.push(`${first}${header}\n${next}| --- | --- |\n${next}${body}\n`);
  }
}
for(const prefix of ["- item\n<span>\n","- > quote\n<span>\n","<div>\n"]) samples.push(`${prefix}| a |\n| --- |\n| b |\n`);
for(const comment of ["<!-- é -->","<!-- 👩‍💻 -->","<!-- \t -->",'<!-- " -->']) samples.push(`| ${comment} |\n| ${"-".repeat(comment.length)} |\n| ${"x".repeat(comment.length)} |\n`);
for(const value of widths.filter(value=>!/[\r\n\u2028\u2029]/u.test(value))) samples.push(`| ${value} |\n| --- |\n| abc |\n`);
const options=[true,false,...["aligned","compact","tight","any"].flatMap(style=>[{style},{style,aligned_delimiter:true}]),{style:"aligned",severity:"warning"}];
const observations=samples.flatMap((input,sample)=>options.map((options,configuration)=>({id:`${sample}-${configuration}`,input,options,findings:lint({strings:{input},config:{default:false,MD060:options}}).input.map(f=>({line:f.lineNumber,range:f.errorRange,detail:f.errorDetail,severity:f.severity}))})));
for(const [name,rows]of [["md060-observations.json",observations],["md060-width-observations.json",widthObservations]]) {
  const receipt=new URL(name,import.meta.url),serialized="[\n"+rows.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
  if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
}
if(!process.argv.includes("--source-only")) {
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md060-oracle-"),binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  try{for(const row of observations){for(const fix of [false,true]){
    writeFileSync(directory+"/.markdownlint-cli2.jsonc",JSON.stringify({fix,config:{default:false,MD060:row.options}}));
    const native=spawnSync(binary,["-"],{cwd:directory,input:row.input,encoding:"utf8"});
    assert.equal(native.stdout,row.input,row.id);
    const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD060: Table column style \[(.*)\]$/gm)].map(m=>({line:Number(m[1]),column:Number(m[2]),detail:m[4],severity:m[3].toLowerCase()}));
    assert.deepEqual(actual,row.findings.map(f=>({line:f.line,column:f.range[0],detail:f.detail,severity:f.severity})),`${row.id}: ${JSON.stringify(row.input)}: ${native.stderr}`);
    assert.equal(native.status,row.findings.some(f=>f.severity==="error")?1:0,row.id);
  }}}finally{rmSync(directory,{recursive:true});}
}
console.log(`PASS: ${observations.length} pinned MD060 cases and ${widthObservations.length} display-width baselines${process.argv.includes("--source-only")?"":" with native comparisons"}`);
