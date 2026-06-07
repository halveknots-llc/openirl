#!/usr/bin/env python3
"""Static validation for the OpenIRL handoff repository."""
from __future__ import annotations
import json, re, sys, tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TEXT_SUFFIXES = {'.rs','.md','.toml','.yml','.yaml','.json','.sh','.ps1','.py','.html','.txt','.conf'}

def joined(parts):
    return ''.join(parts)

DENIED_TERMS = [
    joined(['T','O','D','O']), joined(['F','I','X','M','E']), joined(['X','X','X']),
    joined(['s','t','u','b']), joined(['p','l','a','c','e','h','o','l','d','e','r']),
    joined(['n','o','t',' ','i','m','p','l','e','m','e','n','t','e','d']),
    joined(['d','r','y','-','r','u','n','-','o','n','l','y']),
    joined(['R','e','a','d','y','F','o','r','S','m','o','k','e']),
    joined(['N','e','e','d','s','L','i','v','e','V','a','l','i','d','a','t','i','o','n']),
]
LEGACY_TERMS = [
    joined(['I','M','P','L','E','M','E','N','T','A','T','I','O','N','_','W','A','V','E']),
    joined(['P','R','E','_','C','O','D','E','X','_','W','A','V','E']),
    joined(['p','r','e','_','c','o','d','e','x','_','w','a','v','e','s']),
]
LEGACY_WORD = joined(['w','a','v','e'])

def files():
    for path in ROOT.rglob('*'):
        if path.is_file() and path.suffix.lower() in TEXT_SUFFIXES and 'target' not in path.parts and '.git' not in path.parts:
            yield path

def rel(path: Path) -> str:
    return str(path.relative_to(ROOT)).replace('\\','/')

def main() -> int:
    findings=[]
    for path in ROOT.rglob('*.json'):
        if 'target' in path.parts: continue
        try: json.loads(path.read_text(encoding='utf-8'))
        except Exception as exc: findings.append((rel(path),'json',str(exc)))
    for path in ROOT.rglob('*.toml'):
        if 'target' in path.parts: continue
        try: tomllib.loads(path.read_text(encoding='utf-8'))
        except Exception as exc: findings.append((rel(path),'toml',str(exc)))
    denied = [re.compile(re.escape(term), re.I) for term in DENIED_TERMS + LEGACY_TERMS]
    legacy_word = re.compile(re.escape(LEGACY_WORD), re.I)
    for path in files():
        relative = rel(path)
        if legacy_word.search(relative):
            findings.append((relative,'filename','legacy numbered-pass label in filename'))
        text=path.read_text(encoding='utf-8', errors='replace')
        for idx,line in enumerate(text.splitlines(),1):
            if 'Redacted password value sample' in line:
                continue
            if legacy_word.search(line):
                findings.append((f'{relative}:{idx}','legacy-label',line.strip()[:160]))
                continue
            for pat in denied:
                if pat.search(line):
                    findings.append((f'{relative}:{idx}','marker',line.strip()[:160]))
                    break
    required = [
        'README.md','CODEX_TASKS.md','Cargo.toml','apps/openirl-agent/src/main.rs',
        'crates/openirl-v1/src/lib.rs','docs/ARCHITECTURE.md','docs/SECURITY.md',
        'docs/VALIDATION.md','docs/features/obs-reconciliation.md','scripts/audit/handoff_audit.py'
    ]
    for item in required:
        if not (ROOT/item).exists():
            findings.append((item,'inventory','missing required file'))
    if findings:
        print('static validation: fail')
        for loc,kind,msg in findings[:100]:
            print(f'[{kind}] {loc}: {msg}')
        if len(findings)>100: print(f'... {len(findings)-100} additional findings')
        return 1
    print('static validation: pass')
    return 0
if __name__ == '__main__':
    raise SystemExit(main())
