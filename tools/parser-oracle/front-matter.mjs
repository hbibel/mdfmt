import assert from "node:assert/strict";
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";
import {RE2JS} from "re2js";

assert.equal(JSON.parse(readFileSync(new URL("node_modules/markdownlint/package.json",import.meta.url))).version,"0.42.0");
const cases=[];
const add=(id,input,pattern,coordinateDifference=false)=>cases.push({id,input,pattern,coordinateDifference});
for(const newline of ["\n","\r\n","\r"]){for(const bom of ["","\uFEFF"]){add(`metadata-${JSON.stringify(newline)}-${!!bom}`,bom+"%%%\nname: 😀\n# Secret!\n%%%\n\n# Visible!\n".replaceAll("\n",newline),"^%%%\\n[\\s\\S]*?\\n%%%\\n");}}
for(const [id,pattern]of [["absent",undefined],["empty",""],["nonmatch","^NOT-PRESENT"],["zero-width","^"],["nonprefix","name:"],["partial-line","^%%%"],["ascii-space","^%%%\\s+"],["unicode-space-not-re2","^%%%\\s+"],["unicode-property","^%%%\\p{L}+\\n"],["lookahead-unsupported","(?=%%%)[\\s\\S]*?%%%\\n"],["lookbehind-unsupported","(?<=x)y"],["backreference-unsupported","^((a)\\2)"],["invalid-js","["]]){const input=id==="unicode-space-not-re2"?"%%%\u00A0\n# Visible!\n":id==="unicode-property"?"%%%Title\n# Visible!\n":id==="absent"||id==="empty"||id==="nonmatch"||id==="zero-width"?"---\n# Secret!\n---\n\n# Visible!\n":"%%%\nname: value\n%%%\n\n# Visible!\n";add(id,input,pattern,["partial-line","ascii-space","unicode-space-not-re2","nonprefix"].includes(id));}
const rows=[];
add("class-ampersands-are-literal","%%%ab&\n# Visible!\n","^%%%[ab&&]+\\n");
add("class-tildes-are-literal","%%%ab~\n# Visible!\n","^%%%[ab~~]+\\n");
add("class-leading-dash-range","%%%0\n# Visible!\n","^%%%[--a]+\\n");
add("class-backspace-unsupported","%%%\b\n# Visible!\n","^%%%[\\b]\\n");
add("control-escape","%%%\x01\n# Visible!\n","^%%%\\cA\\n");
add("class-control-escape","%%%\x01\n# Visible!\n","^%%%[\\cA]\\n");
add("word-boundary-ascii","%%%é\n# Visible!\n","^%%%\\Bé\\n");
add("negative-word-boundary-ascii","%%%a\n# Visible!\n","^%%%\\ba\\n");
add("vertical-tab-control","%%%\u000B\n# Visible!\n","^%%%\\v\\n");
add("end-anchor-final-newline","%%%\n","^%%%$");
add("body-dependent-boundary","%%%\n# Secret!\n%%%\n*body*\n","^%%%\\n[\\s\\S]*?\\n%%%\\n\\B");
for(const codepoint of [0x1e6c0,0x10940,0x11db0])for(const property of ["\\p{L}","\\P{C}","\\P{Cn}","[\\p{L}]"])add(`new-letter-${codepoint.toString(16)}-${property}`,`${String.fromCodePoint(codepoint)}\n# Secret!\nEND\n\n# Body!\n`,`^${property}[\\s\\S]*?END\\n`);
for(const name of ["C","Cc","Cf","Cn","Co","Cs","L","LC","Ll","Lm","Lo","Lt","Lu","M","Mc","Me","Mn","N","Nd","Nl","No","P","Pc","Pd","Pe","Pf","Pi","Po","Ps","S","Sc","Sk","Sm","So","Z","Zl","Zp","Zs","ASCII_Hex_Digit","Alphabetic","Dash","Emoji","Emoji_Component","Emoji_Modifier","Emoji_Modifier_Base","Emoji_Presentation","Extended_Pictographic","Hex_Digit","Lowercase","Math","Quotation_Mark","Terminal_Punctuation","Uppercase","White_Space","Any","Assigned"])for(const ch of ["A",String.fromCodePoint(0x1e6c0)])for(const escape of ["p","P"])add(`property-${name}-${ch}-${escape}`,`${ch}\n# Secret!\nEND\n\n# Body!\n`,`^[\\${escape}{${name}}][\\s\\S]*?END\\n`);
for(const pattern of ["\\p{Letter}","\\p{Lowercase_Letter}","\\p{ASCII}","\\p{Script=Greek}","a{1001}","a{0,1001}","(?:a{100}){11}","(?:a{1000}){2,}"])add(`invalid-re2-${pattern}`,"%%%Title\n# Body!\n",pattern);
for(const pattern of ["a{1000}","a{0,1000}","a{1000,}","(?:a{100}){10}","[a{1001}]","\\{1001\\}"])add(`valid-repeat-${pattern}`,"a\n# Body!\n",`^${pattern}\\n`);
for(const fixture of cases){let observation;
 try{const front=fixture.pattern?new RegExp(fixture.pattern,"u"):undefined;const normalized=fixture.input.replace(/^\uFEFF/,"").replace(/\r\n?/g,"\n");const match=front?RE2JS.compile(RE2JS.translateRegExp(front)).exec(normalized):null;const metadata=[];const results=lint({strings:{doc:fixture.input},frontMatter:front,config:{default:false,MD026:true,"front-probe":true},customRules:[{names:["front-probe"],description:"Capture metadata",tags:["test"],parser:"micromark",function:params=>metadata.push(...params.frontMatterLines)}]}).doc;observation={metadata,normalizedPrefix:match&&match.index===0?match[0]:null,findings:results.map(finding=>({rule:finding.ruleNames[0],line:finding.lineNumber}))};}
 catch(error){observation={error:error.name,message:error.message};}
 const row={...fixture,source:observation};rows.push(row);if(process.argv.includes("--source-only"))continue;
 const root=mkdtempSync("/tmp/opencode/mdfmt-front-matter-");try{writeFileSync(path.join(root,".markdownlint-cli2.jsonc"),JSON.stringify({fix:false,frontMatter:fixture.pattern,config:{default:false,MD026:true}}));writeFileSync(path.join(root,"doc.md"),fixture.input);const binary=process.env.MDFMT_BINARY||fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));const output=spawnSync(binary,["doc.md"],{cwd:root,encoding:"utf8",timeout:10000});if(observation.error){assert.equal(output.status,2,fixture.id);assert.equal(output.stdout,"");assert.ok(output.stderr.includes("frontMatter"));}else{assert.equal(output.status,observation.findings.length?1:0,`${fixture.id}: ${output.stderr}`);assert.equal(output.stdout,fixture.input);if(!fixture.coordinateDifference){const native=[...output.stderr.matchAll(/doc\.md:(\d+):\d+: Error (MD\d+):/g)].map(match=>({rule:match[2],line:Number(match[1])}));assert.deepEqual(native,observation.findings,fixture.id);}else{assert.equal([...output.stderr.matchAll(/Error MD026:/g)].length,observation.findings.length,fixture.id);}}assert.equal(readFileSync(path.join(root,"doc.md"),"utf8"),fixture.input);}finally{rmSync(root,{recursive:true,force:true});}}
const receipt=new URL("../../tests/fixtures/front-matter/cases.json",import.meta.url),serialized=JSON.stringify(rows,null,2)+"\n";if(process.argv.includes("--record")){mkdirSync(path.dirname(fileURLToPath(receipt)),{recursive:true});writeFileSync(receipt,serialized);}else assert.equal(serialized,readFileSync(receipt,"utf8"));
console.log(`PASS: ${rows.length} pinned front-matter ${process.argv.includes("--source-only")?"source-only observations (native not checked)":"native metadata/diagnostic/preservation cases"}`);
