# Adbyss

[![ci](https://img.shields.io/github/actions/workflow/status/Blobfolio/adbyss/ci.yaml?style=flat-square&label=ci)](https://github.com/Blobfolio/adbyss/actions)
[![deps.rs](https://deps.rs/repo/github/blobfolio/adbyss/status.svg?style=flat-square&label=deps.rs)](https://deps.rs/repo/github/blobfolio/adbyss)<br>
[![license](https://img.shields.io/badge/license-wtfpl-ff1493?style=flat-square)](https://en.wikipedia.org/wiki/WTFPL)
[![contributions welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square&label=contributions)](https://github.com/Blobfolio/adbyss/issues)

Adbyss is a DNS blocklist manager for x86-64 Linux machines.

While ad-blocking browser extensions are extremely useful, they only block unwatned content *in the browser*, and require read/write access to every page you visit, which adds overhead and potential security/privacy issues.

Adbyss instead writes "blackhole" records directly to your system's `/etc/hosts` file, preventing all spammy connection attempts system-wide. As this is just a text file, no special runtime scripts are required, and there is very little overhead.



## Installation

Debian and Ubuntu users can just grab the pre-built `.deb` package from the [latest release](https://github.com/Blobfolio/adbyss/releases/latest).

This application is written in [Rust](https://www.rust-lang.org/) and can alternatively be built from source using [Cargo](https://github.com/rust-lang/cargo):

```bash
# See "cargo install --help" for more options.
cargo install \
    --git https://github.com/Blobfolio/adbyss.git \
    --bin adbyss

# Optional: download/save the advanced settings template so you can make
# changes if/as needed.
wget -qO- https://raw.githubusercontent.com/Blobfolio/adbyss/refs/heads/master/adbyss/skel/adbyss.yaml | \
    sudo tee /etc/adbyss.yaml >/dev/null
```



## Usage

It's easy.

Advanced settings are stored in `/etc/adbyss.yaml`. Just grab, edit, and save the [default config](https://raw.githubusercontent.com/Blobfolio/adbyss/refs/heads/master/adbyss/skel/adbyss.yaml) to that location if you want to change anything.

Otherwise, just run `sudo adbyss [FLAGS] [OPTIONS]`.

The following flags are available:

| Short | Long | Description |
| ----- | ---- | ----------- |
| | `--disable` | Remove all Adbyss entries from the hostfile. |
| `-h` | `--help` | Print help information and exit. |
| `-q` | `--quiet` | Do *not* summarize changes after write. |
| | `--show` | Print a sorted blackholable hosts list to STDOUT, one per line. |
| | `--stdout` | Print the would-be hostfile to STDOUT instead of writing it to disk. |
| `-V` | `--version` | Print program version and exit. |
| `-y` | `--yes` | Non-interactive mode; answer "yes" to all prompts. |

And the following option is available:

| Short | Long | Value | Description |
| ----- | ---- | ----- | ----------- |
| `-c` | `--config` | `<PATH>` | Use this configuration instead of /etc/adbyss.yaml. |

After running Adbyss for the first time, you might find some web sites are no longer working as expected. Most likely you're blocking an evil dependency the web site thinks it *needs*. No worries, just open your browser's Network Dev Tool window and reload the page. Make note of any failing domain(s), and update the `/etc/adbyss.yaml` configuration accordingly.

Restart your browser and/or computer and everything should be peachy again.

If ads persist in displaying even after running Adbyss and rebooting, double-check the browser isn't bypassing your computer's local DNS records. (Firefox's DNS-Over-HTTPS feature sometimes does this.) Tweak your settings as needed and you should be back in business.

It is important to remember that scammers and capitalists birth new schemes all the time. It is a good idea to rerun Adbyss weekly or so to ensure your hosts list contains the latest updates.



## Automation

The repository contains two `systemd` scripts — a [timer](https://github.com/Blobfolio/adbyss/tree/master/adbyss/skel/systemd/adbyss.timer) and a [service](https://github.com/Blobfolio/adbyss/tree/master/adbyss/skel/systemd/adbyss.service) — that can be used to automatically update your `/etc/hosts` file once daily using the global settings (stored in `/etc/adbyss.yaml`).

If you installed Adbyss using the pre-built `.deb` package, all you need to do is enable and start the timer, then you can forget all about it!

```bash
sudo systemctl enable adbyss.timer
sudo systemctl start adbyss.timer
```

If you built Adbyss manually, you'll need to manually copy both scripts to the appropriate `/etc/systemd` or `/usr/lib/systemd` subfolder and run:

```bash
sudo systemctl daemon-reload
sudo systemctl enable adbyss.timer
sudo systemctl start adbyss.timer
```



## Removal

If you're using the `systemd` service, be sure to stop and disable those scripts before doing anything else:

```bash
sudo systemctl stop adbyss.timer
sudo systemctl disable adbyss.timer
```

Then to remove all blocked entries, you can either open the hostfile in an editor and remove the `# ADBYSS #` marker and all subsequent lines, or run:

```bash
adbyss --disable
```

Save, reboot, and you should be back to normal!
