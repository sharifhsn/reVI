# Windows test box

VIctory needs real Windows for the parts WARP and a Linux shell cannot prove:
D3D12 device creation on a vendor driver, `LoadLibrary` of the title's DLLs,
`SetupAPI`, the package and licence APIs, speech, and anything that hangs the
desktop when it goes wrong.

`tools/windows-lab/` is the driver for that box. It is host-side, scripted and
verified by screenshots, so a test run is reproducible and reviewable instead
of "it worked on my machine".

## Live instance

One box exists and is up:

| | |
|---|---|
| Host | `aster` (Debian, KVM) |
| VM | `leonida-win`, Windows 11 24H2 build 26100.9457, 6 vCPU / 8 GB |
| Driver | `e1000e` NIC, `std` VGA, QEMU 10.0.13 |
| Access | `vmctl` on the host, ssh on `127.0.0.1:2222` |
| Account | `leonida`, administrator |
| GPU | none passed through yet — WARP or llvmpipe only |

```sh
ssh aster
sudo /home/sharif/lab/vmctl status
sudo /home/sharif/lab/vmctl shot
```

Environment for the box lives in `/home/sharif/lab/lab.env`; the scripts and
the gotcha list are in that directory and mirrored here in
`tools/windows-lab/`.

## What it needs before GPU work

The RX 6700 XT on `aster` is not passed through. Until it is, `VICTORY_D3D12_WARP`
runs and real-adapter behaviour does not. Adding it means `vfio-pci` on the
card, a second `-device vfio-pci` in `vmctl start`, and a monitor attached to
the card's HDMI — the host then loses it.

## Non-goals

No GUI client, no snapshots per test, no CI hookup from the guest. The box is
for a human or an agent to drive deliberately and read the screenshots.

## Budget

Boot to desktop is about 40 s on this host, and window launches take up to
20 s each. A test that needs three windows costs a minute before the title
runs. Prefer one elevated shell and drive everything from it.
