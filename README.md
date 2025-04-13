# zola_abridge_helper

Designed to make working with abridge easier by providing:

- Verifications that guidelines suggested by abridge are followed for description length
- Sets / verifies tags and series values on pages that are in a section
- Ensures sections have a explicit value set for `transparent`

Rules can also be found in the long help output of the executable `--help`.

# Install

```sh
cargo install zola_abridge_helper
```

# Usage

After installing run the following to see the available options

```sh
zola_abridge_helper --help
```

<!-- TODO find way to automate having the help output show up here. Needs to be automatic because doing it manually is not sustainable. -->

To see instructions on setting it up as a pre-push hook see [my notes](https://c-git.github.io/misc/documentation-update/#using-zola-chrono) for how I did it for my use case for `zola_chrono` which is very similar (this project was based on that one).

## License

All code in this repository is dual-licensed under either:

- Apache License, Version 2.0
- MIT license

at your option.
This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are very good reasons to include both as noted in
this [issue](https://github.com/bevyengine/bevy/issues/2373) on [Bevy](https://bevyengine.org)'s repo.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
