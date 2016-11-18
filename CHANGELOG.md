# v0.1.2 (2016-11-18)

- BREAKING CHANGE: Due to new proc_macro semantics, every `ErrorKind` enum now needs a special `Msg(String)` member.
- Added `backtrace = ...` item to top-level `#[error_chain]` attribute to allow overriding the backtrace type or disabling backtrace functionality completely.

# v0.1.1 (2016-11-12)

- Fixed default name for `ChainErr` trait to be `ChainErr`, and added `chain_err = "Foo"` item to top-level `#[error_chain]` attribute to override the name.

# v0.1.0 (2016-11-12)

First release.
