#!/usr/bin/env python3
"""Explicit trusted nonshipping Core C-PGO feasibility; no performance acceptance."""
import argparse
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from quickjs_pgo_build import prepare

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--request', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--scratch', type=Path, required=True)
    parser.add_argument('--seconds', type=int, default=5400)
    args = parser.parse_args()
    prepare(args.request, args.output, args.scratch, seconds=args.seconds)
