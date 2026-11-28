#!/usr/bin/env python3
"""Summarize GitHub PR checks.

Usage: pr-check-status.py <checks.json>

The JSON is the output of `gh pr checks --json name,state,bucket,link`.
Prints `STATE|detail` first, then one `FAILED\tname\tlink` line per failed
check. STATE is GREEN, RED, or RUNNING.

A missing or empty check list is unknown/running, never green: the caller must
wait for checks to appear rather than merging on an unreadable response.
"""
import json
import sys


def load(path):
    try:
        with open(path) as handle:
            data = json.load(handle)
    except Exception:
        return None
    return data if isinstance(data, list) else None


def summarize(checks):
    if checks is None:
        return ["RUNNING|checks unreadable; not judging"], []
    if not checks:
        return ["RUNNING|no checks reported yet"], []

    failed = [c for c in checks if c.get("bucket") in ("fail", "cancel")]
    pending = [c for c in checks
               if c.get("bucket") not in ("pass", "skipping")]
    if failed:
        detail = "{} of {} checks failed".format(len(failed), len(checks))
        return [f"RED|{detail}"], failed
    if pending:
        detail = "{} of {} checks pending".format(len(pending), len(checks))
        running = [c.get("name", "?") for c in pending][:4]
        if running:
            detail += " · " + ", ".join(running)
        return [f"RUNNING|{detail}"], []
    return [f"GREEN|{len(checks)} checks passed"], []


def main(argv):
    if len(argv) != 2:
        print("usage: pr-check-status.py <checks.json>", file=sys.stderr)
        return 2
    lines, failed = summarize(load(argv[1]))
    print("\n".join(lines))
    for check in failed:
        print("FAILED\t{}\t{}".format(check.get("name", "?"),
                                      check.get("link", "")))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
