import assert from "node:assert/strict";
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
import {readConfig,lint} from "markdownlint/sync";
import parsers from "markdownlint-cli2/parsers";

assert.equal(JSON.parse(readFileSync(new URL("node_modules/markdownlint/package.json",import.meta.url))).version,"0.42.0");
assert.equal(JSON.parse(readFileSync(new URL("node_modules/markdownlint-cli2/package.json",import.meta.url))).version,"0.24.0");

const cases=[];
const text=value=>typeof value==="string"?value:JSON.stringify(value);
const rules=width=>({default:false,MD013:{line_length:width,strict:true}});
function add(id,metadata={},reference="pkg",extra={},nativeError){
 const files={".markdownlint.json":text({extends:reference,MD026:true}),"node_modules/pkg/package.json":text({name:"pkg",...metadata}),"node_modules/pkg/config.json":text(rules(24)),"node_modules/pkg/index.json":text(rules(27)),...Object.fromEntries(Object.entries(extra).map(([name,value])=>[name,text(value)]))};cases.push({id,files,nativeError});
}
add("main-json",{main:"config.json"});add("index-json");add("main-dot-json",{main:"./config.json"});add("invalid-main-index",{main:"missing.json"});add("main-directory-index",{main:"configs"},"pkg",{"node_modules/pkg/configs/index.json":rules(30)});
for(const [extension,contents] of [["yaml","default: false\nMD013:\n  line_length: 24\n  strict: true\n"],["yml","default: false\nMD013:\n  line_length: 24\n  strict: true\n"],["toml","default = false\n[MD013]\nline_length = 24\nstrict = true\n"],["jsonc","{\"default\":false,/*pinned*/\"MD013\":{\"line_length\":24,\"strict\":true},}"]])add(`main-${extension}`,{main:`config.${extension}`},"pkg",{[`node_modules/pkg/config.${extension}`]:contents});
add("subpath-json",{},"pkg/config.json");add("subpath-extension",{},"pkg/config");
add("scoped",{},"@team/config",{"node_modules/@team/config/package.json":{main:"rules.json"},"node_modules/@team/config/rules.json":rules(24)});
add("nearer-ancestor",{},"pkg",{"work/.markdownlint.json":{extends:"pkg",MD026:true},"work/node_modules/pkg/package.json":{main:"rules.json"},"work/node_modules/pkg/rules.json":rules(31)});cases.at(-1).invocation="work";
add("parent-ancestor",{main:"config.json"},"pkg",{"work/deep/.markdownlint.json":{extends:"pkg",MD026:true}});cases.at(-1).invocation="work/deep";
add("exports-string",{main:"index.json",exports:"./config.json"});
for(const [id,exports] of [["default-first",{default:"./config.json",require:"./index.json"}],["require-first",{require:"./index.json",default:"./config.json"}],["module-sync",{"module-sync":"./config.json",require:"./index.json"}],["node-addons",{"node-addons":"./config.json",require:"./index.json"}],["node-first",{node:"./config.json",require:"./index.json"}],["ignore-import",{import:"./index.json",require:"./config.json"}],["ignore-browser",{browser:"./index.json",default:"./config.json"}]])add(id,{exports});
add("exports-dot",{exports:{".":"./config.json"}});add("exports-subpath",{exports:{"./strict":"./config.json"}},"pkg/strict");
add("exports-pattern",{exports:{"./configs/*":"./rules/*.json"}},"pkg/configs/strict",{"node_modules/pkg/rules/strict.json":rules(24)});
add("exports-specific-pattern",{exports:{"./*":"./general/*.json","./configs/*":"./rules/*.json"}},"pkg/configs/strict",{"node_modules/pkg/rules/strict.json":rules(24),"node_modules/pkg/general/configs/strict.json":rules(27)});
add("exports-null-root",{main:"config.json",exports:{".":null}});add("exports-null-field-legacy",{main:"config.json",exports:null});add("exports-private-subpath",{exports:"./config.json"},"pkg/index.json");
add("exports-array",{exports:["./config.json","./index.json"]});add("exports-array-missing-first",{exports:["./missing.json","./config.json"]});add("exports-array-invalid-first",{exports:["../invalid.json","./config.json"]});add("exports-array-null-first",{exports:[null,"./config.json"]});add("exports-array-empty",{exports:[]});
add("exports-outside",{exports:"../config.json"});add("exports-invalid-mixed",{exports:{".":"./config.json",require:"./index.json"}});add("exports-numeric-condition",{exports:{0:"./config.json",default:"./index.json"}});add("exports-missing-file",{exports:"./absent.json"});
for(const [id,exports] of [
 ["numeric-nested",{require:{0:"./config.json",default:"./index.json"}}],
 ["numeric-array-hard-error",[{0:"./config.json",default:"./index.json"},"./config.json"]],
 ["numeric-unselected-condition",{browser:{0:"./config.json"},default:"./index.json"}],
 ["numeric-unselected-subpath",{".":"./config.json","./unused":{0:"./index.json"}}],
 ["numeric-non-index-leading-zero",{"01":"./index.json",default:"./config.json"}],
 ["numeric-non-index-max",{"4294967295":"./index.json",default:"./config.json"}],
 ["numeric-largest-index",{"4294967294":"./index.json",default:"./config.json"}],
 ["conditional-null-stops",{require:null,default:"./config.json"}],
 ["array-empty-fallback",[[],"./config.json"]],
 ["array-primitive-fallback",[false,"./config.json"]],
 ["array-all-undefined-condition",{require:[{browser:"./index.json"}],default:"./config.json"}],
 ["array-error-before-undefined",{require:["../bad",{browser:"./index.json"}],default:"./config.json"}],
 ["array-error-before-null",{require:["../bad",null],default:"./config.json"}],
])add(id,{exports});
add("package-relative-chain",{main:"config.json"},"pkg",{"node_modules/pkg/config.json":{extends:"./base.yaml",MD013:{line_length:24,strict:true}},"node_modules/pkg/base.yaml":"default: false\nMD026: false\n"});
add("package-package-chain",{main:"config.json"},"pkg",{"node_modules/pkg/config.json":{extends:"shared",MD013:{line_length:24,strict:true}},"node_modules/shared/package.json":{main:"rules.json"},"node_modules/shared/rules.json":{default:false}});
add("imports",{},"#shared",{"package.json":{name:"project",imports:{"#shared":"./shared.json"}},"shared.json":rules(24)});
add("self-reference",{},"project/strict",{"package.json":{name:"project",exports:{"./strict":"./shared.json"}},"shared.json":rules(24)});
add("exact-local-shadow",{main:"index.json"},"pkg/config.json",{"pkg/config.json":rules(24)});
add("relative-extension-fallback",{},"./shared",{"shared.json":rules(24)});
add("direct-directory-shadow",{},"pkg.json",{"pkg.json/inner.json":rules(24),"node_modules/pkg.json/package.json":{main:"rules.json"},"node_modules/pkg.json/rules.json":rules(24)});
add("missing-package",{},"absent-package/config.json");add("invalid-package-json",{},"pkg",{"node_modules/pkg/package.json":"{ broken"});
for(const extension of ["js","cjs","mjs","node",""]){const target=`config${extension?'.'+extension:''}`;add(`unsupported-${extension||'extensionless'}`,{main:target},"pkg",{[`node_modules/pkg/${target}`]:rules(24)},"unsupported-config-format");}

