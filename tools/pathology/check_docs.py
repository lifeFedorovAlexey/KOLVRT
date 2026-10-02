"""Check local links, source-ledger consistency and the required law/ADR sections."""
import re
import sys
from urllib.parse import unquote

from validate import ROOT, read_json


def check():
    errors = []
    markdown = sorted(ROOT.rglob('*.md'))
    for path in markdown:
        if '.git' in path.parts or '.venv' in path.parts:
            continue
        text = path.read_text(encoding='utf-8')
        for target in re.findall(r'\[[^\]]*\]\(([^)]+)\)', text):
            if re.match(r'^[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'):
                continue
            local = unquote(target.split('#')[0]).strip('<>')
            if not (path.parent / local).exists():
                errors.append(f'{path.relative_to(ROOT)}: missing link {target}')
    law_path = ROOT / 'docs/architecture/KERNEL_LAWS.md'
    law_text = law_path.read_text(encoding='utf-8')
    blocks = re.split(r'^## (LAW-\d{3}) — ', law_text, flags=re.MULTILINE)
    ids = blocks[1::2]
    if not 30 <= len(ids) <= 50 or len(ids) != len(set(ids)):
        errors.append('Kernel laws: expected 30..50 unique IDs')
    fields = ('Rule', 'Rationale', 'Historical evidence', 'Prevents', 'Allowed exceptions', 'Enforcement', 'Testing')
    for ident, body in zip(ids, blocks[2::2]):
        for field in fields:
            if not re.search(r'\*\*' + re.escape(field) + r':\*\*\s+\S', body):
                errors.append(f'{ident}: missing {field}')
        if 'research/pathology/KOL-PATH-' not in body:
            errors.append(f'{ident}: missing case evidence')
    adrs = sorted((ROOT / 'docs/adr').glob('[0-9][0-9][0-9][0-9]-*.md'))
    if not adrs:
        errors.append('No ADRs found')
    for path in adrs:
        text = path.read_text(encoding='utf-8')
        for field in ('Context', 'Decision', 'Alternatives', 'Why rejected', 'Consequences',
                      'Compatibility impact', 'Performance impact', 'Security impact', 'Testing', 'Reversibility'):
            if f'## {field}\n' not in text:
                errors.append(f'{path.name}: missing {field}')
    ledger = read_json(ROOT / 'research/sources/linux-sources.json')
    catalog = {s['id']: s for s in ledger}
    if len(catalog) != len(ledger):
        errors.append('Duplicate source ID in ledger')
    for path in sorted((ROOT / 'research/pathology').glob('*.json')):
        case = read_json(path)
        for source in case['sources']:
            entry = {k: v for k, v in source.items() if k != 'subsystem'}
            if catalog.get(source['id']) != entry:
                errors.append(f'{case["id"]}: source differs from ledger: {source["id"]}')
    return errors, len(markdown), len(ids), len(adrs)


def main():
    try:
        errors, documents, laws, adrs = check()
    except (OSError, ValueError, KeyError) as exc:
        print(f'Document checks could not complete: {exc}', file=sys.stderr)
        return 2
    if errors:
        print('\n'.join(errors), file=sys.stderr)
        return 1
    print(f'Checked {documents} Markdown files, {laws} laws, {adrs} ADRs, local links and source consistency.')
    print('External URL contents and historical truth require manual review.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
