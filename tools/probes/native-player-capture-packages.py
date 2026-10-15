#!/usr/bin/env python3
"""Build up to four exact scene-bound capture-capable packages; CPU startup only.

This is a trusted host diagnostic, not a project-selected build/cache interface.
Each job is {"name": "ar", "project": "/absolute/project.incant"}, optionally
with canonical scene/camera ULIDs. All build/input copies are removed before any
startup. Actual selected-frame rendering requires a separate execution grant.
"""
import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from native_export_session import jobs_from_file, prepare


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('jobs', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--seconds', type=int, default=3600)
    args = parser.parse_args()
    result = prepare(jobs_from_file(args.jobs), args.output, seconds=args.seconds)
    print(json.dumps(result))


if __name__ == '__main__':
    main()
