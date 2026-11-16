import assert from "node:assert/strict";
import {mkdtempSync,writeFileSync,readFileSync,rmSync,mkdirSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {lint} from "markdownlint/sync";

assert.equal(JSON.parse(readFileSync(new URL("node_modules/markdownlint/package.json",import.meta.url))).version,"0.42.0");
const cases=[];
function add(id,input,rule,options){cases.push({id,input,config:{default:false,[rule]:options}});}
for(const rule of ["MD001","MD025","MD041"]){for(const pattern of ["^name:","^NAME:","^absent:","","^([a-z]+)-\\1:","(?<=prefix)name:"]){const metadata=pattern.includes("\\1")?"name-name:":pattern.includes("prefix")?"prefixname:":"name:";add(`${rule}-title-${pattern}`,`---\n${metadata} Demo\n---\n\n${rule==="MD001"?"### Deep":rule==="MD025"?"# Heading":"Ordinary prose."}\n`,rule,{front_matter_title:pattern});}}
for(const [text,pattern,match_case]of [["prefix tail","(?<=prefix )tail",false],["repeat repeat","^(\\w+) \\1$",false],["K","^k$",false],["K","^k$",true],["ſ","^s$",false],["😀","^.$",true],["😀","^\\uD83D$",true],["Title","title",false]])add(`heading-${text}-${pattern}-${match_case}`,`# ${text}\n`,"MD043",{headings:[`% ${pattern}`],match_case});
add("heading-level-mismatch","## Title\n","MD043",{headings:["% Title"]});
add("heading-mixed-regex-prefix","## Title\n","MD043",{headings:["%# ^Title$"]});
add("heading-mixed-literal-prefix","## Title\n","MD043",{headings:["#% Title"]});
add("heading-wildcard-loop","# Intro\n\n## Item 1\n\n## Item 2\n\n# End\n","MD043",{headings:["% ^Intro$","^","%% ^Item [0-9]$","^","% ^End$"]});
for(const [fragment,pattern]of [["missing","^missing$"],["missing","^#missing$"],["skip","(?<=s)kip"],["abcabc","^(abc)\\1$"],["😀","^.$"],["😀","^..$"],["😀","^\\uD83D"],["K","^k$"],["missing",""]])add(`fragment-${fragment}-${pattern}`,`[link](#${fragment})\n`,"MD051",{ignored_pattern:pattern});
add("title-nonunicode-fold","---\nK: value\n---\n\nOrdinary prose.\n","MD041",{front_matter_title:"^k:"});
for(const key of ["K","ſ"])for(const pattern of ["^\\w+:","^[\\w]+:","^\\W+:","^[\\W]+:"])for(const rule of ["MD001","MD025","MD041"])add(`title-shorthand-${rule}-${key}-${pattern}`,`---\n${key}: Title\n---\n\n${rule==="MD001"?"### Deep":rule==="MD025"?"# Heading":"Ordinary prose."}\n`,rule,{front_matter_title:pattern});
for(const key of ["K","ſ"])for(const pattern of ["^\\w$","^[\\w]$","^\\W$","^[\\W]$"])for(const match_case of [false,true])add(`heading-shorthand-${key}-${pattern}-${match_case}`,`# ${key}\n`,"MD043",{headings:[`% ${pattern}`],match_case});
add("inline-pattern-config","<!-- markdownlint-configure-file {\"MD043\":{\"headings\":[\"% ^Intro$\"]}} -->\n# Intro\n","MD043",false);
add("inline-fragment-disable","<!-- markdownlint-disable MD051 -->\n[link](#missing)\n","MD051",{ignored_pattern:"^skip$"});

const rows=[];
for(const fixture of cases){const findings=lint({strings:{doc:fixture.input},config:fixture.config}).doc.map(item=>({rule:item.ruleNames[0],line:item.lineNumber})).sort((a,b)=>a.line-b.line||a.rule.localeCompare(b.rule));rows.push({...fixture,findings});if(process.argv.includes("--source-only"))continue;
 const root=mkdtempSync("/tmp/opencode/mdfmt-config-pattern-");try{writeFileSync(path.join(root,".markdownlint-cli2.jsonc"),JSON.stringify({fix:false,config:fixture.config}));writeFileSync(path.join(root,"doc.md"),fixture.input);const binary=process.env.MDFMT_BINARY||fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));const output=spawnSync(binary,["doc.md"],{cwd:root,encoding:"utf8",timeout:10000});assert.equal(output.status,findings.length?1:0,`${fixture.id}: ${output.stderr}`);assert.equal(output.stdout,fixture.input,fixture.id);const native=[...output.stderr.matchAll(/doc\.md:(\d+):\d+: Error (MD\d+):/g)].map(match=>({line:Number(match[1]),rule:match[2]})).sort((a,b)=>a.line-b.line||a.rule.localeCompare(b.rule));assert.deepEqual(native,findings,fixture.id);assert.equal(readFileSync(path.join(root,"doc.md"),"utf8"),fixture.input);}finally{rmSync(root,{recursive:true,force:true});}}
const receipt=new URL("../../tests/fixtures/config-patterns/cases.json",import.meta.url),serialized=JSON.stringify(rows,null,2)+"\n";
if(process.argv.includes("--record")){mkdirSync(path.dirname(fileURLToPath(receipt)),{recursive:true});writeFileSync(receipt,serialized);}else assert.equal(serialized,readFileSync(receipt,"utf8"));
console.log(`PASS: ${rows.length} pinned configuration-pattern ${process.argv.includes("--source-only")?"source-only observations (native not checked)":"native diagnostic/source-preservation cases"}`);
