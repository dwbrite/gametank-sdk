## Development

Install the tools locally:

    cargo install --path .

This builds `gte`, `gtrom`, `gtgo`, `gtld`. `build.rs` regenerates
`src/bin/gtrom/rom-template.tar.gz` from `rom-template/` on every build, so
template changes are picked up automatically.

Test template changes in place:

    cd rom-template
    gtrom build      # or flash / run

`rom-template/` is excluded from the root workspace and builds against the
`mos` toolchain via its own `rust-toolchain.toml`. Real builds go through
podman using `docker.io/dwbrite/rust-mos:gte`.

## Releasing

    cargo release <level> --execute

Bumps the version, regenerates and commits the tarball, tags, publishes to
crates.io, and pushes. Omit `--execute` for a dry run.

For a release candidate, give the version explicitly the first time
(`cargo release 0.19.0-rc.1`), then `cargo release rc` to increment and
`cargo release release` to drop the suffix.

Note: jj leaves git in detached HEAD. Run `git checkout master` before
releasing and `jj git import` afterward.

## Distribution

Binary artifacts, installers, and target platforms are configured in
`dist-workspace.toml`. After editing it:

    dist init

which regenerates `.github/workflows/release.yml`. Commit both.

    dist plan     # what would be built
    dist build    # build for the host platform only

Pushing a `gametank-sdk-v*` tag triggers the release workflow.
