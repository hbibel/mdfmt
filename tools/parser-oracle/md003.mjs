import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";
import helpers from "./node_modules/markdownlint/helpers/micromark-helpers.cjs";

function headingIdentity(input) {
  let result=[];
  const capture={names:["capture"],description:"Capture heading identity",tags:["testcapture"],parser:"micromark",
    function(params){result=helpers.filterByTypes(params.parsers.micromark.tokens,["atxHeading","setextHeading"]).map(token=>({level:helpers.getHeadingLevel(token),text:helpers.getHeadingText(token)}));}};
  lint({strings:{input},config:{default:false,capture:true},customRules:[capture],noInlineConfig:true});
  return result;
}

const samples=[
  "# Title\n", "# Title #\n", "Title\n===\n", "Subtitle\n---\n", "### Third\n", "### Third ###\n",
  "# A\n## B ##\n\nC\n===\n", "Title\n===\n\n## Subtitle\n\n### Third ###\n", "A\nB\n===\n", "> A\n> B\n> ===\n",
  "> # Title\n", "> Title\n> ===\n", "- # Title\n", "- Title\n  ===\n", "  # Title\n", "\uFEFF# 😀 Title\r\n",
  "# Title\r", "# Title", "# Title ###\n", "# Title#\n", "#\n", "# #\n", "# ###\n", "# `code` [link](url)\n",
  "# <b>Title</b>\n", "# *Title*\n", "# [label](url \"Title\")\n", "Paragraph\n# Title\n", "# Title\nParagraph\n",
  "| a |\n| --- |\n| # Title |\n", "```\n# Code\n```\n", "---\ntitle: # data\n---\n# Title\n", "$$\n# Math\n$$\n# Title\n",
  "<!-- markdownlint-disable-next-line MD003 -->\n# First #\n# Second\n", "<!-- markdownlint-disable-file MD003 -->\n# Title\n",
  "<!-- markdownlint-disable-next-line MD013 -->\n# Title\n", "Title\n===\n\n# Other\n", "# ![alt](image)\n",
  "- item\n<span>\nA\nB\n===\n### C\n", " # [Tab](#tab/linux)\n## Second ##\n", "> - # Nested\n",
  "# # Title\n", "### # Title\n", "> # # Title\n", "- # # Title\n", "# Title # #\n", "## Title # #\n",
  "Title #\n===\n", "# Title. # #\n", "# \\# Title\n",
];
for(const [first,next] of [["",""],["> ","> "],["- ","  "],["  ","  "]]) {
  for(const text of ["# Title","Title #","Title # #","Title. #","\\# Title","[a](url) #","`code` #"]) {
    samples.push(first+"# "+text+"\n",first+"# "+text+" #\n",first+text+"\n"+next+"===\n");
  }
}
const options=[true,false,...["consistent","atx","atx_closed","setext","setext_with_atx","setext_with_atx_closed"].map(style=>({style})),{style:"atx_closed",severity:"warning"}];
const observations=samples.flatMap((input,sample)=>options.map((options,configuration)=>({
  id:`${sample}-${configuration}`,input,options,
  findings:lint({strings:{input},config:{default:false,MD003:options}}).input.map(f=>({line:f.lineNumber,detail:f.errorDetail,severity:f.severity})),
})));
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
const receipt=new URL("md003-observations.json",import.meta.url);
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
if(!process.argv.includes("--source-only")) {
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md003-oracle-");
  try{for(const row of observations){for(const fix of [false,true]){
    writeFileSync(directory+"/.markdownlint-cli2.jsonc",JSON.stringify({fix,config:{default:false,MD003:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});
    if(!fix)assert.equal(native.stdout,row.input,row.id);
    assert.deepEqual(headingIdentity(native.stdout),headingIdentity(row.input),`${row.id}: heading text/level preservation`);
    const expected=lint({strings:{input:native.stdout},config:{default:false,MD003:row.options}}).input.map(f=>({line:f.lineNumber,detail:f.errorDetail,severity:f.severity}));
    const actual=[...native.stderr.matchAll(/^-:(\d+):1: (Error|Warning) MD003: Heading style \[([^\r\n]*)\]$/gm)].map(m=>({line:Number(m[1]),detail:m[3],severity:m[2].toLowerCase()}));
    assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,row.id);
    if(fix){const second=spawnSync(binary,["-"],{input:native.stdout,encoding:"utf8",cwd:directory});assert.equal(second.stdout,native.stdout,row.id);assert.equal(second.stderr,native.stderr,row.id);}
  }}}finally{rmSync(directory,{recursive:true});}
}
console.log(`PASS: ${observations.length} pinned MD003 observations${process.argv.includes("--source-only")?"":" and native diagnostic/idempotence comparisons"}`);
