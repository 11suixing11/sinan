#!/usr/bin/env python3
"""Check the tagged CI commit and recheck a complete signed draft before publication."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import urllib.parse

from release import (MAX_BINARY, REPOSITORY, VERSION, digest, ensure, load_roots,
                     read_regular, verify_bundle)

REQUIRED_JOBS = frozenset(("check", "compose-smoke", "Agent musl (amd64)",
                          "Agent musl (arm64)", "Reality installation and accounting"))


class Github:
    def command(self, endpoint, method="GET", fields=(), binary=False):
        command = ["gh", "api", "--hostname", "github.com", "--method", method,
                   "-H", "X-GitHub-Api-Version: 2022-11-28", "-H",
                   "Accept: application/octet-stream" if binary else
                   "Accept: application/vnd.github+json", endpoint]
        for field in fields:
            command.extend(("-F", field))
        return command

    def environment(self):
        environment = dict(os.environ)
        for name in ("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy",
                     "https_proxy", "all_proxy"):
            environment.pop(name, None)
        environment["GH_HOST"] = "github.com"
        return environment

    def api(self, endpoint, method="GET", fields=()):
        result = subprocess.run(self.command(endpoint, method, fields),
                                capture_output=True, timeout=120,
                                env=self.environment(), check=False)
        ensure(result.returncode == 0, "GitHub API request failed")
        ensure(len(result.stdout) <= 4 * 1024 * 1024, "GitHub response is too large")
        return json.loads(result.stdout)

    def download(self, asset, destination):
        # Select immutable asset ID, not a mutable tag/name download lookup.
        with destination.open("xb") as output:
            result = subprocess.run(self.command(
                f"repos/{REPOSITORY}/releases/assets/{asset['id']}", binary=True),
                stdout=output, stderr=subprocess.PIPE, timeout=300,
                env=self.environment(), check=False)
        ensure(result.returncode == 0, "GitHub asset download failed")


def tag_identity(github, tag):
    ensure(tag.startswith("agent-v") and VERSION.fullmatch(tag[7:]), "invalid release tag")
    value = github.api(f"repos/{REPOSITORY}/git/ref/tags/{urllib.parse.quote(tag, safe='')}")
    ensure(value.get("ref") == "refs/tags/" + tag, "tag reference does not match")
    initial = value["object"]
    current = initial
    for _ in range(5):
        ensure(isinstance(current, dict) and re.fullmatch(r"[0-9a-f]{40}", current["sha"]),
               "invalid Git tag object")
        if current["type"] == "commit":
            return {"ref_sha": initial["sha"], "ref_type": initial["type"],
                    "commit": current["sha"]}
        ensure(current["type"] == "tag", "tag does not point to a commit")
        current = github.api(f"repos/{REPOSITORY}/git/tags/{current['sha']}")["object"]
    raise ValueError("tag object nesting is too deep")


def required_checks(github, commit):
    query = urllib.parse.urlencode({"head_sha": commit, "branch": "main", "event": "push",
                                    "per_page": 100})
    runs = github.api(f"repos/{REPOSITORY}/actions/workflows/ci.yml/runs?{query}")
    ensure(type(runs.get("total_count")) is int and 0 < runs["total_count"] <= 100,
           "missing or excessive CI runs for the tagged main commit")
    candidates = [run for run in runs["workflow_runs"] if run.get("head_sha") == commit
                  and run.get("head_branch") == "main" and run.get("event") == "push"
                  and run.get("path", "").split("@")[0] == ".github/workflows/ci.yml"]
    ensure(candidates, "tagged commit has no main push CI evidence")
    run = max(candidates, key=lambda candidate: candidate["id"])
    ensure(run["status"] == "completed" and run["conclusion"] == "success",
           "latest tagged main commit CI has not passed")
    result = github.api(f"repos/{REPOSITORY}/actions/runs/{run['id']}/jobs?filter=latest&per_page=100")
    ensure(type(result.get("total_count")) is int and result["total_count"] <= 100,
           "excessive CI jobs")
    jobs = {}
    for job in result["jobs"]:
        if job["name"] in REQUIRED_JOBS:
            ensure(job["name"] not in jobs, "duplicate required CI job")
            ensure(job.get("head_sha") == commit and job["status"] == "completed"
                   and job["conclusion"] == "success", "required CI job has not passed")
            jobs[job["name"]] = job["id"]
    ensure(set(jobs) == REQUIRED_JOBS, "a required CI job is missing")
    return {"run_id": run["id"], "run_attempt": run["run_attempt"], "jobs": jobs}


def release_snapshot(github, tag):
    value = github.api(f"repos/{REPOSITORY}/releases/tags/{urllib.parse.quote(tag, safe='')}")
    ensure(type(value["id"]) is int and value["id"] > 0 and value["tag_name"] == tag,
           "wrong release identity")
    ensure(isinstance(value["target_commitish"], str)
           and re.fullmatch(r"[0-9a-f]{40}", value["target_commitish"]),
           "release draft must record the exact build commit")
    assets = github.api(f"repos/{REPOSITORY}/releases/{value['id']}/assets?per_page=100")
    ensure(isinstance(assets, list) and 0 < len(assets) <= 20, "invalid release asset count")
    normalized, names, identifiers = [], set(), set()
    for asset in assets:
        ensure(type(asset["id"]) is int and asset["id"] > 0
               and asset["id"] not in identifiers, "invalid or duplicate asset ID")
        ensure(isinstance(asset["name"], str) and
               re.fullmatch(r"[0-9A-Za-z][0-9A-Za-z.+_-]*", asset["name"])
               and asset["name"] not in names, "invalid or duplicate asset name")
        ensure(type(asset["size"]) is int and 0 < asset["size"] <= MAX_BINARY
               and asset["state"] == "uploaded", "incomplete or oversized asset")
        ensure(isinstance(asset["digest"], str)
               and re.fullmatch(r"sha256:[0-9a-f]{64}", asset["digest"]), "asset lacks a SHA256 digest")
        normalized.append({field: asset[field] for field in ("id", "name", "size", "digest", "state")})
        identifiers.add(asset["id"])
        names.add(asset["name"])
    return {"id": value["id"], "tag": tag, "draft": value["draft"],
            "build_commit": value["target_commitish"],
            "assets": sorted(normalized, key=lambda asset: asset["name"])}


def require_components(metadata):
    identities = {(entry["name"], entry["arch"]) for entry in metadata["artifacts"]}
    ensure(len(metadata["artifacts"]) == 6 and identities == {
        (name, arch) for name in ("agent", "sing-box", "nodequality")
        for arch in ("amd64", "arm64")}, "release must contain every module on both architectures")


def checked_publication(github, tag, roots, minisign, publish=False):
    identity = tag_identity(github, tag)
    checks = required_checks(github, identity["commit"])
    before = release_snapshot(github, tag)
    ensure(before["draft"] is True, "only a draft can enter final publication verification")
    ensure(before["build_commit"] == identity["commit"], "tag differs from the draft build commit")
    with tempfile.TemporaryDirectory(prefix="sinan-publication-") as temporary:
        bundle = Path(temporary)
        for asset in before["assets"]:
            path = bundle / asset["name"]
            github.download(asset, path)
            data = read_regular(path)
            ensure(len(data) == asset["size"] and "sha256:" + digest(data) == asset["digest"],
                   "download differs from selected GitHub asset ID and digest")
        metadata = verify_bundle(bundle, roots, minisign, tag)
        require_components(metadata)
    # Re-fetch every identity and check immediately before the only mutation.
    ensure(tag_identity(github, tag) == identity, "tag changed during release verification")
    ensure(required_checks(github, identity["commit"]) == checks,
           "CI evidence changed during release verification")
    ensure(release_snapshot(github, tag) == before, "release assets changed during verification")
    evidence = {"tag": tag, "tag_identity": identity, "checks": checks, "release": before,
                "published": False}
    if publish:
        result = github.api(f"repos/{REPOSITORY}/releases/{before['id']}", "PATCH", ("draft=false",))
        ensure(result["id"] == before["id"] and result["draft"] is False, "publication request failed")
        after = release_snapshot(github, tag)
        ensure(after == dict(before, draft=False) and tag_identity(github, tag) == identity,
               "publication state changed across the final API call; manual incident review required")
        evidence["published"] = True
    return evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check-tag")
    check.add_argument("--tag", required=True)
    check.add_argument("--expected-sha", required=True)
    publish = commands.add_parser("verify-publication")
    publish.add_argument("--tag", required=True)
    publish.add_argument("--trusted-keys", required=True)
    publish.add_argument("--minisign", default="minisign")
    publish.add_argument("--publish", action="store_true")
    parser.add_argument("--evidence")
    args = parser.parse_args()
    github = Github()
    if args.command == "check-tag":
        identity = tag_identity(github, args.tag)
        ensure(identity["commit"] == args.expected_sha, "tag differs from build commit")
        evidence = {"tag_identity": identity, "checks": required_checks(github, identity["commit"])}
    else:
        roots = load_roots(args.trusted_keys, publication=True)
        evidence = checked_publication(github, args.tag, roots, args.minisign, args.publish)
    if args.evidence:
        Path(args.evidence).write_text(json.dumps(evidence, sort_keys=True) + "\n")
    print("Tagged CI commit and release publication checks passed.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, json.JSONDecodeError, subprocess.TimeoutExpired) as error:
        raise SystemExit(f"Publication refused: {error}") from error
