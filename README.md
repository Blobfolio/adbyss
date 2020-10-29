# Adbyss

Adbyss is a DNS blocklist manager for x86-64 Linux machines.

While ad-blocking browser extensions are extremely useful, they only block
unwatned content *in the browser*, and require read/write access to every
page you visit, which adds overhead and potential security/privacy issues.

Adbyss instead writes "blackhole" records directly to your system's `/etc/hosts`
file, preventing all spammy connection attempts system-wide. As this is just a
text file, no special runtime scripts are required, and there is very little
overhead.



## Installation

This application is written in [Rust](https://www.rust-lang.org/) and can be installed using [Cargo](https://github.com/rust-lang/cargo).

For stable Rust (>= `1.47.0`), run:
```bash
RUSTFLAGS="-C link-arg=-s" cargo install \
    --git https://github.com/Blobfolio/adbyss.git \
    --bin adbyss \
    --target x86_64-unknown-linux-gnu
```

Pre-built `.deb` packages are also added for each [release](https://github.com/Blobfolio/adbyss/releases/latest). They should always work for the latest stable Debian and Ubuntu.



## Usage

It's easy. Just run `sudo adbyss [FLAGS] [OPTIONS]`.

The following flags are available:
```bash
-h, --help          Prints help information.
    --no-backup     Do *not* back up the hostfile when writing changes.
    --no-preserve   Do *not* preserve custom entries from hostfile when
                    writing changes.
    --no-summarize  Do *not* summarize changes after write.
    --stdout        Send compiled hostfile to STDOUT.
-V, --version       Prints version information.
-y, --yes           Non-interactive mode; answer "yes" to all prompts.
```

And the following options are available:
```bash
--filter <lists>    Specify which of [adaway, adbyss, stevenblack,
                    yoyo] to use, separating multiple lists with
                    commas. [default: all]
--hostfile <path>   Hostfile to use. [default: /etc/hosts]
--exclude <hosts>   Comma-separated list of hosts to *not* blacklist.
--regexclude <pats> Same as --exclude except it takes a comma-separated
                    list of regular expressions.
--include <hosts>   Comma-separated list of additional hosts to
                    blacklist.
```

Click [here](https://docs.rs/regex/1.4.1/regex/index.html#syntax) for regular expression syntax information.

After running Adbyss for the first time, you might find some web sites are no longer working as expected. Most likely you're blocking an evil dependency the web site thinks it *needs*. No worries, just open your browser's Network Dev Tool window and reload the page. Make note of any failing domain(s), and rerun Adbyss with `--exclude domain1,domain2,etc`.

Restart your browser and/or computer and everything should be peachy again.

If ads persist in displaying even after running Adbyss and rebooting, double-check the browser isn't bypassing your computer's local DNS records. (Firefox's DNS-Over-HTTPS feature sometimes does this.) Tweak your settings as needed and you should be back in business.

It is important to remember that scammers and capitalists birth new schemes all the time. It is a good idea to rerun Adbyss weekly or so to ensure your hosts list contains the latest updates.



## Removal

To remove all Adbyss rules from your hosts file, simply open the hosts file in a text editor, find the big-obvious `# ADBYSS #` marker, and delete it and everything following it. Save, reboot, and you're back to normal.



## Credits

| Library | License | Author |
| ---- | ---- | ---- |
| [AdAway](https://adaway.org/) | GPLv3+ | AdAway |
| [chrono](https://crates.io/crates/chrono) | Apache-2.0 OR MIT | Kang Seonghoon, Brandon W Maister |
| [lazy_static](https://crates.io/crates/lazy_static) | Apache-2.0 OR MIT | Marvin Löbel |
| [publicsuffix](https://crates.io/crates/publicsuffix) | Apache-2.0 OR MIT | rushmorem |
| [rayon](https://crates.io/crates/rayon) | Apache-2.0 OR MIT | Josh Stone, Niko Matsakis |
| [regex](https://crates.io/crates/regex) | Apache-2.0 OR MIT | The Rust Project Developers |
| [Steven Black](https://github.com/StevenBlack/hosts) | MIT | Steven Black |
| [tempfile-fast](https://crates.io/crates/tempfile-fast) | MIT | Chris West (Faux) |
| [ureq](https://crates.io/crates/ureq) | Apache-2.0 OR MIT | Martin Algesten |
| [Yoyo](https://pgl.yoyo.org/adservers/) || Peter Lowe |



## License

Copyright © 2020 [Blobfolio, LLC](https://blobfolio.com) &lt;hello@blobfolio.com&gt;

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
