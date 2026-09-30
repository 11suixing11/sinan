#!/usr/bin/env python3
"""Exercise the actual Agent's independent OpenRC one-shot services in a disposable VM."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid


def main():
    assert os.environ.get("SINAN_OPENRC_SMOKE") == "1", "requires a disposable OpenRC environment"
    binary = str(Path(sys.argv[1]).resolve())
    host_namespace = os.readlink("/proc/self/ns/mnt")
    with tempfile.TemporaryDirectory(prefix="sinan-openrc-jobs-") as directory:
        root = Path(directory)
        for program, args, seconds, succeeds in [
            ("/bin/sh", ["-c", f"readlink /proc/self/ns/mnt > {root}/namespace"], 10, True),
            ("/bin/false", [], 10, False),
            ("/bin/sh", ["-c", "sleep 30 & wait"], 1, False),
        ]:
            unit = f"sinan-diagnostic-{uuid.uuid4()}.service"
            job = dict(unit=unit, program=program, args=args, working_directory=directory, timeout_secs=seconds)
            spec = root / f"{unit}.json"
            spec.write_text(json.dumps(job))
            subprocess.run([binary, "service-job", "--spec", str(spec)], check=True, timeout=20)
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                result = subprocess.run([binary, "service-job", "--spec", str(spec), "--status"], capture_output=True, text=True, check=True, timeout=10)
                status = json.loads(result.stdout)
                if status != "running":
                    break
                time.sleep(0.2)
            assert (status == "succeeded") == succeeds, (job, status)
            if not succeeds:
                assert isinstance(status, dict) and "failed" in status, status
            if seconds == 1:
                assert "timeout" in status["failed"]["error"], status
            if succeeds:
                assert (root / "namespace").read_text().strip() != host_namespace
            repeated = subprocess.run([binary, "service-job", "--spec", str(spec)], capture_output=True, timeout=20)
            assert repeated.returncode != 0, "a completed job must not run again"
            subprocess.run(["rc-service", "--", unit.removesuffix(".service"), "stop"], capture_output=True, timeout=10)
        print("Actual Agent OpenRC jobs: independent lifetime, mount isolation, success, failure, timeout, and replay rejection passed")


if __name__ == "__main__":
    main()