const rows=[];
for(const fixture of cases){const root=mkdtempSync("/tmp/opencode/mdfmt-config-package-");try{
 for(const [name,contents]of Object.entries(fixture.files)){const file=path.join(root,name);mkdirSync(path.dirname(file),{recursive:true});writeFileSync(file,contents);}
 const invocation=fixture.invocation||"",config=path.join(root,invocation,".markdownlint.json");let source;
 try{source={config:readConfig(config,parsers)};}catch(error){source={error:error.code||error.name};}
 rows.push({...fixture,invocation,source});
 if(!process.argv.includes("--source-only")){
  const binary=process.env.MDFMT_BINARY||fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));const input="# Heading!\n\none two three four five six seven eight nine ten\n";writeFileSync(path.join(root,invocation,"doc.md"),input);
  const invoke=()=>spawnSync(binary,["doc.md"],{cwd:path.join(root,invocation),encoding:"utf8",timeout:10000});const first=invoke();
  if(fixture.nativeError||source.error){assert.equal(first.status,2,`${fixture.id}: ${first.stderr}`);assert.equal(first.stdout,"");assert.ok(first.stderr.includes(fixture.nativeError||"invalid-config"),`${fixture.id}: ${first.stderr}`);}
  else{assert.equal(first.status,0,`${fixture.id}: ${first.stderr}`);assert.ok(first.stdout.startsWith("# Heading\n\n"));assert.equal(first.stdout.slice(11).trim().split(/\s+/).join(" "),"one two three four five six seven eight nine ten");assert.deepEqual(lint({strings:{doc:first.stdout},config:source.config}).doc,[]);writeFileSync(path.join(root,invocation,"doc.md"),first.stdout);const second=invoke();assert.equal(second.stdout,first.stdout);assert.equal(second.stderr,first.stderr);assert.equal(second.status,first.status);}
 }
}finally{rmSync(root,{recursive:true,force:true});}}
const receipt=new URL("../../tests/fixtures/config-packages/cases.json",import.meta.url),serialized=JSON.stringify(rows,null,2)+"\n";
if(process.argv.includes("--record")){mkdirSync(path.dirname(fileURLToPath(receipt)),{recursive:true});writeFileSync(receipt,serialized);}else assert.equal(serialized,readFileSync(receipt,"utf8"));
console.log(`PASS: ${rows.length} pinned installed-config-package ${process.argv.includes("--source-only")?"source-only observations (native not checked)":"native resolution/formatting/error/idempotence cases"}`);
