#!/usr/bin/env python3
import json, sys
from pathlib import Path
path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path('plugin/openirl-plugin-manifest.schema.json')
json.loads(path.read_text(encoding='utf-8'))
print('plugin manifest parsed')
