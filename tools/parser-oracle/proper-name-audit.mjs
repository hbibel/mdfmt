import assert from "node:assert/strict";
import {readFileSync,writeFileSync} from "node:fs";
import {lint} from "markdownlint/sync";

const samples=["github\n","`https://example.com github`\n","```text\nhttps://example.com github\n```\n","`github https://example.com github`\n","<span>https://example.com github</span>\n","<div>\nhttps://example.com github\n</div>\n"];
const rows=samples.map(input=>({input,options:{names:["GitHub"]},findings:lint({strings:{input},config:{default:false,MD044:{names:["GitHub"]}}}).input.map(f=>({line:f.lineNumber,range:f.errorRange,detail:f.errorDetail}))}));
const path=new URL("proper-name-audit-observations.json",import.meta.url),text=JSON.stringify(rows,null,2)+"\n";
if(process.argv.includes("--record"))writeFileSync(path,text);else assert.equal(text,readFileSync(path,"utf8"));
console.log(`PASS: ${rows.length} pinned MD044 source-only observations (no native implementation checked; unsafe empty-name probe is not rerun)`);
