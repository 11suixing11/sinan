#!/usr/bin/env python3
"""Keep command output and publish bounded failure details as CI annotations."""
from collections import deque
import os
import subprocess
import sys


def main():
    if len(sys.argv) < 2:
        raise SystemExit('usage: ci-run.py COMMAND [ARG...]')
    recent = deque(maxlen=120)
    with subprocess.Popen(sys.argv[1:], stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          text=True, encoding='utf-8', errors='replace') as child:
        for line in child.stdout:
            print(line, end='', flush=True)
            recent.append(line)
        code = child.wait()
    if code and os.environ.get('GITHUB_ACTIONS') == 'true':
        # GitHub truncates individual check annotations at 4096 characters.
        message = ''.join(recent)[-24000:]
        chunks = [message[i:i + 3000] for i in range(0, len(message), 3000)]
        for index, chunk in enumerate(chunks, 1):
            escaped = chunk.replace('%', '%25').replace('\r', '%0D').replace('\n', '%0A')
            print(f'::error title=Failure output {index}/{len(chunks)}::{escaped}', flush=True)
    raise SystemExit(code)


if __name__ == '__main__':
    main()
