import assert from "node:assert/strict";
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from "node:fs";
import {createHash} from "node:crypto";
import {spawnSync} from "node:child_process";
import {fileURLToPath} from "node:url";
import path from "node:path";
import {lint} from "markdownlint/sync";

assert.equal(JSON.parse(readFileSync(new URL("node_modules/markdownlint/package.json",import.meta.url))).version,"0.42.0");
const directory=new URL("../../tests/fixtures/corpus/",import.meta.url);
const manifest=JSON.parse(readFileSync(new URL("manifest.json",directory)));
for(const item of manifest.sources)assert.equal(createHash("sha256").update(readFileSync(new URL(item.path,directory))).digest("hex"),item.sha256,item.path);
const cases=JSON.parse(readFileSync(new URL("commonmark.json",directory))).map(row=>({id:`commonmark-${row.example}`,input:row.markdown}));
for(const name of ["regress","markdownlint"])cases.push({id:`readme-${name}`,input:readFileSync(new URL(`${name}-README.md`,directory),"utf8")});
assert.equal(cases.length,654);
const atomic=new Set(["link","image","autolink","literalAutolink","codeText","htmlText","htmlFlow","codeFenced","codeIndented","table","definition","hardBreakEscape","hardBreakTrailing","characterReference","characterEscape","listItemPrefix","thematicBreak","mathText","mathFlow","directiveContainer","directiveLeaf","directiveText"]);
const structural=new Set(["paragraph","atxHeading","setextHeading","listOrdered","listUnordered","blockQuote","emphasis","strong"]);
const inlineBlocks=new Set(["paragraph","atxHeading","setextHeading"]);

function normalize(items,trim=false) {
    const result=[];
    for(const item of items) {
        const last=result.at(-1);
        if(item[0]==="data"&&last?.[0]==="data")last[1]=(last[1]+item[1]).replace(/[\t\r\n ]+/gu," ");
        else result.push([...item]);
    }
    if(trim) {
        if(result[0]?.[0]==="data")result[0][1]=result[0][1].replace(/^ +/u,"");
        if(result.at(-1)?.[0]==="data")result.at(-1)[1]=result.at(-1)[1].replace(/ +$/u,"");
    }
    return result.filter(item=>item[0]!=="data"||item[1]!=="");
}

function observe(input) {
    let observation;
    // markdownlint strips the initial BOM and normalizes newlines before parsing.
    const lineStarts=[input.startsWith("\uFEFF")?1:0];
    for(let index=lineStarts[0];index<input.length;index++) {
        if(input[index]==="\r") {
            if(input[index+1]==="\n")index++;
            lineStarts.push(index+1);
        }else if(input[index]==="\n")lineStarts.push(index+1);
    }
    function offset(line,column) {
        assert.ok(line>=1&&line<=lineStarts.length,"upstream line outside original source");
        return Buffer.byteLength(input.slice(0,lineStarts[line-1]+column-1));
    }
    lint({strings:{doc:input},noInlineConfig:true,frontMatter:/^never-match$/u,config:{default:false,"corpus-probe":true},customRules:[{
        names:["corpus-probe"],description:"Capture corpus boundaries",tags:["observation"],parser:"micromark",
        function(params) {
            const protectedRanges=[];
            function visit(tokens,inline=false) {
                return tokens.flatMap(token=> {
                    // markdownlint appends diagnostic-only copies of rendered data.
                    if(token.type.startsWith("undefinedReference"))return [];
                    if(atomic.has(token.type)) {
                        const start=offset(token.startLine,token.startColumn),end=offset(token.endLine,token.endColumn);
                        assert.equal(Buffer.from(input).subarray(start,end).toString().replace(/\r\n?/gu,"\n"),token.text,"upstream coordinates must recover the original atomic payload");
                        protectedRanges.push([start,end,token.type]);
                        return [[token.type,token.text]];
                    }
                    if(token.type==="data")return [["data",token.text.replace(/[\t\r\n ]+/gu," ")]];
                    if(token.type==="lineEnding"&&inline)return [["data"," "]];
                    const children=normalize(visit(token.children,inline||inlineBlocks.has(token.type)),inlineBlocks.has(token.type));
                    return structural.has(token.type)?[[token.type,"open"],...children,[token.type,"close"]]:children;
                });
            }
            const structure=normalize(visit(params.parsers.micromark.tokens));
            observation={protected:protectedRanges,structure};
        }
    }]});
    assert.ok(observation,"source parser did not capture corpus");
    return observation;
}

