import assert from "node:assert/strict";
import {readFileSync,writeFileSync} from "node:fs";
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
console.log(`PASS: ${observations.length} pinned MD030 observations (native implementation pending)`);
