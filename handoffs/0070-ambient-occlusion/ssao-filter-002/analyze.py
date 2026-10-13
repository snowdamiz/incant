#!/usr/bin/env python3
"""Offline analysis entry for 0070-ssao-filter-002 (after release only).

Runs the shared ssao-repair/filter/analyze_filter.py gates through
c2_analysis.py: the C2 oracle, this namespace's plan check, the revised N6T
measure with authored N6 retained, and every screened N8 row. Shared modules
are imported read only and refused unless their bytes match the pins.

  python3 -I -B analyze.py <evidence-dir>
  python3 -I -B analyze.py --selftest
"""

import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path[:0] = [str(Path(__file__).resolve().parent)]
import c2_analysis  # noqa: E402

if __name__ == "__main__":
    sys.exit(c2_analysis.main())
