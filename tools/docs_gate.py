#!/usr/bin/env python3
"""Run a trusted base checkout's gate against a separate candidate checkout."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from docs_gate_lib import main

if __name__ == '__main__':
    raise SystemExit(main())
