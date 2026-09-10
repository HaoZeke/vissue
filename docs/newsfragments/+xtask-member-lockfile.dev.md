The version bump rewrites every workspace member in the lockfile,
including `xtask`, and refuses to finish if `cargo metadata --locked`
disagrees. 0.9.1 left `xtask` at 0.9.0 and CI failed `--locked`.
