#!/usr/bin/env python3
"""Offline analysis entry for 0070-ssao-filter-002 (after release only).

Runs ssao-repair/filter/analyze_filter.py unchanged; the only binding is its
plan check, which must compare the evidence's build record with this
namespace's plan.json rather than 0070-ssao-filter-001's. Exit status, report
format and every gate are analyze_filter.py's.

  python3 -I -B analyze.py <evidence-dir>
"""

import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE.parent / "ssao-repair" / "filter")]
import analyze_filter  # noqa: E402

analyze_filter.NAMESPACE_PLAN = HERE / "plan.json"

if __name__ == "__main__":
    sys.exit(analyze_filter.main())
