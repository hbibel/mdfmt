# Reproducing the audit checks

These checks validate audit data against upstream source metadata and fixture
consistency. They do not run mdfmt or prove Markdown semantic equivalence.

Obtain the two source trees at the commits in `baseline.md`, preserving their
directory names `markdownlint` and `markdownlint-cli2` under a common directory.
Run from the project root, replacing the source-cache argument as needed.
No dependency installation is required.

```sh
python3 - /tmp/opencode/mdfmt-audit-2026-10-21 <<'PY'
import collections
import json
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
docs = Path('docs/compatibility')
data = json.loads((docs / 'rules.json').read_text())
schema = json.loads((root / 'markdownlint/schema/markdownlint-config-schema.json').read_text())['properties']
ids = {key for key in schema if re.fullmatch(r'MD\d{3}', key)}
assert len(data['rules']) == len(ids) == len({r['id'] for r in data['rules']})
assert {r['id'] for r in data['rules']} == ids
registry = (root / 'markdownlint/lib/rules.mjs').read_text()
assert {s.upper() for s in re.findall(r'^  (md\d{3}),?$', registry, re.M)} == ids
for rule in data['rules']:
    assert {'id', 'aliases', 'tags', 'options', 'upstreamFixable', 'fixability', 'rationale', 'source'} <= rule.keys()
    source = (root / 'markdownlint' / rule['source']).read_text()
    names = re.search(r'"names":\s*(\[\s*"' + rule['id'] + r'"[^\]]*\])', source)
    assert json.loads(names[1])[1:] == rule['aliases'], rule['id']
    tags = re.search(r'"tags":\s*(\[[^\]]*\])', source[names.end():])
    assert json.loads(tags[1]) == rule['tags'], rule['id']
    props = next(item['properties'] for item in schema[rule['id']]['oneOf'] if item.get('type') == 'object')
    expected = [{'name': k, 'type': v['type'], 'default': v.get('default')} for k, v in props.items() if k not in ('enabled', 'severity')]
    assert expected == [{k: option[k] for k in ('name', 'type', 'default')} for option in rule['options']], rule['id']
    assert rule['upstreamFixable'] == any(key in source for key in ('"editColumn"', '"deleteCount"', '"insertText"', 'fixInfo')), rule['id']
    assert rule['fixability'] in ('safe', 'conditional', 'diagnostic-only')
print('Rule inventory:', len(ids), 'rules;', sum(r['upstreamFixable'] for r in data['rules']), 'source fix emitters')
print('Proposed fixability:', dict(collections.Counter(r['fixability'] for r in data['rules'])))

properties = json.loads((root / 'markdownlint-cli2/schema/markdownlint-cli2-config-schema.json').read_text())['properties']
text = (docs / 'configuration.md').read_text()
assert set(re.findall(r'^### (\S+) — (?:supported|unsupported|deferred)$', text, re.M)) == set(properties)
print('CLI configuration:', len(properties), 'properties classified')

acceptance = json.loads((docs / 'acceptance.json').read_text())
cases = acceptance['cases']
errors = acceptance['errorCases']
assert len({case['id'] for case in cases + errors}) == len(cases) + len(errors)
for case in cases:
    assert {'id', 'input', 'config', 'expectedOutput', 'remainingRuleIds', 'rationale'} <= case.keys(), case['id']
    assert set(case['remainingRuleIds']) <= ids, case['id']
    assert len(case['remainingRuleIds']) == len(set(case['remainingRuleIds'])), case['id']
    for content in case.get('files', {}).values():
        json.loads(content)
    if 'files' in case:
        assert {'cwd', 'target', 'expectedEffectiveConfig'} <= case.keys(), case['id']
    scenario = case.get('formatterScenario')
    if scenario:
        assert scenario in ('success', 'unavailable', 'failure', 'timeout')
    if scenario in ('unavailable', 'failure', 'timeout'):
        assert case['input'] == case['expectedOutput'], case['id']
    before = [line for line in case['input'].splitlines() if line.startswith('|')]
    after = [line for line in case['expectedOutput'].splitlines() if line.startswith('|')]
    assert before == after, case['id']
    links = r'\[[^\]\n]*\]\([^\)\n]*\)'
    assert re.findall(links, case['input']) == re.findall(links, case['expectedOutput']), case['id']
    options = case['config'].get('MD013') or {}
    if isinstance(options, dict) and options.get('strict') and not case['remainingRuleIds']:
        if options.get('tables', True) and options.get('code_blocks', True) and not scenario and not case['id'].startswith('reference-definition'):
            assert max(map(len, case['expectedOutput'].splitlines()), default=0) <= options['line_length'], case['id']
for case in errors:
    assert {'id', 'path', 'content', 'expectedError', 'rationale'} <= case.keys()
print('Acceptance:', len(cases), 'formatting/resolution cases;', len(errors), 'error cases')
print('PASS: audit consistency only; full formatter and upstream oracle tests pending')
PY
```

