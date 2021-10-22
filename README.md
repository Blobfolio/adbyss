# Adbyss

[![Build Status](https://github.com/Blobfolio/adbyss/workflows/Build/badge.svg)](https://github.com/Blobfolio/adbyss/actions)
[![Dependency Status](https://deps.rs/repo/github/blobfolio/adbyss/status.svg)](https://deps.rs/repo/github/blobfolio/adbyss)

Adbyss is a DNS blocklist manager for x86-64 Linux machines.

While ad-blocking browser extensions are extremely useful, they only block unwatned content *in the browser*, and require read/write access to every page you visit, which adds overhead and potential security/privacy issues.

Adbyss instead writes "blackhole" records directly to your system's `/etc/hosts` file, preventing all spammy connection attempts system-wide. As this is just a text file, no special runtime scripts are required, and there is very little overhead.



## Installation

Debian and Ubuntu users can just grab the pre-built `.deb` package from the [latest release](https://github.com/Blobfolio/adbyss/releases/latest).

This application is written in [Rust](https://www.rust-lang.org/) and can alternatively be built from source using [Cargo](https://github.com/rust-lang/cargo):

```bash
# Clone the source.
git clone https://github.com/Blobfolio/adbyss.git

# Go to it.
cd adbyss

# Copy the configuration file.
sudo cp adbyss/misc/adbyss.yaml /etc

# Build as usual. Specify additional flags as desired.
cargo build \
    --bin adbyss \
    --release
```

(This should work under other 64-bit Unix environments too, like MacOS.)



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



## License

See also: [CREDITS.md](CREDITS.md)

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
