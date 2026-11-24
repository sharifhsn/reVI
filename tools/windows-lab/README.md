# Windows lab VM

A headless Windows 11 box on the Debian lab host, driven entirely over
QEMU's monitor, VNC and an HTTP channel. No GUI client, no clicking around
by hand: every step is a command you can script, and every step is verified
by a screenshot before the next one runs.

Use it for anything VIctory needs a real Windows (real D3D12, real
`LoadLibrary`, real `SetupAPI`) under.

## Layout

| File | Role |
|------|------|
| `vmctl` | All operations: boot, screenshot, type, click, transfer, power off |
| `remote/type.py` | Characters to QEMU `sendkey` qcodes |
| `remote/click.py` | Screenshot coordinates to VNC pointer events |
| `remote/shot.py` | Screendump trigger |
| `remote/mon.py` | One monitor command, **with its output** |
| `remote/upload-server.py` | Guest-to-host uploads (logs, dumps) |
| `lab.env` | Ports, paths, disks for one VM |

Copy `lab.env.example` to `lab.env` and fill it in. Nothing in it is secret.
The `lab.env` checked in beside this file is the live instance's, with the
lab-VM password in it; the username and password are worthless elsewhere.

## The one rule

**Look at the screen between steps.** Everything here is fire-and-forget: a
`sendkey` name QEMU does not know is dropped without a word, a keystroke that
lands in the wrong window becomes a menu accelerator, and a click on a dead
button exits 0. `vmctl shot` after each `type` and read the PNG.

## Install (once)

1. Install Windows from a stock ISO into a dedicated partition pair: a FAT32
   EFI system partition typed `c12a7328-f81f-11d2-ba4b-00a0c93ec93b`, and an
   NTFS partition for the OS. **Type the ESP correctly.** A plain basic-data
   partition makes `sysprep` fail with `0xC0000452` and the install cannot
   finish, with nothing else in the log to explain it.
2. Skip the hardware checks with `HKLM\SYSTEM\Setup\LabConfig` values
   `BypassTPMCheck`, `BypassSecureBootCheck`, `BypassRAMCheck`,
   `BypassCPUCheck`, `BypassStorageCheck` rather than patching the ISO.
3. In the VM, install OpenSSH Server and put your key in the **user's**
   `~\.ssh\authorized_keys`. Two traps, both silent:
   - an account created by unattended setup has *Password required: No* and
     rejects every key with `mm_answer_keyallowed: ... key is not allowed`.
     Fix: `net user <user> /passwordreq:yes`, then set the password again.
   - the `Match Group administrators` block points sshd at
     `C:\ProgramData\ssh\administrators_authorized_keys`, which refused a
     correct key with a correct ACL and no explanation. Comment the Match
     block out and use the user file.
4. `vmctl srv start`, then `vmctl start`.

## Gotchas worth their own paragraph

- **qcode names, not characters.** `[` is `bracket_left`, `|` is
  `shift-backslash`, `@` is `shift-2`, `"` is `shift-apostrophe`. There is no
  `shift-x` character in the table; uppercase is `shift-` plus the letter.
  Anything unlisted in `type.py` must be added there, because QEMU ignores it
  silently.
- **Clicks need scaling.** Address them in screenshot pixels; the framebuffer
  is often smaller and unscaled clicks land short.
- **`-qmp` goes before `-monitor`** in the qemu argv or it aborts with
  `Property 'qemu-fixed-text-console.device' not found`.
- **`cmd /c echo KEY>file`**, no space before `>`. With a space, Windows wrote
  the file as UTF-16LE and sshd refused to parse it.
- **PowerShell parses `[IO.File]` fine; `cmd` relocation is `cd /d`.** The
  monitor sends keystrokes into whatever is focused, including a `setup.exe`
  that reads no keyboard.
- **UAC on the secure desktop:** move focus with `left`, then `ret`; the
  default is *No*.
- **The guest firewall prompts.** Until a prompt is accepted, a bare port
  rule can still leave the port dead (`SYN_SENT` in `info usernet`). Turning
  the firewall off is fine for a lab: `netsh advfirewall set allprofiles
  state off`.
- **Raw partitions need the host to let go.** `umount /boot/efi`, and stop the
  VM before touching the disk from the host.
- **Slow console.** This is a TCG-ish experience for window launches; a
  Terminal can take 20 s to appear. Wait before concluding a click missed.
- **A black screenshot is usually a blanked display**, not a dead VM. Use
  `vmctl wake`; check `vmctl status` before believing the screen.
- **hostfwd entries can vanish.** If ssh connection-refuses while `vmctl
  status` says running, re-add the mapping: `vmctl mon "hostfwd_add
  tcp:127.0.0.1:2222-:22"`.
- **`vmctl get` runs the transfer over ssh when the box answers and on the
  console otherwise.** Nothing crosses the boundary but HTTP. Guest files come back over HTTP because
  nothing else crosses the boundary.

## Daily use

```sh
vmctl status
vmctl shot                 # eyes first
vmctl type 'C:\victory\victory.exe --diag'
vmctl shot
vmctl get 'C:\victory\crashes.log'
vmctl stop
```

If a step needs elevation, get an elevated shell once (`meta_l-x`, `a`, accept
UAC) and keep typing into it, rather than re-elevating per command.
