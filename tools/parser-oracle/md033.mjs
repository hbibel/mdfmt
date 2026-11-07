import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";

const samples=[
  "<b>x</b>\n", "word <B>x</B> and <i/>\n", "\uFEFF😀 <b title='😀'>x</b>\r\n",
  "<!-- <b>hidden</b> -->\n", "<!-- open\n<b>\n", "<?php?>\n", "<![CDATA[<b>]]>\n", "<!DOCTYPE html>\n",
  "<https://example.test>\n", "mail <user@example.com>\n", "`<b>`\n", "```\n<b>\n```\n", "    <b>\n",
  "$<b>$\n", "$$\n<b>\n$$\n", "<script>\nconst x = '<b>';\n</script>\n", "<style>\n<b>\n</style>\n",
  "<div>\n`<b>`\n<i>\n</div>\n", "<div>\n| h |\n| --- |\n| <b> |\n</div>\n",
  "> <b>x</b>\n", "- <b>x</b>\n", "> - <b>x</b>\n", "- item\n<span>\n<b>x</b>\n", "> 1.\t<b>x</b>\n",
  "<b\n title='x'>y</b>\n", "> <b\n> title='x'>y</b>\n", "- <b\n  title='x'>y</b>\n",
  "![<b>alt</b>](img)\n", "> ![x\n> <b>alt</b>](img)\n", "![x ![<b>alt</b>](inner)](img)\n",
  "![<b>alt</b>][ref]\n\n[ref]: img\n", "> ![<b>alt</b>][ref]\n>\n> [ref]:\n>   img\n",
  "| a |\n| --- |\n| <b>x</b> |\n", "> | <b>x</b> |\n> | --- |\n", "- | a |\n  | --- |\n  | ![<b>alt</b>](img) |\n",
  "[<b>label</b>](url)\n", "# # <b>x</b>\n", "<!-- x -- y <b> -->\n", "x <!-- x -- y <b> -->\n",
  '<b title="<!-- " -->">x</b>\n', 'x <b title="<!-- " -->">x</b>\n',
  "---\ntitle: '<b>'\n---\nText\n", "+++\ntitle='<b>'\n+++\nText\n", '{\n"title":"<b>"\n}\nText\n',
  "---\n<!--\n---\n-->\n<b>x</b>\n", "<!-- markdownlint-disable-next-line MD033 -->\n<b>\n<i>\n",
  "<!-- markdownlint-disable-file html -->\n<b>\n", "<!-- markdownlint-disable MD033 -->\n<b>\n<!-- markdownlint-enable MD033 -->\n<i>\n",
  "# Heading!\n\n\n<b>x</b>\n", "<B>x</B>\r", "<b>x</b>", "<img src='x' alt='y'>\n", "<input disabled>\n",
  "<custom:tag A='x'>\n", "<custom-element>\n", "<b/>\n", "</b>\n", "~~~\n<b>\n~~~\n",
  "\uFEFF---\ntitle: '<b>'\n---\n<b>\n", "![a | <b>\n--- | ---](img)\n", "- ![a | <b>\n  --- | ---](img)\n",
  '![<b title="]">alt</b>](img)\n', "![`<b>`](img)\n", "![$<b>$](img)\n", "![x\n`<b>`](img)\n",
  "<b>ordinary</b>\n\n| h |\n| --- |\n| <b>table</b> |\n", "<!-- 😀 --> <b>x</b>\n",
  "<!--> <b> -->\n", "<!---> <b> -->\n", "<!-- <b> --->\n", "x <!-- x -- y <b> -->\n",
  "<!-- <b>\n<i> --> <em>\n", "<!-- <b>\n<i> -->\n<em>\n", "<!-- 😀 -->\r<b>\r",
  "<div>\n$<b>$\n</div>\n", "<div>\n$$\n<b>\n$$\n</div>\n",
  "![x\n\n<b>](img)\n", "![![<b>][r]][r]\n\n[r]: img\n",
  "| h |\n| --- |\n| ![<b>](img) |\n", "| <!-- <b> --> |\n| --- |\n",
  "> <!-- <b> --> <i>\n", "- <!-- <b> --> <i>\n", "\t<b>\n",
  "<!-- markdownlint-disable-next-line no-inline-html -->\n<b>\n<i>\n",
  "<div>\n![<b>](img)\n</div>\n", "<b title='😀\nvalue'>x</b>\n",
];
const options=[true,false,{severity:"warning"},{allowed_elements:["B","SPAN","img"]},
  {allowed_elements:["b"],table_allowed_elements:[]},{table_allowed_elements:["b"]},
  {allowed_elements:["span"],table_allowed_elements:["b","em"]}];
const observations=samples.flatMap((input,sample)=>options.map((options,configuration)=>{
  const id=`${sample}-${configuration}`;
  try{return {id,input,options,findings:lint({strings:{input},config:{default:false,MD033:options}}).input.map(f=>({line:f.lineNumber,range:f.errorRange??null,detail:f.errorDetail??null,severity:f.severity}))};}
  catch(error){throw new Error(`${id}: ${JSON.stringify(input)}`,{cause:error});}
}));
const receipt=new URL("md033-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
if(!process.argv.includes("--source-only")){
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md033-oracle-");
  try{for(const row of observations){for(const fix of [false,true]){
    writeFileSync(directory+"/.markdownlint-cli2.jsonc",JSON.stringify({fix,config:{default:false,MD033:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});
    assert.equal(native.stdout,row.input,row.id);
    const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD033: Inline HTML \[(.*)\]$/gm)].map(m=>({line:Number(m[1]),column:Number(m[2]),detail:m[4],severity:m[3].toLowerCase()}));
    assert.deepEqual(actual,row.findings.map(f=>({line:f.line,column:f.range?.[0]??1,detail:f.detail,severity:f.severity})),`${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status,row.findings.some(f=>f.severity==="error")?1:0,row.id);
  }}}finally{rmSync(directory,{recursive:true});}
}
console.log(`PASS: ${observations.length} pinned MD033 observations${process.argv.includes("--source-only")?"":" and native range/source-preservation comparisons"}`);
