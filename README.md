# Adbyss

Adbyss is a DNS blocklist manager for x86-64 Linux machines.

While ad-blocking browser extensions are extremely useful, they only block unwatned content *in the browser*, and require read/write access to every page you visit, which adds overhead and potential security/privacy issues.

Adbyss instead writes "blackhole" records directly to your system's `/etc/hosts` file, preventing all spammy connection attempts system-wide. As this is just a text file, no special runtime scripts are required, and there is very little overhead.



## Installation

This application is written in [Rust](https://www.rust-lang.org/) and can be built using [Cargo](https://github.com/rust-lang/cargo). If building manually, don't forget to copy the configuration file:

```bash
sudo cp adbyss/misc/adbyss.yaml /etc
```

Pre-built `.deb` packages are also added for each [release](https://github.com/Blobfolio/adbyss/releases/latest). They should always work for the latest stable Debian and Ubuntu.



## Usage

It's easy.

Settings are stored in `/etc/adbyss.yaml`. Edit those as needed.

Otherwise, just run `sudo adbyss [FLAGS] [OPTIONS]`.

The following flags are available:

```bash
    --disable       Remove *all* Adbyss entries from the hostfile.
-h, --help          Prints help information.
-q, --quiet         Do *not* summarize changes after write.
    --show          Print a sorted blackholable hosts list to STDOUT, one per
                    line.
    --stdout        Print the would-be hostfile to STDOUT instead of writing
                    it to disk.
-V, --version       Prints version information.
-y, --yes           Non-interactive mode; answer "yes" to all prompts.
```

And the following option is available:

```bash
-c, --config <path> Use this configuration instead of /etc/adbyss.yaml.
```

After running Adbyss for the first time, you might find some web sites are no longer working as expected. Most likely you're blocking an evil dependency the web site thinks it *needs*. No worries, just open your browser's Network Dev Tool window and reload the page. Make note of any failing domain(s), and update the `/etc/adbyss.yaml` configuration accordingly.

Restart your browser and/or computer and everything should be peachy again.

If ads persist in displaying even after running Adbyss and rebooting, double-check the browser isn't bypassing your computer's local DNS records. (Firefox's DNS-Over-HTTPS feature sometimes does this.) Tweak your settings as needed and you should be back in business.

It is important to remember that scammers and capitalists birth new schemes all the time. It is a good idea to rerun Adbyss weekly or so to ensure your hosts list contains the latest updates.



## Removal

To remove all Adbyss rules from your hosts file, either run `adbyss --disable`, or open the hostfile in a text editor, find the big-obvious `# ADBYSS #` marker, and delete it and all subsequent lines.

Save, reboot, and you're back to normal.



## Credits

| Library | License | Author |
| ---- | ---- | ---- |
| [ahash](https://crates.io/crates/ahash) | Apache-2.0 OR MIT | Tom Kaitchuck |
| [chrono](https://crates.io/crates/chrono) | Apache-2.0 OR MIT | Kang Seonghoon, Brandon W Maister |
| [idna](https://crates.io/crates/idna) | Apache-2.0 OR MIT | The `rust-url` Developers |
| [lazy_static](https://crates.io/crates/lazy_static) | Apache-2.0 OR MIT | Marvin Löbel |
| [rayon](https://crates.io/crates/rayon) | Apache-2.0 OR MIT | Josh Stone, Niko Matsakis |
| [regex](https://crates.io/crates/regex) | Apache-2.0 OR MIT | The Rust Project Developers |
| [serde](https://crates.io/crates/serde) | Apache-2.0 OR MIT | David Tolnay, Erick Tryzelaar |
| [serde_yaml](https://crates.io/crates/serde_yaml) | Apache-2.0 OR MIT | David Tolnay |
| [tempfile-fast](https://crates.io/crates/tempfile-fast) | MIT | Chris West (Faux) |
| [ureq](https://crates.io/crates/ureq) | Apache-2.0 OR MIT | Martin Algesten |

| Data | License | Author |
| ---- | ---- | ---- |
| [AdAway](https://adaway.org/) | GPLv3+ | AdAway |
| [Public Suffix List](https://publicsuffix.org/list/) | MPL-2.0 | Mozilla Foundation |
| [Steven Black](https://github.com/StevenBlack/hosts) | MIT | Steven Black |
| [Yoyo](https://pgl.yoyo.org/adservers/) | MCRAE GENERAL PUBLIC LICENSE (v4.r53) | Peter Lowe |



## License

Copyright © 2021 [Blobfolio, LLC](https://blobfolio.com) &lt;hello@blobfolio.com&gt;

This work is free. You can redistribute it and/or modify it under the terms of the Do What The Fuck You Want To Public License, Version 2.

    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    Version 2, December 2004
    
    Copyright (C) 2004 Sam Hocevar <sam@hocevar.net>
    
    Everyone is permitted to copy and distribute verbatim or modified
    copies of this license document, and changing it is allowed as long
    as the name is changed.
    
    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION
    
    0. You just DO WHAT THE FUCK YOU WANT TO.
