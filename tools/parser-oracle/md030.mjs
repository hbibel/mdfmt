import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,unlinkSync,rmdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";
import {applyFixes} from "markdownlint";

const samples=[
  "-  one\n-   two\n", "1.  one\n3.   two\n", "- one\n- two\n", "1) one\n2) two\n",
  "-  one\n   continuation\n-   two\n", "1.  one\n    continuation\n3.   two\n",
  "-  one\n\n-   two\n", "1.  one\n\n3.   two\n", "-   \n-  two\n", "1.   \n3.  two\n",
  "-     literal\n-  two\n", "1.     literal\n3.  two\n", "-\tone\n- \ttwo\n", "1.\tone\n3. \ttwo\n",
  "> -  one\n> -   two\n", "- > -  one\n  > -   two\n", "- one\n  -  child\n  -   child\n- two\n",
  "1. one\n   1.  child\n   3.   child\n3. two\n", "\uFEFF-  😀\r\n-   two\r\n", "-  one\r-   two\r",
  "```\n-  one\n```\n", "    -  one\n", "| -  one |\n| --- |\n| 1.  two |\n",
  "<!-- markdownlint-disable-next-line MD030 -->\n-  one\n-   two\n",
  "<!-- markdownlint-disable-file list-marker-space -->\n-  one\n", "-  one\n\n   ```\n   code\n   ```\n",
  "-  one\n\n       literal\n", "-  # Heading\n-   [link](url)\n", "-  > quote\n   > more\n",
  "9.\ntext\n2.  a\n4.   b\n", "    code\n2.  a\n4.   b\n1. c\n", "text\n> 2.  a\n> 4.   b\n",
  "> - 01.\t\r\n>   01.item1\r\n>   01.  item2\r\n>   01.    \r\n>   01.\titem4\r\n",
];
const configurations=[true,false,{ul_single:2,ol_single:3,ul_multi:4,ol_multi:2},{ul_single:0,ol_single:0,ul_multi:0,ol_multi:0},
  {ul_single:5,ol_single:5,ul_multi:5,ol_multi:5},{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD030:options}}).input;
  const bom=input.startsWith("\uFEFF")?"\uFEFF":"";
  return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,column:f.errorRange[0],length:f.errorRange[1],detail:f.errorDetail,severity:f.severity})),
    output:bom+applyFixes(input.slice(bom.length),findings)};
}));
const receipt=new URL("md030-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);
else assert.equal(serialized,readFileSync(receipt,"utf8"));
function boundaries(input) {
  let result=[];
  const types=new Set(["blockQuote","listOrdered","listUnordered","listItemPrefix","paragraph","atxHeading","setextHeading","codeFenced","codeIndented","gfmTable","codeFlowValue","codeFencedFenceInfo","codeFencedFenceMeta"]);
  lint({strings:{input},config:{default:false,capture_spacing:true},customRules:[{
    names:["capture_spacing"],description:"Capture list spacing ownership and code",tags:["observation"],parser:"micromark",
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
if(process.argv.includes("--source-only"))console.log(`PASS: ${observations.length} pinned MD030 observations`);
else {
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-md030-oracle-"),configuration=directory+"/.markdownlint-cli2.jsonc";
  let differences=0,rejected=0;
  try{for(const row of observations){
    if(row.options.ul_single===0){
      writeFileSync(configuration,JSON.stringify({config:{default:false,MD030:row.options}}));
      const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});
      assert.equal(native.status,2,row.id);assert.match(native.stderr,/violates the pinned rule option schema/);rejected++;continue;
    }
    for(const fix of [false,true]){
      writeFileSync(configuration,JSON.stringify({fix,config:{default:false,MD030:row.options}}));
      const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});
      if(!fix)assert.equal(native.stdout,row.input,row.id);
      const expected=lint({strings:{input:native.stdout},config:{default:false,MD030:row.options}}).input
        .map(f=>({line:f.lineNumber,column:f.errorRange[0],detail:f.errorDetail,severity:f.severity}));
      const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD030: Spaces after list markers \[([^\r\n]*)\]$/gm)]
        .map(m=>({line:Number(m[1]),column:Number(m[2]),detail:m[4],severity:m[3].toLowerCase()}));
      assert.deepEqual(actual,expected,`${row.id}: fix ${fix}: ${native.stderr}`);
      assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,native.stderr);
      if(fix){
        if(native.stdout!==row.output)differences++;
        assert.deepEqual(boundaries(native.stdout),boundaries(row.input),`${row.id}: starts/code/container ownership`);
        const second=spawnSync(binary,["-"],{input:native.stdout,encoding:"utf8",cwd:directory});
        assert.equal(second.stdout,native.stdout,`${row.id}: idempotence`);assert.equal(second.status,native.status,row.id);
      }
    }
  }}finally{unlinkSync(configuration);rmdirSync(directory);}
  console.log(`PASS: ${observations.length-rejected} pinned MD030/native diagnostics, ownership/code/idempotence cases; ${rejected} schema rejections; ${differences} preserved upstream-fix differences`);
}
