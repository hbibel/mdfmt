import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";
import { parse } from "./node_modules/markdownlint/lib/micromark-parse.mjs";

const combined = process.argv.includes("--combined");
const rule = combined ? "combined" : process.argv[process.argv.indexOf("--rule") + 1];
assert.ok(["MD049", "MD050", "combined"].includes(rule), "Use --rule MD049 or --rule MD050 or --combined");
let samples = [
  "", "plain text\n", "_one_ and *two*\n", "*one* and _two_\n",
  "__one__ and **two**\n", "**one** and __two__\n", "***both***\n", "___both___\n",
  "**_both_**\n", "__*both*__\n", "*a _b_ c*\n", "_a *b* c_\n", "**a __b__ c**\n",
  "__a **b** c__\n", "a*one*b\n", "a**one**b\n", "é*one*é\n", "é**one**é\n",
  "😀*one*😀\n", "😀**one**😀\n", "_*one*_\n", "*one*_*two*_\n", "_one_**two**\n",
  "😀_one_😀\n", "😀__one__😀\n", "😀_one_ and *two*\n", "_one_😀 and *two*\n",
  "_😀_ and *two*\n", "__😀__ and **two**\n", "𐀀*one*𐀀\n", "𐀀_one_𐀀\n",
  "****four****\n", "*****five*****\n", "******six******\n", "_a__b_\n", "*a**b*\n",
  "*a _b* c_\n", "**a __b** c__\n", "\\_literal\\_ and *real*\n",
  "&#95;literal&#95; and _real_\n", "*first\nsecond*\n", "__first\nsecond__\n",
  "*first\r\nsecond*\r\n", "\uFEFF_one_ and *two*\r\n", "_one_\r*two*\r",
  "# _one_ and *two*!\n", "_one_ and *two*\n===\n", "> _one_ and *two*\n",
  "- _one_ and *two*\n", "> - __one__ and **two**\n", "-\t_one_\n  *two*\n",
  ">\t_one_ and *two*\n", "> *first\n> second*\n", "- *first\n  second*\n",
  "[a _b_](url) and *two*\n", "[a **b**](url) and __two__\n",
  "![a _b_](img) and *two*\n", "[_one_][id] and *two*\n\n[id]: url\n",
  "[_one_] and *two*\n", "[id]: url '_title_'\n\n*one*\n",
  "| _one_ | *two* |\n| --- | --- |\n", "| __one__ | **two** |\n| --- | --- |\n",
  "`_one_ and *two*`\n", "```md\n_one_ and *two*\n```\n", "    _one_ and *two*\n",
  "$*math*$ and _real_\n", "$$\n_one_ and *two*\n$$\n",
  "<div>\n_one_ and *two*\n</div>\n", "<span>_one_ and *two*</span>\n",
  "- _one_\n<span>\n*two*\n", "<!-- _hidden_ and *hidden* -->\n_one_ and *two*\n",
  "<!-- unfinished\n_one_ and *two*\n", "_one_ <!-- 😀 --> *two*\n",
  "---\ntitle: _one_ and *two*\n---\n_one_ and *two*\n",
  "+++\ntitle = '_one_'\n+++\n_one_ and *two*\n",
  `<!-- markdownlint-disable-next-line ${rule} -->\n_one_ __one__\n\n*two* **two**\n`,
  `*first <!-- markdownlint-disable-next-line ${rule} -->\nsecond*\n`,
  `**first <!-- markdownlint-disable-next-line ${rule} -->\nsecond**\n`,
  `<!-- markdownlint-disable-file ${rule} -->\n_one_ and *two*; __one__ and **two**\n`,
  "_one_ and *a long emphasized phrase with many words to exercise wrapping*\n",
];
for (const marker of ["*", "**", "_", "__"]) {
  for (const [left, right] of [["a", "a"], ["0", "_"], ["é", "é"], ["中", "中"], ["-", "."], ["", ""], [" ", " "]]) {
    samples.push(`${left}${marker}text${marker}${right}\n`);
  }
}
for(const prefix of ["","a","é","😀","!"])for(const body of ["***one***","***one* two**","**one *two***","*__one__*","**_one_ two**","__one__****two**","_a 😀_b_","_a _b_ c_","****one***","***one****","**one****two**","*one***two*","_one___two__","__one___two_","***one**two*","_!__x__!_"]) samples.push(`${prefix}${body}\n`);
for(const body of ["***one***","***one* two**","_a 😀_b_","_!__x__!_","**!x!*","*!x!**"])for(const wrap of [s=>`[${s}](url)\n`,s=>`![${s}](img)\n`,s=>`| ${s} |\n| --- |\n`,s=>`|${s}|\n|---|\n`,s=>`> ${s}\n`,s=>`>${s}\n`]) samples.push(wrap(body));
samples.push("![[a][*id*]](img)\n\n[*id*]: /u\n\n_one_\n","![[a][__id__]](img)\n\n[__id__]: /u\n\n**one**\n","> ![![a\n> _b_](inner)](outer)\n\n*two*\n","> ![![a\n> __b__](inner)](outer)\n\n**two**\n",`<!-- markdownlint-disable-next-line ${rule} -->\n_a 😀_b_\n\n*two*\n`);
let options = [true, false, {style:"asterisk"}, {style:"underscore"}, {severity:"warning"}];
if(combined) {
  samples=["***both***\n","___both___\n","**_both_**\n","__*both*__\n","*a _b_ c* and __d **e** f__\n","_one_ **two** *three* __four__\n","😀___both___😀\n","😀***both***😀\n","😀__one__😀 and *two*\n","*one*_*two*_\n","_one_**two**\n","[a ___b___](url) and ***two***\n","> ![a __b__\n> _c_](img) and ***two***\n","| ___one___ | ***two*** |\n| --- | --- |\n","_one_ and **two**\n\n*three* and __four__\n","\uFEFF___one___ and ***two***\r\n","> - ___one___ and ***two***\n","<span>___one___ and ***two***</span>\n","<!-- markdownlint-disable-next-line MD049 MD050 -->\n___one___\n\n***two***\n","***first <!-- markdownlint-disable-next-line MD050 -->\nsecond***\n"];
  options=["consistent","asterisk","underscore"].flatMap(emphasis=>["consistent","asterisk","underscore"].map(strong=>({MD049:{style:emphasis},MD050:{style:strong}})));
}
const settings = options => combined ? {default:false,...options} : {default:false,[rule]:options};
function findings(input, options) {
  return lint({strings:{input},config:settings(options)}).input.map(f => ({
    ...(combined?{code:f.ruleNames[0]}:{}),
    line:f.lineNumber, column:f.errorRange?.[0] ?? 1, length:f.errorRange?.[1] ?? 0,
    detail:f.errorDetail, severity:f.severity,
  }));
}
const rows = samples.flatMap((input, sample) => options.map((options, configuration) => ({
  id:`${sample}-${configuration}`, input, options, findings:findings(input, options),
})));
const receipt = new URL(combined ? "emphasis-combined-observations.json" : `${rule.toLowerCase()}-observations.json`, import.meta.url);
const serialized = "[\n" + rows.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
if (process.argv.includes("--source-only")) {
  console.log(`PASS: ${rows.length} pinned ${rule} observations (native implementation not checked)`);
} else {
  const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
  const directory = mkdtempSync("/tmp/opencode/mdfmt-emphasis-oracle-");
  const configuration = directory + "/.markdownlint-cli2.jsonc";
  const sourceView = input => input.replace(/^\uFEFF/, "").replace(/\r\n?/g, "\n");
  function snapshot(input) {
    const view = token => [token.type,
      ["emphasisSequence", "strongSequence"].includes(token.type) ? token.text.length :
        token.children.length ? token.children.map(view) : token.text];
    return parse(sourceView(input)).map(view);
  }
  function protectedSource(input) {
    const result=[];
    const visit=tokens=>{for(const token of tokens){
      if(["link","image","codeText","codeFenced","codeIndented","table","htmlFlow","htmlText"].includes(token.type)) result.push([token.type,token.text]);
      else visit(token.children);
    }};
    visit(parse(sourceView(input))); return result;
  }
  try { for (const row of rows) for (const fix of [false, true]) {
    writeFileSync(configuration, JSON.stringify({fix,config:settings(row.options)}));
    const native=spawnSync(binary,["-"],{input:row.input,encoding:"utf8",cwd:directory});
    assert.equal(native.error,undefined,row.id);
    if(!fix) assert.equal(native.stdout,row.input,row.id);
    const expected=findings(native.stdout,row.options);
    const description=rule==="MD049"?"Emphasis style":"Strong style";
    const pattern=combined ? /^-:(\d+):(\d+): (Error|Warning) (MD049|MD050): (?:Emphasis|Strong) style \[([^\r\n]*)\]$/gm : new RegExp(`^-:(\\d+):(\\d+): (Error|Warning) ${rule}: ${description} \\[([^\\r\\n]*)\\]$`,"gm");
    const actual=[...native.stderr.matchAll(pattern)].map(m=>({line:Number(m[1]),column:Number(m[2]),detail:m[combined?5:4],severity:m[3].toLowerCase(),...(combined?{code:m[4]}:{})}));
    const order=(a,b)=>a.line-b.line||a.column-b.column||(a.code??"").localeCompare(b.code??"");
    assert.deepEqual(actual.sort(order),expected.map(({length,...f})=>f).sort(order),`${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status,expected.some(f=>f.severity==="error")?1:0,`${row.id}: ${native.stderr}`);
    if(fix){
      assert.deepEqual(snapshot(native.stdout),snapshot(row.input),`${row.id}: inline structure/content`);
      assert.deepEqual(protectedSource(native.stdout),protectedSource(row.input),`${row.id}: protected source`);
      const second=spawnSync(binary,["-"],{input:native.stdout,encoding:"utf8",cwd:directory});
      assert.equal(second.stdout,native.stdout,`${row.id}: idempotence`);
      assert.equal(second.stderr,native.stderr,`${row.id}: stable diagnostics`);
      assert.equal(second.status,native.status,`${row.id}: stable status`);
    }
  }} finally {unlinkSync(configuration); rmdirSync(directory);}
  console.log(`PASS: ${rows.length} pinned ${rule}/native diagnostic, structure/content, protected-source and idempotence comparisons`);
}
