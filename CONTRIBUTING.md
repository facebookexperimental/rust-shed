# Contributing to rust-shed
We want to make contributing to this project as easy and transparent as
possible.

## Our Development Process
External pull requests are first applied to facebook's internal branch, then
synced with rust-shed github repository.

## Pull Requests
We actively welcome your pull requests.

1. Fork the repo and create your branch from `master`.
2. If you've added code that should be tested, add tests.
3. If you've changed APIs, update the documentation.
4. Ensure the test suite passes.
5. Make sure your code lints.
6. If you haven't already, complete the Contributor License Agreement ("CLA").

## Contributor License Agreement ("CLA")
In order to accept your pull request, we need you to submit a CLA. You only need
to do this once to work on any of Facebook's open source projects.

Complete your CLA here: <https://code.facebook.com/cla>

## Issues
We use GitHub issues to track public bugs. Please ensure your description is
clear and has sufficient instructions to be able to reproduce the issue.

Facebook has a [bounty program](https://www.facebook.com/whitehat/) for the safe
disclosure of security bugs. In those cases, please go through the process
outlined on that page and do not file a public issue.

## Coding Style
Keep `use` statements sorted in the following order:

1. Imports from external crates.
2. Imports from within this crate.
3. `super` imports.
4. `self` imports.

Within each subgroup, `use` statements should be in alphabetical order.

Use [`rustfmt`](https://github.com/rust-lang-nursery/rustfmt/) to format your
code. This means:

* 4 spaces for indentation rather than tabs
* 80 character line length recommended, up to 100 characters if necessary.

This project uses the `rustfmt` currently based on nightly Rust
(`rustfmt-nightly` as of June 2017). For instructions on how to install it, see
the
[`rustfmt` README](https://github.com/rust-lang-nursery/rustfmt/#installation).

## License
By contributing to rust-shed, you agree that your contributions will be
licensed under both the [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE) files in the root directory of this source
tree.
