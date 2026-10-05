#!/usr/bin/env python3
"""Read a gate verdict.json for run-pipeline.sh.

Usage: verdict_info.py <verdict.json> signature|alerts-count|alerts|failed
"""

import json
import sys


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    with open(argv[1]) as f:
        d = json.load(f)
    steps = d["steps"]
    what = argv[2]
    if what == "signature":
        # Everything that must be identical between repeated gate runs.
        print(d["verdict"], *[f"{s['step']}={s['ok']}" for s in steps])
    elif what == "alerts-count":
        print(sum(len(s.get("open_alerts", [])) for s in steps))
    elif what == "alerts":
        for s in steps:
            for a in s.get("open_alerts", []):
                print(f"    {a['file']}:{a['line']}: {a['query']}: {a['message'][:150]}")
    elif what == "failed":
        print(",".join(s["step"] for s in steps if not s["ok"]))
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
