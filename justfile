##
# Development Recipes
#
# This justfile is intended to be run from inside a Docker sandbox:
# https://github.com/Blobfolio/righteous-sandbox
#
# docker run \
#	--rm \
#	-v "{{ invocation_directory() }}":/share \
#	-it \
#	--name "righteous_sandbox" \
#	"righteous/sandbox:debian"
#
# Alternatively, you can just run cargo commands the usual way and ignore these
# recipes.
##

pkg_id      := "adbyss"
pkg_name    := "Adbyss"
pkg_dir1    := justfile_directory() + "/adbyss"
pkg_dir2    := justfile_directory() + "/adbyss_psl"

cargo_dir   := "/tmp/" + pkg_id + "-cargo"
cargo_bin   := cargo_dir + "/release/" + pkg_id
doc_dir     := justfile_directory() + "/doc"
release_dir := justfile_directory() + "/release"
skel_dir    := pkg_dir2 + "/skel"

export RUSTFLAGS := "-C target-cpu=x86-64-v3"



# Bench PSL!
bench BENCH="":
	#!/usr/bin/env bash

	clear
	if [ -z "{{ BENCH }}" ]; then
		cargo bench \
			-p adbyss_psl \
			--benches \
			--target-dir "{{ cargo_dir }}"
	else
		cargo bench \
			-p adbyss_psl \
			--bench "{{ BENCH }}" \
			--target-dir "{{ cargo_dir }}"
	fi
	exit 0


# Build Release!
@build:
	# First let's build the Rust bit.
	env SHOW_TOTALS=1 cargo build \
		--bin "{{ pkg_id }}" \
		--release \
		--target-dir "{{ cargo_dir }}"


# Build Debian package!
@build-deb: clean credits fetch-vendor test build
	# cargo-deb doesn't support target_dir flags yet.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	mv "{{ cargo_dir }}" "{{ justfile_directory() }}/target"

	# Build the deb.
	cargo-deb \
		--no-build \
		-p {{ pkg_id }} \
		-o "{{ release_dir }}"

	just _fix-chown "{{ pkg_dir2 }}"
	just _fix-chown "{{ release_dir }}"
	mv "{{ justfile_directory() }}/target" "{{ cargo_dir }}"


@clean:
	# Most things go here.
	[ ! -d "{{ cargo_dir }}" ] || rm -rf "{{ cargo_dir }}"

	# But some Cargo apps place shit in subdirectories even if
	# they place *other* shit in the designated target dir. Haha.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	[ ! -d "{{ pkg_dir1 }}/target" ] || rm -rf "{{ pkg_dir1 }}/target"
	[ ! -d "{{ pkg_dir2 }}/target" ] || rm -rf "{{ pkg_dir2 }}/target"

	cargo update -w


# Clippy.
@clippy:
	clear
	cargo clippy \
		--workspace \
		--release \
		--all-features \
		--target-dir "{{ cargo_dir }}"


# Generate CREDITS, completions, man.
@credits:
	cargo bashman -m "{{ pkg_dir1 }}/Cargo.toml" -t x86_64-unknown-linux-gnu
	just _fix-chown "{{ justfile_directory() }}/CREDITS.md"
	just _fix-chown "{{ release_dir }}"


# Build Docs.
@doc:
	# Make the docs.
	cargo +nightly rustdoc \
		--manifest-path "{{ pkg_dir2 }}/Cargo.toml" \
		--release \
		--all-features \
		--target-dir "{{ cargo_dir }}" \
		-- \
		--cfg docsrs

	# Move the docs and clean up ownership.
	[ ! -d "{{ doc_dir }}" ] || rm -rf "{{ doc_dir }}"
	mv "{{ cargo_dir }}/doc" "{{ doc_dir }}"
	just _fix-chown "{{ doc_dir }}"


# Fetch Vendor Files.
@fetch-vendor:
	clear

	fyi info "Original vendor files."
	md5sum "{{ skel_dir }}/raw/public_suffix_list.dat"

	wget -nv \
		-O "{{ skel_dir }}/raw/public_suffix_list.dat" \
		"https://raw.githubusercontent.com/publicsuffix/list/master/public_suffix_list.dat"

	just _fix-chown "{{ skel_dir }}"
	just _fix-chmod "{{ skel_dir }}"

	fyi info "New vendor files."
	md5sum "{{ skel_dir }}/raw/public_suffix_list.dat"


# Test Run.
@run *ARGS:
	clear

	cargo run \
		--bin "{{ pkg_id }}" \
		--release \
		--target-dir "{{ cargo_dir }}" \
		-- {{ ARGS }}


# Unit tests!
@test:
	clear
	cargo test \
		--workspace \
		--all-features \
		--target-dir "{{ cargo_dir }}"

	cargo test \
		--workspace \
		--release \
		--all-features \
		--target-dir "{{ cargo_dir }}"



# Get/Set version.
version:
	#!/usr/bin/env bash

	# Current version.
	_ver1="$( toml get "{{ pkg_dir1 }}/Cargo.toml" package.version | \
		sed 's/"//g' )"

	# Find out if we want to bump it.
	_ver2="$( whiptail --inputbox "Set {{ pkg_name }} version:" --title "Release Version" 0 0 "$_ver1" 3>&1 1>&2 2>&3 )"

	exitstatus=$?
	if [ $exitstatus != 0 ] || [ "$_ver1" = "$_ver2" ]; then
		exit 0
	fi

	fyi success "Setting version to $_ver2."

	# Set the release version!
	just _version "{{ pkg_dir1 }}" "$_ver2"
	just _version "{{ pkg_dir2 }}" "$_ver2"


# Set version for real.
@_version DIR VER:
	[ -f "{{ DIR }}/Cargo.toml" ] || exit 1

	# Set the release version!
	toml set "{{ DIR }}/Cargo.toml" package.version "{{ VER }}" > /tmp/Cargo.toml
	just _fix-chown "/tmp/Cargo.toml"
	mv "/tmp/Cargo.toml" "{{ DIR }}/Cargo.toml"


# Init dependencies.
@_init:
	[ -f "/etc/adbyss.yaml" ] || cp "{{ pkg_dir1 }}/skel/adbyss.yaml" "/etc"


# Fix file/directory permissions.
@_fix-chmod PATH:
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type f -exec chmod 0644 {} +
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type d -exec chmod 0755 {} +


# Fix file/directory ownership.
@_fix-chown PATH:
	[ ! -e "{{ PATH }}" ] || chown -R --reference="{{ justfile() }}" "{{ PATH }}"
