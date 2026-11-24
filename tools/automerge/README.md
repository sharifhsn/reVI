# automerge

Merges decomp-only pull requests that pass `revi check`, so a queue of
one-function PRs never waits on a human. Policy lives in
[CONTRIBUTING.md](../../CONTRIBUTING.md); the only judgement here is mechanical.

* A PR is considered when it carries the `automerger` label.
* Anything outside `decomp/` (Cargo.toml, tools, CI) or with `revi check`
  problems is labelled `needs-human` and commented with reasons.
* Merges are squashes. The bot never pushes to a branch.

Deployment: run it on the Forgejo host as a systemd user timer, with the
environment from `revi-automerge.env` (mode 0600). `--once` runs a single pass;
without it the process loops every `REVI_INTERVAL` seconds.

The gate is the part worth testing locally: `revi check decomp/map/functions.csv
decomp` must pass on the branch before anything is merged. There is no CI
runner in the loop by design, so a compromised runner cannot merge.
