import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
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
console.log(`PASS: ${observations.length} pinned MD029 observations (native implementation pending)`);
