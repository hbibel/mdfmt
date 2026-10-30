import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";

const samples = [
  "```js\ncode\n```\n", "~~~js\ncode\n~~~\n", "```\na\n```\n\n~~~\nb\n~~~\n",
  "~~~\na\n~~~\n\n```\nb\n```\n", "````js\ncode\n`````\n", "~~~~js\ncode\n~~~~~\n",
  "~~~\n```\n~~~\n", "```\n~~~\n```\n", "~~~\nalpha ``` beta\n~~~\n",
  "~~~js `meta`\ncode\n~~~\n", "~~~js\ncode", "```js\ncode", "   ~~~js\n   code\n   ~~~\n",
  "> ~~~js\n> code\n> ~~~\n", "- ~~~js\n  code\n  ~~~\n", "> - ~~~js\n>   code\n>   ~~~\n",
  "    ~~~js\n    code\n    ~~~\n", "\uFEFF~~~js\r\ncode\r\n~~~\r\n", "~~~js\rcode\r~~~\r",
  "~~~ js extra \ncode\n~~~  \n", "~~~js\tmeta\ncode\n~~~\n", "| ~~~ |\n| --- |\n",
  "<!-- markdownlint-disable-next-line MD048 -->\n~~~\na\n~~~\n\n```\nb\n```\n",
  "<!-- markdownlint-disable-file code-fence-style -->\n~~~\ncode\n~~~\n",
  "~~~\ncode\n<!-- markdownlint-disable MD048 -->\n~~~\n",
  ">\t~~~js\n>\tcode\n>\t~~~\n", "-\t~~~js\n\tcode\n\t~~~\n",
  "---\ntitle: x\n---\n~~~\ncode\n~~~\n", "~~~\ncode\n~~~~~~\n",
  "~~~\ncode\n    ~~~", "~~~\n> ~~~", "~~~\n    ```\n~~~\n", "~~~\n~~~\n", "~~~", "~~~\n",
  "> ~~~\n> code\n", "> ~~~\n> ~~~", "- ~~~\n  ~~\n", "~~~js extra\n  a\n~~~\n",
  "~~~\n<!-- markdownlint-disable-next-line MD048 -->\n~~~\n", "<div>\n~~~\ncode\n~~~\n</div>\n",
  "- > ~~~js\n  > code\n  > ~~~\n", "~~~js\u00a0extra\ncode\n~~~\n",
  "```~~~js\ncode\n```\n\n```rust\nok\n```\n",
];
const configurations = [true, false, {style:"backtick"}, {style:"tilde"}, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => ({
  id: `${sample}-${configuration}`, input, options,
  findings: lint({ strings: { input }, config: {default:false,MD048:options} }).input.map(finding => ({
    line:finding.lineNumber, detail:finding.errorDetail, severity:finding.severity,
  })),
})));
const receipt = new URL("md048-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
function codeBlocks(input) {
  let result = [];
  lint({strings:{input},config:{default:false,capture_code:true},customRules:[{
    names:["capture_code"],description:"Capture literal code boundaries",tags:["observation"],parser:"micromark",
    function(params) {
      const values = tokens => tokens.flatMap(token =>
        ["codeFlowValue","codeFencedFenceInfo","codeFencedFenceMeta"].includes(token.type) ? [[token.type,token.text]] : values(token.children));
      const visit = tokens => { for(const token of tokens) {
        if(["codeFenced","codeIndented"].includes(token.type)) {
          const parents=[]; for(let parent=token.parent;parent;parent=parent.parent) parents.push(parent.type);
          result.push({type:token.type,parents,values:values(token.children)});
        } else visit(token.children);
      }};
      visit(params.parsers.micromark.tokens);
    },
  }]});
  return result;
}
if (process.argv.includes("--source-only")) {
  console.log(`PASS: ${observations.length} pinned MD048 observations (native implementation pending)`);
} else {
  const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
  const directory = mkdtempSync("/tmp/opencode/mdfmt-md048-oracle-");
  const configuration = directory + "/.markdownlint-cli2.jsonc";
  let repairs=0;
  try { for (const row of observations) {
    for (const fix of [false, true]) {
      writeFileSync(configuration, JSON.stringify({ fix, config: {default:false,MD048:row.options} }));
      const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
      if(!fix) assert.equal(native.stdout, row.input, row.id);
      const expected=lint({strings:{input:native.stdout},config:{default:false,MD048:row.options}}).input
        .map(finding=>({line:finding.lineNumber,detail:finding.errorDetail,severity:finding.severity}));
      const actual = [...native.stderr.matchAll(/^-:(\d+):1: (Error|Warning) MD048: Code fence style \[([^\r\n]*)\]$/gm)]
        .map(match => ({line:Number(match[1]),detail:match[3],severity:match[2].toLowerCase()}));
      assert.deepEqual(actual, expected, `${row.id}: fix ${fix}: ${native.stderr}`);
      assert.equal(native.status, expected.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
      if(fix) {
        if(native.stdout!==row.input) repairs++;
        assert.deepEqual(codeBlocks(native.stdout),codeBlocks(row.input),`${row.id}: protected code/info/container boundaries`);
        const second=spawnSync(binary,["-"],{input:native.stdout,encoding:"utf8",cwd:directory});
        assert.equal(second.stdout,native.stdout,`${row.id}: idempotence`);
        assert.equal(second.status,native.status,`${row.id}: stable status`);
      }
    }
  } } finally { unlinkSync(configuration); rmdirSync(directory); }
  console.log(`PASS: ${observations.length} pinned MD048/native diagnostics, literal-code preservation and idempotence cases; ${repairs} native marker repairs absent upstream`);
}
