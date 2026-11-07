import assert from "node:assert/strict";
import {readFileSync, writeFileSync, mkdtempSync, rmSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";

const samples = [
  "", "\n", "# Title\n", "### Initial\n", "# One\n### Three\n## Two\n#### Four\n",
  "# First\n# Second\n", "preamble\n\n# First\n# Second\n", "## Before\n# First\n# Second\n",
  "<!-- comment -->\n\n# First\n# Second\n", "<!-- comment --> visible\n# First\n# Second\n",
  "[a]: /url\n\n# First\n# Second\n", "> # First\n> ### Third\n\n# Second\n",
  "- # First\n- ### Third\n\n# Second\n", "# [Linux](#tab/linux)\n# First\n# Second\n",
  "# First\n# [Linux](#tab/linux)\n# Second\n", "Title\n===\n\nSubtitle\n---\n",
  "> A  \n> B\n> ===\n\n### Three\n", "# <b>Title</b>\n# Second\n",
  "<h1>HTML title</h1>\n\n# Second\n", "<h2>HTML title</h2>\n\n# Second\n",
  "<div>\n# Not a title\n</div>\n\n# First\n# Second\n", "````\n# code\n````\n\n# First\n# Second\n",
  "<!-- markdownlint-disable-next-line MD001 MD025 MD041 MD043 -->\n# One\n### Three\n# Two\n",
  "# One\n<!-- markdownlint-disable-next-line MD001 -->\n### Three\n##### Five\n",
  "<!-- markdownlint-disable-file headings -->\n# One\n### Three\n# Two\n",
  "\uFEFF# 😀 First\r\n### Third\r\n# Second\r\n", "# First\r### Third\r# Second",
  "# A\tB\n### Third\n# Second\n", "# A\n## B\n## C\n# D\n", "# A\n# B\n# C\n",
  "# A\n## B\n# A\n## B\n# D\n", "# Very long heading context with astral 😀 and other text\n# Another very long heading context with astral 😀\n",
  "# First\n\n$$\n# Math\n### Math\n$$\n\n### Third\n# Second\n",
  "- item\n<span>\n# First\n### Third\n# Second\n", "# First\n\n> 1.\t# Nested\n\n# Second\n",
  "<!-->\n# First\n# Second\n", "<!-- a --> <!-- b -->\n# First\n# Second\n", "<h1/>\n# First\n# Second\n",
  "#\n#\n", "---\ntitle: x\n---", "# [Linux](#tab/linux) #\n# First\n# Second\n",
  " <h1>Title</h1>\n", "  <h1>Title</h1>\n", "   <h1>Title</h1>\n",
  " # [Tab](#tab/linux)\n# A\n# B\n", "  # [Tab](#tab/linux)\n# A\n# B\n", "# [Tab](#t&#97;b/linux)\n# A\n# B\n",
  "# [Tab](<#tab/linux>)\n# A\n# B\n", "# [Tab](  #tab/linux  )\n# A\n# B\n",
  "- item\n<span>\nA\nB\n===\n### C\n",
  "# First\n# # Title\n", "# First\n# Title # #\n",
  "# # [Tab](#tab/linux)\n# A\n# B\n",
];
for (const metadata of ["---\ntitle: x\n---\n", "---\n\"TiTlE\": x\n---\n", "+++\nTITLE = 'x'\n+++\n", "{\n\"title\": \"x\"\n}\n", "---\nnot_title: x\n---\n"]) {
  for (const heading of ["# First\n# Second\n", "### Third\n"]) samples.push(metadata+heading);
}
const options = {
  MD001: [true, false, {severity:"warning"}, {front_matter_title:""}, {front_matter_title:"^\\s*title\\s*[:=]"}],
  MD025: [true, false, {severity:"warning"}, {level:2}, {front_matter_title:""}],
  MD041: [true, false, {severity:"warning"}, {level:2}, {allow_preamble:true}, {front_matter_title:""}],
  MD043: [true, false, {headings:[]}, {headings:["# A","## B","*","# D"]}, {headings:["?","+"]}, {headings:["*","# D"]}, {headings:["^","# A","## B","^","# D"]}, {headings:["# TITLE"],match_case:true}, {headings:["# A B","### C"]}],
};
const observations = Object.entries(options).flatMap(([rule, configurations])=>samples.flatMap((input, sample)=>configurations.map((options, configuration)=>{
  const row={id:`${rule}-${sample}-${configuration}`,rule,input,options};
  const project=f=>({line:f.lineNumber,context:f.errorContext??null,detail:f.errorDetail??null,severity:f.severity});
  try {return {...row,findings:lint({strings:{input},config:{default:false,[rule]:options}}).input.map(project)};}
  catch(error) {
    assert.equal(input,"---\ntitle: x\n---");
    assert.equal(rule,"MD043");
    assert.match(error.message,/Cannot read properties of undefined/);
    return {...row,upstreamError:error.message,findings:lint({strings:{input:""},config:{default:false,[rule]:options}}).input.map(f=>({...project(f),line:3}))};
  }
})));
for(const [input,required] of [["# # Title\n","# # Title"],["# Title # #\n","# Title #"],["# # #\n","# #"]]) {
  const options={headings:[required]};
  observations.push({id:`literal-${required}`,rule:"MD043",input,options,
    findings:lint({strings:{input},config:{default:false,MD043:options}}).input.map(f=>({line:f.lineNumber,context:f.errorContext??null,detail:f.errorDetail??null,severity:f.severity}))});
}
const receipt = new URL("heading-structure-observations.json", import.meta.url);
const serialized = "[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record")) writeFileSync(receipt,serialized);
else assert.equal(serialized,readFileSync(receipt,"utf8"));
if (!process.argv.includes("--source-only")) {
  const directory=mkdtempSync("/tmp/opencode/mdfmt-heading-oracle-");
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  try { for(const row of observations) {
    writeFileSync(directory+"/.markdownlint-cli2.jsonc",JSON.stringify({config:{default:false,[row.rule]:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});
    assert.equal(native.stdout,row.input,row.id);
    const actual=[...native.stderr.matchAll(/^-:(\d+):\d+: (Error|Warning) (MD\d+): (.*)$/gm)].map(m=>({line:Number(m[1]),severity:m[2].toLowerCase(),rule:m[3],message:m[4]}));
    const descriptions={MD001:"Heading levels should only increment by one level at a time",MD025:"Multiple top-level headings in the same document",MD041:"First line in a file should be a top-level heading",MD043:"Required heading structure"};
    assert.deepEqual(actual,row.findings.map(f=>({line:f.line,severity:f.severity,rule:row.rule,message:descriptions[row.rule]+(f.detail?` [${f.detail}]`:"")+(f.context?` [Context: "${f.context}"]`:"")})),row.id);
    assert.equal(native.status,row.findings.some(f=>f.severity==="error")?1:0,row.id);
  } } finally {rmSync(directory,{recursive:true});}
}
console.log(`PASS: ${observations.length} pinned heading-structure observations${process.argv.includes("--source-only")?"":" and native CLI comparisons"}`);
