import assert from "node:assert/strict";
import {readFileSync,writeFileSync} from "node:fs";
import {lint} from "markdownlint/sync";

const samples=[
  ["[text][id]\n\n[id]: /url\n",{full:false}],
  ["[text][ID]\n\n[id]: /url\n",{full:false}],
  ["[TEXT][]\n\n[text]: /url\n",{collapsed:false}],
  ["[TEXT]\n\n[text]: /url\n",{shortcut:false}],
  ["[text]()\n\n[text]: /url\n",{inline:false}],
  ["[text]( \"title\")\n\n[text]: /url\n",{inline:false}],
  ["[label](https://example.com \"title\")\n",{inline:false}],
  ["[a *b*][id]\n\n[id]: /url\n",{full:false}],
  ["[A &amp; B][]\n\n[A &amp; B]: /url\n",{collapsed:false}],
];
const rows=samples.map(([input,options])=>({input,options,findings:lint({strings:{input},config:{default:false,MD054:options}}).input.map(f=>({line:f.lineNumber,range:f.errorRange,context:f.errorContext,detail:f.errorDetail,fix:f.fixInfo}))}));
const path=new URL("link-style-audit-observations.json",import.meta.url),text=JSON.stringify(rows,null,2)+"\n";
if(process.argv.includes("--record"))writeFileSync(path,text);else assert.equal(text,readFileSync(path,"utf8"));
console.log(`PASS: ${rows.length} pinned MD054 source-only observations (no native implementation checked)`);