The source-fix check recognizes fix-information fields in each referenced
module. Shared MD019/MD021 and MD049/MD050 modules were also inspected manually.
It establishes that a rule can emit a fix, not that every violation is fixable.
The simple link and table checks apply only to the deliberately small fixture
syntax here; they are not reusable Markdown parsing or semantic validation.

## Narrow source experiments

The following runs the actual dependency-free merge module and extracted pure
effective-config and front-matter-title functions. The source cache must include
`markdownlint/helpers/helpers.cjs`. It does not run the complete cli2 resolver.

```sh
node --input-type=module - /tmp/opencode/mdfmt-audit-2026-10-21 <<'JS'
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { pathToFileURL } from 'node:url';
const root = process.argv[2];
const { default: merge } = await import(pathToFileURL(`${root}/markdownlint-cli2/merge-options.mjs`));
assert.deepEqual(
  merge({ config: { default: false, MD013: { line_length: 60, strict: true } } },
        { config: { MD013: { line_length: 40 } } }),
  { config: { default: false, MD013: { line_length: 40 } } }
);
const source = fs.readFileSync(`${root}/markdownlint/lib/markdownlint.mjs`, 'utf8');
const start = source.indexOf('function getEffectiveConfig(');
assert(start >= 0);
const end = source.indexOf('\n}', start) + 2;
const effective = Function(`return (${source.slice(start, end)});`)();
const rules = [{ names: ['MD013'] }];
const aliases = { MD013: ['MD013'], LINE_LENGTH: ['MD013'] };
const enabled = effective(rules, { default: false, MD013: false, line_length: 'warning' }, aliases);
assert.equal(enabled.rulesEnabled.get('MD013'), true);
assert.equal(enabled.rulesSeverity.get('MD013'), 'warning');
const disabled = effective(rules, { default: false, line_length: 'warning', MD013: false }, aliases);
assert.equal(disabled.rulesEnabled.get('MD013'), false);
const cases = JSON.parse(fs.readFileSync('docs/compatibility/acceptance.json', 'utf8')).cases;
const explicit = cases.find(c => c.id === 'discovered-cli2-overrides-explicit-base');
assert(explicit, 'Missing explicit-config precedence acceptance case');
const base = JSON.parse(explicit.files[explicit.explicitConfigPath]);
const discovered = JSON.parse(explicit.files[`${explicit.cwd}/.markdownlint-cli2.jsonc`]);
assert.deepEqual(merge(base, discovered).config, explicit.expectedEffectiveConfig);
const inventory = JSON.parse(fs.readFileSync('docs/compatibility/rules.json', 'utf8'));
for (const id of ['MD001', 'MD025', 'MD041']) {
  const option = inventory.rules.find(rule => rule.id === id).options.find(o => o.name === 'front_matter_title');
  assert.equal(typeof option.sourceDefault, 'string', `${id}: missing source-level default`);
  const pattern = new RegExp(option.sourceDefault, option.sourceFlags);
  assert.equal(pattern.test('"TITLE": A title'), true);
  assert.equal(pattern.test('not_title: A title'), false);
}
const helpers = fs.readFileSync(`${root}/markdownlint/helpers/helpers.cjs`, 'utf8');
const titleSource = helpers.match(/function frontMatterHasTitle[\s\S]*?\n  }/)[0];
const hasTitle = Function(`return (${titleSource});`)();
assert.equal(hasTitle(['"TITLE": A title']), true);
assert.equal(hasTitle(['not_title: A title']), false);
assert.equal(hasTitle(['"TITLE": A title'], '^\\s*title\\s*[:=]'), false);
console.log('PASS: shallow merging, declaration order/severity, explicit-base merge, and title-default source experiments');
JS
```
