#!/usr/bin/env python3
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

COMMANDS = {
    "traditional_demo": "traditional",
    "zk_demo": "generate",
    "verify_e2e": "verify-e2e",
    "integrity_cases": "integrity-cases",
    "availability_benchmark": "availability-benchmark",
    "compare": "compare",
}


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] not in COMMANDS:
        names = " | ".join(COMMANDS)
        print(f"Usage: python script/zk_auth_cli.py <{names}> [args...]", file=sys.stderr)
        return 2

    subcommand = COMMANDS[sys.argv[1]]
    args = sys.argv[2:]
    return subprocess.call(
        ["cargo", "run", "--bin", "zk_auth_demo", "--", subcommand, *args],
        cwd=ROOT,
    )


if __name__ == "__main__":
    raise SystemExit(main())