assert.deepEqual(observe("alpha\nbeta\n").structure,observe("alpha beta\n").structure);
assert.notDeepEqual(observe("alpha **beta**\n").structure,observe("alpha**beta**\n").structure,"word separation around inline nodes must remain observable");
assert.notDeepEqual(observe("&copy;\n").structure,observe("&amp;\n").structure,"entity payload must remain observable");
assert.deepEqual(observe("[foo]: /url\n===\n[foo]\n").structure,observe("[foo]: /url\n=== [foo]\n").structure,"micromark content chunks are not semantic blocks");
assert.deepEqual(observe("[foo [bar](/uri)](/uri)\n").structure,observe("[foo\n[bar](/uri)](/uri)\n").structure,"artificial undefined-reference overlays are not rendered content");
const coordinateRows=[];
for(const bom of ["","\uFEFF"])for(const newline of ["\n","\r\n","\r"])for(const [name,body]of [
    ["link","alpha [link](/url)\n"],
    ["container","> - 😀\talpha beta\nlazy [link](/url) and `code`\n\n> tail\n"]
]) {
    const input=bom+body.replace(/\n/gu,newline),source=observe(input),bytes=Buffer.from(input);
    const link=source.protected.filter(span=>span[2]==="link");
    assert.equal(link.length,1,"coordinate control lost the source link");
    assert.equal(link[0][0],bytes.indexOf("[link](/url)"),"upstream range must use original BOM/newline bytes");
    assert.equal(bytes.subarray(link[0][0],link[0][1]).toString(),"[link](/url)","upstream link slice");
    if(name==="container") {
        const code=source.protected.find(span=>span[2]==="codeText");
        assert.ok(code,"coordinate control lost inline code");
        assert.equal(bytes.subarray(code[0],code[1]).toString(),"`code`","upstream inline-code slice");
    }
    coordinateRows.push({id:`coordinates-${coordinateRows.length}`,input,...source});
}
const coordinateDestination=new URL("coordinate-observations.json",directory),coordinateSerialized=JSON.stringify(coordinateRows,null,2)+"\n";
if(process.argv.includes("--record"))writeFileSync(coordinateDestination,coordinateSerialized);else assert.equal(readFileSync(coordinateDestination,"utf8"),coordinateSerialized);
const root=process.argv.includes("--source-only")?null:mkdtempSync("/tmp/opencode/mdfmt-corpus-oracle-");
const rows=[];
try {
    if(root)writeFileSync(path.join(root,".markdownlint-cli2.jsonc"),JSON.stringify({frontMatter:"^never-match$",noInlineConfig:true,config:{default:false,MD013:{line_length:12}}}));
    for(const fixture of cases) {
        const source=observe(fixture.input);
        rows.push({...fixture,...source});
        if(!root)continue;
        const binary=process.env.MDFMT_BINARY||fileURLToPath(new URL("../../target/debug/mdfmt",import.meta.url));
        const input=path.join(root,"doc.md");writeFileSync(input,fixture.input);
        const run=text=>spawnSync(binary,["-"],{input:text,cwd:root,encoding:"utf8",timeout:10000,maxBuffer:4*1024*1024});
        const first=run(fixture.input);
        assert.ok(!first.error&&[0,1].includes(first.status),`${fixture.id}: ${first.error||first.stderr}`);
        const second=run(first.stdout);
        assert.ok(!second.error&&second.status===first.status,`${fixture.id}: repeat ${second.error||second.stderr}`);
        assert.equal(second.stdout,first.stdout,`${fixture.id}: idempotence`);
        assert.deepEqual(observe(first.stdout).structure,source.structure,`${fixture.id}: source parser structure/atomic values`);
        assert.equal(readFileSync(input,"utf8"),fixture.input,`${fixture.id}: input file changed`);
    }
}finally{if(root)rmSync(root,{recursive:true,force:true});}
const destination=new URL("observations.json",directory),serialized=JSON.stringify(rows,null,2)+"\n";
if(process.argv.includes("--record"))writeFileSync(destination,serialized);else assert.equal(readFileSync(destination,"utf8"),serialized);
console.log(`PASS: ${rows.length} licensed corpus ${root?"native/source preservation and idempotence cases":"source-only observations (native not checked)"}`);
