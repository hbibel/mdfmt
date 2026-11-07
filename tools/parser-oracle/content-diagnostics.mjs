import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";

const emphasis=[
  "*Heading*\n", "**Heading**\n", "_Heading_\n", "__Heading__\n", "***Heading***\n",
  "*Sentence.*\n", "**Question?**\n", "*Heading！*\n", "*A &amp; B*\n", "*A\\*B*\n", "*A\nB*\n",
  "*Heading* more\n", "<b>*Heading*</b>\n", "*<b>Heading</b>*\n", "*`Heading`*\n", "*[Heading](url)*\n",
  "> *Heading*\n", "- **Heading**\n", "| header |\n| --- |\n| **Heading** |\n",
  "<div>\n*Heading*\n</div>\n", "<!-- comment -->\n\n*Heading*\n", "# *Heading*\n",
  "*😀 Heading*\n", "\uFEFF**Heading**\r\n", "*Heading*\r\n\r\n**Other**",
  "<!-- markdownlint-disable-next-line MD036 -->\n*Heading*\n\n**Other**\n",
  "<!-- markdownlint-disable-file MD036 -->\n*Heading*\n", "**A**\n\n*B*\n",
  "<div>*Heading*</div>\n", "$$\n\n*Heading*\n\n$$\n", "*A  B*\n", "*A\tB*\n", "  **Heading**\n",
  "- item\n<span>\n*Heading*\n", "*A* <!-- comment -->\n", "*A*\n<!-- comment -->\n",
  "<div>*A\tB*</div>\n", "<div>*AB\tC*</div>\n",
];
const links=[
  "[here](url)\n", "[Click HERE!](url)\n", "[click_here](url)\n", "[more...](url)\n", "[Read documentation](url)\n",
  "[**here**](url)\n", "[*here*](url)\n", "[~~here~~](url)\n", "[`here`](url)\n", "[*`here`*](url)\n",
  "[<b>here</b>](url)\n", "[*<b>here</b>*](url)\n", "[h&#101;re](url)\n", "[he\\re](url)\n",
  "[here][ref]\n\n[ref]: /url\n", "[here][]\n\n[here]: /url\n", "[here]\n\n[here]: /url\n", "[here][missing]\n", "![here](image)\n",
  "[![here](image)](url)\n", "[here\nnow](url)\n", "> [click\n> here](url)\n", "- [click\n  here](url)\n",
  "[here](url) and [more](next)\n", "<https://here>\n", "https://here\n", "<a href='url'>here</a>\n",
  "<div>\n[here](url)\n</div>\n", "| a |\n| --- |\n| [here](url) |\n", "# [here](url)\n", "`[here](url)`\n",
  "[héRé](url)\n", "[😀](url)\n", "\uFEFF😀 [here](url)\r\n", "[CLICK\tHERE](url)\n", "[here](url)\r[more](url)",
  "<!-- markdownlint-disable-next-line MD059 -->\n[here](url)\n[more](url)\n",
  "<!-- markdownlint-disable-file MD059 -->\n[here](url)\n", "[ here ](url)\n", "[](url)\n",
  "![see [here](url)](image)\n", "$$\n[here](url)\n$$\n", "$[here](url)$\n", "[he$re$](url)\n",
  "- item\n<span>\n[here](url)\n", "> 1.\t[here](url)\n", "[here](url \"Title\")\n",
  "> ![x\n> [here](url)](image)\n", "> ![x [here]](image)\n>\n> [here]:\n>   /url\n",
  "> > ![x\n> > [here](url)](image)\n",
];
const configs={MD036:[true,false,{severity:"warning"},{punctuation:""},{punctuation:"?"},{punctuation:"a-z"},{punctuation:"^a-z"},{punctuation:"\\w"}],MD059:[true,false,{severity:"warning"},{prohibited_texts:[]},{prohibited_texts:["HERE","Click here","héRé","😀",""]}]};
const observations=Object.entries(configs).flatMap(([rule,options])=>(rule==="MD036"?emphasis:links).flatMap((input,sample)=>options.map((options,configuration)=>{
  const row={id:`${rule}-${sample}-${configuration}`,rule,input,options};
  try {return {...row,findings:lint({strings:{input},config:{default:false,[rule]:options}}).input.map(f=>({line:f.lineNumber,range:f.errorRange??null,context:f.errorContext??null,severity:f.severity}))};}
  catch(error) {
    assert.equal(input,"[](url)\n");
    assert.match(error.message,/Value of 'range'.*MD059/);
    return {...row,upstreamError:error.message,findings:[{line:1,range:null,context:"[]",severity:"error"}]};
  }
})));
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
const receipt=new URL("content-diagnostics-observations.json",import.meta.url);
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
if(!process.argv.includes("--source-only")) {
  const binary=fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
  const directory=mkdtempSync("/tmp/opencode/mdfmt-content-oracle-");
  try{for(const row of observations){for(const fix of [false,true]){
    writeFileSync(directory+"/.markdownlint-cli2.jsonc",JSON.stringify({fix,config:{default:false,[row.rule]:row.options}}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});
    assert.equal(native.stdout,row.input,row.id);
    const actual=[...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) (MD\d+): (.*)$/gm)].map(m=>({line:Number(m[1]),column:Number(m[2]),severity:m[3].toLowerCase(),rule:m[4],message:m[5]}));
    const description=row.rule==="MD036"?"Emphasis used instead of a heading":"Link text should be descriptive";
    const expected=row.findings.map(f=>({line:f.line,column:f.range?.[0]??1,severity:f.severity,rule:row.rule,message:description+(f.context?` [Context: "${f.context}"]`:"")}));
    assert.deepEqual(actual,expected,`${row.id}: ${native.stderr}`);
    assert.equal(native.status,row.findings.some(f=>f.severity==="error")?1:0,row.id);
  }}}finally{rmSync(directory,{recursive:true});}
}
console.log(`PASS: ${observations.length} pinned content-diagnostic observations${process.argv.includes("--source-only")?"":" and native CLI preservation comparisons"}`);
