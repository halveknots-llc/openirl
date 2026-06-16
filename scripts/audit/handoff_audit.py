#!/usr/bin/env python3
"""Strict source-readiness audit for OpenIRL."""
from __future__ import annotations
import json, re, shutil, sys, tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TEXT_SUFFIXES = {'.rs','.md','.toml','.yml','.yaml','.json','.sh','.ps1','.py','.html','.txt','.conf'}

def joined(parts):
    return ''.join(parts)

DENIED_CODE = {
    'task_macro': re.compile(r'(?<![A-Za-z0-9_])' + joined(['t','o','d','o']) + r'!\s*\('),
    'missing_impl_macro': re.compile(r'(?<![A-Za-z0-9_])' + joined(['u','n','i','m','p','l','e','m','e','n','t','e','d']) + r'!\s*\('),
    'panic_macro': re.compile(r'(?<![A-Za-z0-9_])' + joined(['p','a','n','i','c']) + r'!\s*\('),
    'unwrap_call': re.compile(r'(?<![A-Za-z0-9_])' + joined(['u','n','w','r','a','p']) + r'\s*\('),
    'expect_call': re.compile(r'(?<![A-Za-z0-9_])' + joined(['e','x','p','e','c','t']) + r'\s*\('),
}
DENIED_TERMS = [
    joined(['T','O','D','O']), joined(['F','I','X','M','E']), joined(['X','X','X']),
    joined(['s','t','u','b']), joined(['p','l','a','c','e','h','o','l','d','e','r']),
    joined(['n','o','t',' ','i','m','p','l','e','m','e','n','t','e','d']),
    joined(['d','r','y','-','r','u','n','-','o','n','l','y']),
    joined(['R','e','a','d','y','F','o','r','S','m','o','k','e']),
    joined(['N','e','e','d','s','L','i','v','e','V','a','l','i','d','a','t','i','o','n']),
    joined(['I','M','P','L','E','M','E','N','T','A','T','I','O','N','_','W','A','V','E']),
    joined(['P','R','E','_','C','O','D','E','X','_','W','A','V','E']),
]
LEGACY_WORD = joined(['w','a','v','e'])

def rel(path: Path) -> str:
    return str(path.relative_to(ROOT)).replace('\\','/')

def text_files():
    for path in ROOT.rglob('*'):
        if path.name.startswith('._'):
            continue
        if path.is_file() and path.suffix.lower() in TEXT_SUFFIXES and 'target' not in path.parts and '.git' not in path.parts:
            yield path

def main() -> int:
    findings=[]
    for path in ROOT.rglob('*.json'):
        if path.name.startswith('._'):
            continue
        if 'target' in path.parts: continue
        try: json.loads(path.read_text(encoding='utf-8'))
        except Exception as exc: findings.append({'path':rel(path),'category':'json','message':str(exc)})
    for path in ROOT.rglob('*.toml'):
        if path.name.startswith('._'):
            continue
        if 'target' in path.parts: continue
        try: tomllib.loads(path.read_text(encoding='utf-8'))
        except Exception as exc: findings.append({'path':rel(path),'category':'toml','message':str(exc)})
    for path in ROOT.rglob('*.rs'):
        if path.name.startswith('._'):
            continue
        if 'target' in path.parts: continue
        text=path.read_text(encoding='utf-8', errors='replace')
        for name,pat in DENIED_CODE.items():
            for match in pat.finditer(text):
                findings.append({'path':rel(path),'line':text.count('\n',0,match.start())+1,'category':name,'message':'denied Rust pattern'})
    denied = [re.compile(re.escape(term), re.I) for term in DENIED_TERMS]
    legacy_word = re.compile(re.escape(LEGACY_WORD), re.I)
    for path in text_files():
        relative = rel(path)
        if legacy_word.search(relative):
            findings.append({'path':relative,'category':'legacy-filename','message':'legacy numbered-pass filename'})
        for idx,line in enumerate(path.read_text(encoding='utf-8', errors='replace').splitlines(),1):
            if 'Redacted password value sample' in line:
                continue
            if legacy_word.search(line):
                findings.append({'path':relative,'line':idx,'category':'legacy-label','message':line.strip()[:200]})
                continue
            for pat in denied:
                if pat.search(line):
                    findings.append({'path':relative,'line':idx,'category':'denied-marker','message':line.strip()[:200]})
                    break
    required_docs = ['docs/features/obs-reconciliation.md','docs/features/local-ingest.md','docs/features/encoder-profiles.md','docs/features/dashboard.md','docs/features/security.md','docs/features/brownout.md','docs/VALIDATION.md','docs/RELEASE_CHECKLIST.md']
    for item in required_docs:
        if not (ROOT/item).exists():
            findings.append({'path':item,'category':'inventory','message':'required source-readiness doc missing'})
    report={'status':'pass' if not findings else 'fail','findings':findings,'tooling':{tool:shutil.which(tool) for tool in ['cargo','rustc','rustfmt']}}
    out_json=ROOT/'audit/handoff-audit.json'
    out_md=ROOT/'audit/HANDOFF_AUDIT.md'
    out_json.parent.mkdir(parents=True, exist_ok=True)
    out_json.write_text(json.dumps(report, indent=2, sort_keys=True), encoding='utf-8')
    out_md.write_text('# OpenIRL Source Readiness Audit\n\n**Status:** '+report['status'].upper()+'\n\nFindings: '+str(len(findings))+'\n', encoding='utf-8')
    print('handoff audit:', report['status'])
    if findings:
        for item in findings[:80]:
            loc=item['path']+(f":{item.get('line')}" if item.get('line') else '')
            print(f"[{item['category']}] {loc}: {item['message']}")
        if len(findings)>80: print(f'... {len(findings)-80} additional findings')
        return 1
    return 0
if __name__ == '__main__':
    raise SystemExit(main())
