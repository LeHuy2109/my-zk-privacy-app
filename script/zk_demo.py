#!/usr/bin/env python3
from zk_auth_cli import main
import sys

if __name__ == "__main__":
    sys.argv.insert(1, "zk_demo")
    raise SystemExit(main())
