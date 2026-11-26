#!/usr/bin/env -S uv run --quiet --script
"""Auto-merge bot for decomp PRs on the self-hosted Forgejo.

Policy, from CONTRIBUTING.md: a PR that only adds files under `decomp/`,
keeps claims and notes consistent, and passes `revi check` is squash-merged
without a human. Anything else is labelled `needs-human` and left alone.

Run from cron or a systemd timer. Needs REVI_TOKEN (an account with write
access), REVI_URL, REVI_REPO and REVI_WORK (the bare repository on disk).
`--once` processes the queue and exits, which is what the timer uses.
"""

import json
import os
import re
import subprocess
import sys
import shutil
import tempfile
import time
import urllib.error
import urllib.request

URL = os.environ.get("REVI_URL", "http://127.0.0.1:3000").rstrip("/")
REPO = os.environ.get("REVI_REPO", "sharif/reVI")
TOKEN = os.environ.get("REVI_TOKEN", "")
WORK = os.environ.get("REVI_WORK", "/home/sharif/srv/revi/bot-work")
REVI = os.environ.get("REVI_BIN", "/srv/revi/revi")
LABEL = "automerger"
HUMAN = "needs-human"

# Files a contributor may touch without a human.
ALLOWED = re.compile(r"^decomp/")
MAX_BYTES = 512 * 1024


def api(method, path, body=None):
    data = None if body is None else json.dumps(body).encode()
    request = urllib.request.Request(
        f"{URL}/api/v1/repos/{REPO}{path}",
        data=data,
        method=method,
        headers={
            "Authorization": f"token {TOKEN}",
            "Content-Type": "application/json",
            "Accept": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            raw = response.read()
            return json.loads(raw) if raw else None
    except urllib.error.HTTPError as error:
        return {"_status": error.code, "_body": error.read().decode()[:500]}


def comment(number, text):
    api("POST", f"/issues/{number}/comments", {"body": text})


def label(number, name, present=True):
    if present:
        api("POST", f"/issues/{number}/labels", {"labels": [name]})
    else:
        api("DELETE", f"/issues/{number}/labels/{name}")


def changed_files(number):
    files, page = [], 1
    while True:
        batch = api("GET", f"/pulls/{number}/files?page={page}&limit=100") or []
        if not isinstance(batch, list) or not batch:
            return files
        files += [(f["filename"], f.get("status")) for f in batch]
        page += 1


def check_pull(pull):
    """Return a list of reasons to refuse. Empty means mergeable."""
    number = pull["number"]
    files = changed_files(number)
    if not files:
        return ["no files"]
    reasons = []
    for name, status in files:
        if status == "removed":
            continue
        if not ALLOWED.match(name):
            reasons.append(f"`{name}` is outside `decomp/`")
        elif not name.startswith("decomp/map/") and not re.search(
            r"\.(c|md|json)$", name
        ):
            reasons.append(f"`{name}` is not `.c`, `.md` or `.json`")
    if reasons:
        return reasons

    fetch = subprocess.run(
        ["git", "-C", WORK, "fetch", "-q", "origin", f"refs/pull/{number}/head"],
        capture_output=True,
        text=True,
    )
    if fetch.returncode:
        return [f"fetch failed: {fetch.stderr.strip()[:200]}"]
    sha = subprocess.run(
        ["git", "-C", WORK, "rev-parse", "FETCH_HEAD"], capture_output=True, text=True
    ).stdout.strip()
    temp = tempfile.mkdtemp(prefix="revi-pr-")
    try:
        add = subprocess.run(
            ["git", "-C", WORK, "worktree", "add", "-q", "--detach", temp, sha],
            capture_output=True,
            text=True,
        )
        if add.returncode:
            return [f"worktree failed: {add.stderr.strip()[:200]}"]
        for name, _ in files:
            path = os.path.join(temp, name)
            if os.path.exists(path) and os.path.getsize(path) > MAX_BYTES:
                reasons.append(f"`{name}` is over {MAX_BYTES} bytes")
        result = subprocess.run(
            [REVI, "check", "decomp/map/functions.csv", "decomp"],
            cwd=temp,
            capture_output=True,
            text=True,
        )
        if result.returncode:
            tail = (result.stdout + result.stderr).strip().splitlines()
            problems = [
                line.strip()
                for line in tail
                if line.startswith("  ") and line.strip()
            ]
            reasons += problems[:12] or ["`revi check` failed"]
    finally:
        subprocess.run(
            ["git", "-C", WORK, "worktree", "remove", "--force", temp],
            capture_output=True,
        )
        shutil.rmtree(temp, ignore_errors=True)
    return reasons


def process(pull):
    number = pull["number"]
    reasons = check_pull(pull)
    if reasons:
        label(number, HUMAN)
        label(number, LABEL, present=False)
        comment(
            number,
            "Left for a human:\n\n"
            + "\n".join(f"* {reason}" for reason in reasons[:15])
            + "\n\nRules: CONTRIBUTING.md.",
        )
        print(f"#{number} needs human: {reasons[0]}")
        return
    merged = api(
        "POST",
        f"/pulls/{number}/merge",
        {"Do": "squash", "MergeTitleField": f"{pull['title']} (#{number})"},
    )
    state = api("GET", f"/pulls/{number}") or {}
    if state.get("merged") or (merged and not merged.get("_status")):
        comment(number, "Merged automatically: `decomp/`-only and `revi check` clean.")
        print(f"#{number} merged")
    else:
        label(number, HUMAN)
        print(f"#{number} merge failed: {merged}")


def main():
    if not TOKEN:
        sys.exit("REVI_TOKEN is not set")
    pulls = api("GET", "/pulls?state=open&limit=50")
    if not isinstance(pulls, list):
        print(f"listing failed: {pulls}")
        return
    for pull in pulls:
        names = [
            label["name"]
            for label in api("GET", f"/issues/{pull['number']}/labels") or []
            if isinstance(label, dict) and "name" in label
        ]
        if LABEL in names:
            process(pull)


def watch(interval):
    while True:
        try:
            main()
        except Exception as error:  # keep the timer alive
            print(f"error: {error}")
        time.sleep(interval)


if __name__ == "__main__":
    main() if "--once" in sys.argv else watch(int(os.environ.get("REVI_INTERVAL", "120")))
