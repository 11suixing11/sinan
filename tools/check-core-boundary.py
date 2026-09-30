#!/usr/bin/env python3
"""Reject proxy business references in core while preserving native account APIs."""
import argparse
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN = {"user", "users", "subscription", "subscriptions", "quota", "quotas"}
TOKENS = re.compile(r"[A-Za-z][A-Za-z0-9_]*")
CAMEL_PARTS = re.compile(r"_|(?<=[a-z0-9])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])")

# Exact operating-system/SQLite API spellings, scoped to their existing callers.
# Mask only these expressions: a forbidden reference on the same line still fails.
EXCEPTIONS = {
    "src/system/windows.rs": (
        r"\[Security\.Principal\.WindowsIdentity\]::GetCurrent\(\)\.User\b",
    ),
    "src/system/deploy/native/windows.rs": (
        r"\[Security\.Principal\.WindowsIdentity\]::GetCurrent\(\)\.User\b",
        r"\b(?:Get|New|Set|Enable)-LocalUser\b",
        r"(?<![\w-])-(?:UserMayNotChangePassword|UserId|User)\b",
        r"\bUSER_RIGHTS\b",
    ),
    "src/system/deploy/native/unix.rs": (
        r"/Users\b",
        r'"UserShell"',
        r"<key>UserName</key>",
    ),
    "tests/usage.rs": (r"\bPRAGMA user_version\b",),
    "tests/usage_bounds.rs": (
        r"\bPRAGMA user_version\b",
        r'\.pragma_update\(None, "user_version",',
    ),
}


def violations(relative, source):
    findings = []
    for number, line in enumerate(source.splitlines(), 1):
        if re.search(r"singbox|sing-box", line, re.IGNORECASE):
            findings.append((number, "runtime name"))
        for expression in EXCEPTIONS.get(relative, ()):
            line = re.sub(expression, "", line)
        for token in TOKENS.findall(line):
            parts = {part.lower() for part in CAMEL_PARTS.split(token)}
            banned = sorted(parts & FORBIDDEN)
            if banned:
                findings.append((number, "/".join(banned)))
    return findings


def check(directory):
    failed = False
    for path in sorted(directory.rglob("*")):
        if path.is_symlink():
            print(f"{path}: symlinks are not allowed in core", file=sys.stderr)
            failed = True
        elif path.is_file():
            relative = path.relative_to(directory).as_posix()
            try:
                source = path.read_text(encoding="utf-8")
            except UnicodeError:
                print(f"{path}: unreadable core source", file=sys.stderr)
                failed = True
                continue
            for line, reason in violations(relative, source):
                print(f"{path}:{line}: forbidden core reference: {reason}", file=sys.stderr)
                failed = True
    return not failed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--core", type=Path, default=ROOT / "crates/agent-core")
    args = parser.parse_args()
    if not args.core.is_dir():
        parser.error("core directory is missing")
    return 0 if check(args.core) else 1


if __name__ == "__main__":
    sys.exit(main())
