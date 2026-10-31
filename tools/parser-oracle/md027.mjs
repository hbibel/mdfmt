import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  ">  text\n", ">   text\n", ">    text\n", ">     code\n", "> text\n", " >  text\n", ">  >  text\n",
  ">  \n", ">   \n", ">\t text\n", "> \ttext\n", ">\t\ttext\n", ">\u00a0 text\n",
  ">  - item\n", "> - item\n>    continuation\n", "> 1. item\n>     continuation\n",
  "- >  item\n", "- > text\n  >   more\n", "> - >  text\n", ">  - >  text\n",
  ">  # Heading\n", ">  Heading\n> ---\n", ">  ```js\n>   code\n>  ```\n",
  "> ```\n>   code\n> ```\n", ">  ```\n>     code\n>  ```\n", ">    code\n>    more\n",
  ">     code\n>      more\n", "```\n>  literal\n```\n", "    >  literal\n",
  "\uFEFF>  😀 text\r\n", ">  one\r>  two\r", "> one\n\n>  two\n",
  "<!-- markdownlint-disable-next-line MD027 -->\n>  text\n", "<!-- markdownlint-disable-file blockquote -->\n>  text\n",
  ">  | a |\n> | - |\n> | b |\n", ">  text with a rather long context that will be truncated\n",
  ">  ~~~\n>   code\n", ">  ~~~\n> \tcode\n> ~~~\n", ">  ~~~\n>\t code\n> ~~~\n",
  ">  ~~~\n>   \n> ~~~\n", ">   ~~~\n>    code\n>  ~~~\n",
  ">  <div>\n>   text\n> </div>\n", ">  <!-- comment\n>   more -->\n",
  ">  [id]: url\n", ">  [id]:\n>    url\n", ">  `a\n>   b`\n",
  "-   \n    >  text\n", "> -   \n>     >  text\n", ">  - item\n>     more\n",
  "> text\n>\t more\n", ">  >\t text\n", ">  1. >  text\n",
  " - >   - item\n   >   continuation\n",
  ">  *a\n>   b*\n", ">  **a\n>   b**\n", ">  [a\n>   b](url)\n", ">  ![a\n>   b](img)\n",
  ">  ~~a\n>   b~~\n", ">  a [b](\n>   url)\n", ">  a <b\n>   x=\"y\">z</b>\n",
  ">  | a |\n>   | - |\n>   | b |\n", "> >  `a\n>  >   b`\n",
  ">  > - text\n", "> > `a\n>   b`\n", "> > <div>\n>   >  more\n", "> > text\n>     >  more\n",
];
const configurations = [true, false, {list_items:false}, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const row={id:`${sample}-${configuration}`,input,options};
  try {
  const findings=lint({strings:{input},config:{default:false,MD027:options}}).input;
  const bom=input.startsWith("\uFEFF")?"\uFEFF":"";
  return {...row,
    findings:findings.map(finding=>({line:finding.lineNumber,column:finding.errorRange[0],length:finding.errorRange[1],
      context:finding.errorContext,severity:finding.severity})),
    output:bom+applyFixes(input.slice(bom.length),findings)};
  } catch(error) {return {...row,error:error.message};}
}));
const receipt = new URL("md027-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
function boundaries(input) {
  let result=[];
  const types=new Set(["blockQuote","listOrdered","listUnordered","listItemPrefix","paragraph","atxHeading","setextHeading","codeFenced","codeIndented","gfmTable","codeFlowValue","codeFencedFenceInfo","codeFencedFenceMeta"]);
  lint({strings:{input},config:{default:false,capture_quote:true},customRules:[{
    names:["capture_quote"],description:"Capture quote ownership and literal code",tags:["observation"],parser:"micromark",
    function(params) {
      const visit=tokens=>{for(const token of tokens){
        if(types.has(token.type)) {
          const parents=[];for(let parent=token.parent;parent;parent=parent.parent)parents.push(parent.type);
          result.push({type:token.type,parents,value:["codeFlowValue","codeFencedFenceInfo","codeFencedFenceMeta"].includes(token.type)?token.text:null});
        }
        visit(token.children);
      }};
      visit(params.parsers.micromark.tokens);
    },
  }]});
  return result;
}
if(process.argv.includes("--source-only")) {
  console.log(`PASS: ${observations.length} pinned MD027 records, including ${observations.filter(row=>row.error).length} upstream runtime errors (native implementation pending)`);
} else {
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md027-oracle-");
  const configuration=directory+"/.markdownlint-cli2.jsonc";
  let preservedDifferences=0;
  try {for(const row of observations){for(const fix of [false,true]){
    writeFileSync(configuration,JSON.stringify({fix,config:{default:false,MD027:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});
    if(!fix)assert.equal(native.stdout,row.input,row.id);
    let expected;
    if(row.error){
      assert.equal(row.input,">  ~~~\n>\t code\n> ~~~\n","New upstream error needs an explicit native regression");
      assert.equal(native.stdout,row.input,row.id);
      expected=[{line:1,column:3,context:">  ~~~",severity:row.options.severity || "error"}];
    } else expected=lint({strings:{input:native.stdout},config:{default:false,MD027:row.options}}).input
      .map(finding=>({line:finding.lineNumber,column:finding.errorRange[0],context:finding.errorContext,severity:finding.severity}));
    const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD027: Multiple spaces after blockquote symbol \[Context: "([^\r\n]*)"\]$/gm)]
      .map(match=>({line:Number(match[1]),column:Number(match[2]),context:match[4],severity:match[3].toLowerCase()}));
    assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status,expected.some(finding=>finding.severity==="error")?1:0,native.stderr);
    if(fix){
      if(!row.error && native.stdout!==row.output)preservedDifferences++;
      assert.deepEqual(boundaries(native.stdout),boundaries(row.input),`${row.id}: quote/list/code ownership`);
      const second=spawnSync(binary,["-"],{input:native.stdout,encoding:"utf8",cwd:directory});
      assert.equal(second.stdout,native.stdout,`${row.id}: idempotence`);
      assert.equal(second.status,native.status,`${row.id}: stable status`);
    }
  }}} finally {unlinkSync(configuration);rmdirSync(directory);}
  const errors=observations.filter(row=>row.error).length;
  console.log(`PASS: ${observations.length-errors} pinned MD027/native diagnostic/ownership/idempotence cases; ${errors} upstream runtime errors handled safely; ${preservedDifferences} preserved upstream-fix differences`);
}
