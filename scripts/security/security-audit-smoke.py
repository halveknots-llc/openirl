#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[2]
config = (root / 'config/openirl.example.toml').read_text(encoding='utf-8')
assert '127.0.0.1:7707' in config
assert 'redact_logs = true' in config
print('security audit smoke passed')
