import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  "1. one\n3. two\n5. three\n", "1. one\n1. two\n3. three\n", "2. one\n4. two\n6. three\n",
  "0. one\n2. two\n4. three\n", "0. one\n1. two\n2. three\n", "0. one\n0. two\n0. three\n",
  "1. one\n1. two\n1. three\n", "1. one\n", "2. one\n", "0. one\n", "1) one\n3) two\n",
  "01. one\n03. two\n05. three\n", "001. one\n003. two\n005. three\n", "000. one\n002. two\n004. three\n",
  " 9. one\n10. two\n12. three\n", "9. one\n10. two\n12. three\n", "1.  one\n3. two\n5.   three\n",
  "1. one\n   1. child\n   3. child\n4. two\n", "- one\n  2. child\n  4. child\n- two\n",
  "> 2. one\n> 4. two\n", "- > 1. one\n  > 3. two\n", "1. one\n\n3. two\n\n5. three\n",
  "1. one\n\n10. two\n    continuation\n", "1. one\n\n10. two\n\n        literal\n",
  "1. one\n\n10. two\n\n    ```\n    code\n    ```\n", "3.   \n    continuation\n5. more\n",
  "1. one\n3. \n5. three\n", "1.\tone\n3.\ttwo\n", ">\t1. one\n>\t3. two\n",
  "\uFEFF1. 😀 one\r\n3. two\r\n", "1. one\r3. two\r", "```\n1. one\n3. two\n```\n",
  "    1. one\n    3. two\n", "| 1. one |\n| --- |\n| 3. two |\n",
  "<!-- markdownlint-disable-next-line MD029 -->\n2. one\n4. two\n", "1. one\n   <!-- markdownlint-disable-next-line MD029 -->\n3. two\n5. three\n",
  "<!-- markdownlint-disable-file ol-prefix -->\n1. one\n3. two\n", "999999999. one\n999999999. two\n",
  "\uFEFF009. \r\n999999999.item1\r\n10.    item2\r\n01.\r\n2.    \r\n",
  "9.\ntext\n2. item\n4. item\n", "> 9.\n> text\n> 2. item\n> 4. item\n",
  "9.\ntext\n1. item\n3. item\n", "9.\ntext\n\n2. item\n4. item\n",
];
const configurations = [true,false,{style:"one"},{style:"ordered"},{style:"zero"},{style:"one_or_ordered"},{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD029:options}}).input;
  const bom=input.startsWith("\uFEFF")?"\uFEFF":"";
  return {id:`${sample}-${configuration}`,input,options,
    findings:findings.map(finding=>({line:finding.lineNumber,column:finding.errorRange[0],length:finding.errorRange[1],
      detail:finding.errorDetail,severity:finding.severity})),
    output:bom+applyFixes(input.slice(bom.length),findings)};
}));
const receipt=new URL("md029-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);
else assert.equal(serialized,readFileSync(receipt,"utf8"));
function boundaries(input) {
  let result=[];
  const types=new Set(["blockQuote","listOrdered","listUnordered","listItemPrefix","paragraph","atxHeading","setextHeading","codeFenced","codeIndented","gfmTable","codeFlowValue","codeFencedFenceInfo","codeFencedFenceMeta"]);
  lint({strings:{input},config:{default:false,capture_ordered:true},customRules:[{
    names:["capture_ordered"],description:"Capture rendered ordered starts and protected code",tags:["observation"],parser:"micromark",
    function(params){
      const visit=tokens=>{for(const token of tokens){
        if(types.has(token.type)){
          const parents=[];for(let parent=token.parent;parent;parent=parent.parent)parents.push(parent.type);
          const firstPrefix=token.type==="listOrdered"?token.children.find(child=>child.type==="listItemPrefix"):null;
          const value=firstPrefix?.children.find(child=>child.type==="listItemValue");
          result.push({type:token.type,parents,start:value?Number(value.text):null,
            literal:["codeFlowValue","codeFencedFenceInfo","codeFencedFenceMeta"].includes(token.type)?token.text:null});
        }
        visit(token.children);
      }};
      visit(params.parsers.micromark.tokens);
    },
  }]});
  return result;
}
if(process.argv.includes("--source-only")) {
  console.log(`PASS: ${observations.length} pinned MD029 observations (native implementation pending)`);
} else {
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md029-oracle-");
  const configuration=directory+"/.markdownlint-cli2.jsonc";
  let preservedDifferences=0;
  try {for(const row of observations){for(const fix of [false,true]){
    writeFileSync(configuration,JSON.stringify({fix,config:{default:false,MD029:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});
    if(!fix)assert.equal(native.stdout,row.input,row.id);
    const expected=lint({strings:{input:native.stdout},config:{default:false,MD029:row.options}}).input
      .map(finding=>({line:finding.lineNumber,column:finding.errorRange[0],detail:finding.errorDetail,severity:finding.severity}));
    const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD029: Ordered list item prefix \[([^\r\n]*)\]$/gm)]
      .map(match=>({line:Number(match[1]),column:Number(match[2]),detail:match[4],severity:match[3].toLowerCase()}));
    assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status,expected.some(finding=>finding.severity==="error")?1:0,native.stderr);
    if(fix){
      if(native.stdout!==row.output)preservedDifferences++;
      assert.deepEqual(boundaries(native.stdout),boundaries(row.input),`${row.id}: rendered starts/code/container ownership`);
      const second=spawnSync(binary,["-"],{input:native.stdout,encoding:"utf8",cwd:directory});
      assert.equal(second.stdout,native.stdout,`${row.id}: idempotence`);
      assert.equal(second.status,native.status,`${row.id}: stable status`);
    }
  }}} finally {unlinkSync(configuration);rmdirSync(directory);}
  console.log(`PASS: ${observations.length} pinned MD029/native diagnostic, rendered-start/code preservation and idempotence cases; ${preservedDifferences} preserved upstream-fix differences`);
}
