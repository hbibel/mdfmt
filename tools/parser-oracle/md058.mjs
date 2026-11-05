import assert from "node:assert/strict";
import {readFileSync,writeFileSync} from "node:fs";
import {lint} from "markdownlint/sync";
import {applyFixes} from "markdownlint";
const table="| a | b |\n| --- | --- |\n| c | d |\n";
const samples=[];
for(const before of ["","\n","text\n","# heading\n","<!-- comment -->\n","<!-- open\n","end -->\n","visible <!-- comment -->\n"])
  for(const after of ["","\n","# heading\n","<!-- comment -->\n","visible <!-- comment -->\n"])
    samples.push(before+table+after);
for(const [first,next]of [["> ","> "],["- ","  "],["> - ",">   "],["- > ","  > "],["10. ","    "]]){
  const nested=first+table.trimEnd().split("\n").join("\n"+next)+"\n";
  samples.push(nested+"# after\n");
  samples.push("# before\n"+nested);
  samples.push(nested+next.trimEnd()+"\n"+next+"# after\n");
}
samples.push("\uFEFF# before\r\n"+table.replaceAll("\n","\r\n")+"# after\r\n",
  "# before\r"+table.replaceAll("\n","\r")+"# after",
  "# before\n"+table.trimEnd(),
  "- item\n<span>\n"+table+"# after\n",
  "# before\n| a | b |\n| --- | --- |\nexample | f\n# after\n",
  "<!-- markdownlint-disable-next-line MD058 -->\n"+table+"# after\n",
  "<!-- markdownlint-disable-file blanks-around-tables -->\n# before\n"+table+"# after\n",
  "# before\n| 😀 | b |\n| --- | --- |\n| c | d |\n# after\n");
const configurations=[true,false,{severity:"warning"}];
const observations=samples.flatMap((input,sample)=>configurations.map((options,configuration)=>{
  const findings=lint({strings:{input},config:{default:false,MD058:options}}).input;
  const bom=input.startsWith("\uFEFF")?"\uFEFF":"";
  return {id:`${sample}-${configuration}`,input,options,findings:findings.map(f=>({line:f.lineNumber,range:f.errorRange,context:f.errorContext,severity:f.severity})),
    output:bom+applyFixes(input.slice(bom.length),findings)};
}));
const receipt=new URL("md058-observations.json",import.meta.url);
const serialized="[\n"+observations.map(row=>"  "+JSON.stringify(row)).join(",\n")+"\n]\n";
if(process.argv.includes("--record"))writeFileSync(receipt,serialized);else assert.equal(serialized,readFileSync(receipt,"utf8"));
console.log(`PASS: ${observations.length} pinned MD058 observations (native implementation pending)`);
