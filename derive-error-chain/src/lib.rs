#![recursion_limit = "300"]

#![cfg_attr(feature = "cargo-clippy", deny(clippy, clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", allow(
	large_enum_variant,
	too_many_arguments,
	use_self,
))]

//! A Macros 1.1 implementation of <https://crates.io/crates/error-chain>
//!
//! The `error-chain` example
//!
//! ```
//! # #[macro_use] extern crate error_chain;
//! #
//! mod other_error {
//!     error_chain! {}
//! }
//!
//! error_chain! {
//!     types {
//!         Error, ErrorKind, ResultExt, Result;
//!     }
//!
//!     links {
//!         Another(other_error::Error, other_error::ErrorKind) #[cfg(unix)];
//!     }
//!
//!     foreign_links {
//!         Fmt(::std::fmt::Error);
//!         Io(::std::io::Error) #[cfg(unix)];
//!     }
//!
//!     errors {
//!         InvalidToolchainName(t: String) {
//!             description("invalid toolchain name")
//!             display("invalid toolchain name: '{}'", t)
//!         }
//!     }
//! }
//! ```
//!
//! becomes
//!
//! ```
//! # #[macro_use] extern crate derive_error_chain;
//! # #[macro_use] extern crate error_chain;
//! #
//! mod other_error {
//!     #[derive(Debug, ErrorChain)]
//!     pub enum ErrorKind {
//!         Msg(String),
//!     }
//! }
//!
//! #[derive(Debug, ErrorChain)]
//! pub enum ErrorKind {
//!     Msg(String),
//!
//!     #[cfg(unix)]
//!     #[error_chain(link = "other_error::Error")]
//!     Another(other_error::ErrorKind),
//!
//!     #[error_chain(foreign)]
//!     Fmt(::std::fmt::Error),
//!
//!     #[cfg(unix)]
//!     #[error_chain(foreign)]
//!     Io(::std::io::Error),
//!
//!     #[error_chain(custom)]
//!     #[error_chain(description = r#"|_| "invalid toolchain name""#)]
//!     #[error_chain(display = r#"|t| write!(f, "invalid toolchain name: '{}'", t)"#)]
//!     InvalidToolchainName(String),
//! }
//! ```
//!
//! So the obvious differences from `error_chain!` are:
//!
//! - The `ErrorKind` is an enum instead of a macro invocation.
//! - Error links are variants of the enum instead of lines inside the macro.
//! - Links have explicit annotations marking them as chainable / foreign / custom instead of being grouped into corresponding sections of the macro.
//! - Attributes like `#[cfg]` are applied to the variants directly instead of needing special syntax.
//! - `description` and `display` are defined as function expressions specified as attribute values, instead of shorthands integrated into the macro syntax.
//!
//! The less obvious differences are:
//!
//! - The `ErrorKind` must explicitly implement `::std::fmt::Debug`, either automatically using `#[derive]` or manually implemented separately. `error_chain!` does this implicitly.
//! - Unlike `error_chain!`, the `ErrorKind` need not have `pub` visibility. The generated `Error`, `Result` and `ResultExt` will have the same visibility as the `ErrorKind`.
//! - The `ErrorKind` can have a special `Msg(String)` member for converting strings to the `ErrorKind`. `error_chain!` does this implicitly.
//! - Unlike `error-chain`, the `Msg(String)` member is optional. If absent, the `ErrorKind` and `Error` will not impl `From<String>` and `From<&str>`.
//! - Doc comments, since they're effectively attributes, can be applied on the enum variants without any special syntax like `error_chain!` has.
//! - The `ErrorKind` can be generic.
//!
//! # Enum attributes
//!
//! - `#[error_chain(error = "ErrorName")]`
//!
//!     Override the name of the generated `Error` struct to the given name. If not provided, the struct will be named `Error`.
//!
//! - `#[error_chain(result_ext = "ResultExtName")]`
//!
//!     Override the name of the generated `ResultExt` trait to the given name. If not provided, the trait will be named `ResultExt`.
//!
//! - `#[error_chain(result = "ResultName")]`
//!
//!     Override the name of the generated `Result` type alias to the given name. If not provided, the alias will be named `Result`.
//!     If set to the empty string `""`, the alias will not be generated at all.
//!
//! - `#[error_chain(backtrace = "false")]` or `#[error_chain(backtrace = false)]`
//!
//!     Disable backtrace functionality in the generated code. This should be kept in sync with the value of the `backtrace` feature of the `error-chain` crate.
//!     In other words, if you set `backtrace = "false"` here, you must also specify `default-features = false` for `error-chain` in your `Cargo.toml`
//!
//! # Variant definitions
//!
//! - Chainable links
//!
//!     ```
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # mod other_error {
//!     #     #[derive(Debug, ErrorChain)]
//!     #     pub enum ErrorKind {
//!     #         Msg(String),
//!     #     }
//!     # }
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!     #[error_chain(link = "other_error::Error")]
//!     Another(other_error::ErrorKind),
//!     # }
//!     ```
//!
//!     A chainable link is an error and errorkind that have been generated using `error-chain` or `derive-error-chain`. The variant must have a single field
//!     to hold the chained errorkind, and the `link` attribute must specify a path to the chained error.
//!
//!     When the `proc_macro` feature is enabled, the value of the `link` attribute does not need to be stringified:
//!
//!     ```
//!     # #![feature(proc_macro)]
//!     #
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # mod other_error {
//!     #     #[derive(Debug, ErrorChain)]
//!     #     pub enum ErrorKind {
//!     #         Msg(String),
//!     #     }
//!     # }
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!     #[error_chain(link = other_error::Error)]
//!     Another(other_error::ErrorKind),
//!     # }
//!     ```
//!
//! - Foreign links
//!
//!     ```
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!     #[error_chain(foreign)]
//!     Fmt(::std::fmt::Error),
//!     # }
//!     ```
//!
//!     A foreign link is an error that implements `::std::error::Error` but otherwise does not follow `error-chain`'s conventions. The variant must have
//!     a single field to hold the foreign error.
//!
//! - Custom links
//!
//!     ```
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!     #[error_chain(custom)]
//!     InvalidToolchainName(String),
//!     # }
//!     ```
//!
//!     A custom link is an arbitrary variant that can hold any members.
//!
//! # Variant attributes
//!
//! In addition to the above attributes that identify the type of the variant's link, the below attributes can be used on all links.
//!
//! - `#[error_chain(description = "some_function_expression")]`
//!
//!     Specifies a function expression to be used to implement `ErrorKind::description()`.
//!     This value is also returned from the implementation of `::std::error::Error::description()` on the generated `Error`.
//!
//!     This can be an inline lambda:
//!
//!     ```
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(description = r#"|_| "invalid toolchain name""#)]
//!     InvalidToolchainName(String),
//!     # }
//!     ```
//!
//!     or it can be a separate function:
//!
//!     ```
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(description = "invalid_toolchain_name_error_description")]
//!     InvalidToolchainName(String),
//!     # }
//!
//!     // <snip>
//!
//!     fn invalid_toolchain_name_error_description(_: &str) -> &str {
//!         "invalid toolchain name"
//!     }
//!     ```
//!
//!     The function expression must have the signature `(...) -> &'static str`. It should have one parameter for each field of the variant.
//!     The fields are passed in by reference.
//!
//!     Thus in the above example, since `InvalidToolchainName` had a single field of type `String`, the function expression needed to be of type
//!     `(&str) -> &'static str`
//!
//!     If not specified, the default implementation behaves in this way:
//!
//!     - Chainable links: Forwards to the chained error kind's `description()`
//!     - Foreign links: Forwards to the foreign error's implementation of `::std::error::Error::description()`
//!     - Custom links: Returns the stringified name of the variant.
//!
//!     When the `proc_macro` feature is enabled, the value does not need to be stringified:
//!
//!     ```
//!     # #![feature(proc_macro)]
//!     #
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(description = |_| "invalid toolchain name")]
//!     InvalidToolchainName(String),
//!     # }
//!     ```
//!
//!     ```
//!     # #![feature(proc_macro)]
//!     #
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(description = invalid_toolchain_name_error_description)]
//!     InvalidToolchainName(String),
//!     # }
//!     #
//!     # fn invalid_toolchain_name_error_description(_: &str) -> &str {
//!     #     "invalid toolchain name"
//!     # }
//!     ```
//!
//!     When the `proc_macro` feature is enabled, closure expressions that only call `write!` on the `::std::fmt::Formatter` can instead use a shorthand:
//!
//!     ```
//!     # #![feature(proc_macro)]
//!     #
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(description = const("invalid toolchain name"))]
//!     InvalidToolchainName(String),
//!     # }
//!     ```
//!
//! - `#[error_chain(display = "some_function_expression")]`
//!
//!     Specifies a function expression to be used to implement `::std::fmt::Display::fmt()` on the `ErrorKind` and generated `Error`
//!
//!     This can be an inline lambda:
//!
//!     ```
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(display = r#"|t| write!(f, "invalid toolchain name: '{}'", t)"#)]
//!     InvalidToolchainName(String),
//!     # }
//!     ```
//!
//!     or it can be a separate function:
//!
//!     ```
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(display = "invalid_toolchain_name_error_display")]
//!     InvalidToolchainName(String),
//!     # }
//!
//!     // <snip>
//!
//!     fn invalid_toolchain_name_error_display(f: &mut ::std::fmt::Formatter, t: &str) -> ::std::fmt::Result {
//!         write!(f, "invalid toolchain name: '{}'", t)
//!     }
//!     ```
//!
//!     The function expression must have the signature `(&mut ::std::fmt::Formatter, ...) -> ::std::fmt::Result`.
//!     It should have one `&mut ::std::fmt::Formatter` parameter, and one parameter for each field of the variant. The fields are passed in by reference.
//!     For brevity, closure expressions do not need the `&mut ::std::fmt::Formatter` parameter and instead capture `f` from the closure environment.
//!
//!     Thus in the above example, since `InvalidToolchainName` had a single field of type `String`, the function expression needed to be of type
//!     `(&mut ::std::fmt::Formatter, &str) -> ::std::fmt::Result`
//!
//!     If not specified, the default implementation of `::std::fmt::Display::fmt()` behaves in this way:
//!
//!     - Chainable links: Forwards to the chained errorkind's implementation of `::std::fmt::Display::fmt()`
//!     - Foreign links: Forwards to the foreign error's implementation of `::std::fmt::Display::fmt()`
//!     - Custom links: Writes the description of the variant to the formatter.
//!
//!     When the `proc_macro` feature is enabled, the value does not need to be stringified:
//!
//!     ```
//!     # #![feature(proc_macro)]
//!     #
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(display = |t| write!(f, "invalid toolchain name: '{}'", t))]
//!     InvalidToolchainName(String),
//!     # }
//!     ```
//!
//!     ```
//!     # #![feature(proc_macro)]
//!     #
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(display = invalid_toolchain_name_error_display)]
//!     InvalidToolchainName(String),
//!     # }
//!     #
//!     # fn invalid_toolchain_name_error_display(f: &mut ::std::fmt::Formatter, t: &str) -> ::std::fmt::Result {
//!     #     write!(f, "invalid toolchain name: '{}'", t)
//!     # }
//!     ```
//!
//!     When the `proc_macro` feature is enabled, closure expressions that only call `write!` on the `::std::fmt::Formatter` can instead use a shorthand:
//!
//!     ```
//!     # #![feature(proc_macro)]
//!     #
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!     // Tuple variants use `{0}`, `{1}`, and so on
//!         # #[error_chain(custom)]
//!     #[error_chain(display = const("invalid toolchain name: '{0}'"))]
//!     InvalidToolchainName(String),
//!     # }
//!     ```
//!
//!     ```
//!     # #![feature(proc_macro)]
//!     #
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!     // Struct variants use `{name_of_the_field}`
//!         # #[error_chain(custom)]
//!     #[error_chain(display = const("invalid toolchain name: '{name}'"))]
//!     InvalidToolchainName { name: String },
//!     # }
//!     ```
//!
//! - `#[error_chain(cause = "some_function_expression")]`
//!
//!     Specifies a function expression to be used to implement `::std::fmt::Error::cause()` on the generated `Error`
//!
//!     This can be an inline lambda:
//!
//!     ```
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(cause = "|_, err| err")]
//!     Io(::std::path::PathBuf, ::std::io::Error),
//!     # }
//!     ```
//!
//!     or it can be a separate function:
//!
//!     ```
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(cause = "parse_file_error_cause")]
//!     Io(::std::path::PathBuf, ::std::io::Error),
//!     # }
//!
//!     // <snip>
//!
//!     fn parse_file_error_cause<'a>(_: &::std::path::Path, err: &'a ::std::io::Error) -> &'a ::std::error::Error {
//!         err
//!     }
//!     ```
//!
//!     The function expression must have the signature `(...) -> &::std::error::Error`. It should have one parameter for each field of the variant.
//!     The fields are passed in by reference. The result is wrapped in `Option::Some()` for returning from `::std::error::Error::cause()`
//!
//!     Thus in the above example, since `Io` had two fields of type `::std::path::PathBuf` and `::std::io::Error`, the function expression needed to be of type
//!     `(&::std::path::Path, &::std::io::Error) -> &::std::error::Error`
//!
//!     If not specified, the default implementation of `::std::error::Error::cause()` behaves in this way:
//!
//!     - Chainable links: Returns `None`
//!     - Foreign links: Forwards to the foreign error's implementation of `::std::error::Error::cause()`
//!     - Custom links: Returns `None`
//!
//!     When the `proc_macro` feature is enabled, the value does not need to be stringified:
//!
//!     ```
//!     # #![feature(proc_macro)]
//!     #
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(cause = |_, err| err)]
//!     Io(::std::path::PathBuf, ::std::io::Error),
//!     # }
//!     ```
//!
//!     ```
//!     # #![feature(proc_macro)]
//!     #
//!     # #[macro_use] extern crate derive_error_chain;
//!     #
//!     # #[derive(Debug, ErrorChain)]
//!     # pub enum ErrorKind {
//!         # #[error_chain(custom)]
//!     #[error_chain(cause = parse_file_error_cause)]
//!     Io(::std::path::PathBuf, ::std::io::Error),
//!     # }
//!     #
//!     # fn parse_file_error_cause<'a>(_: &::std::path::Path, err: &'a ::std::io::Error) -> &'a ::std::error::Error {
//!     #     err
//!     # }
//!     ```
//!
//! # Conflicts with `error-chain` macros when the `proc_macro` feature is enabled
//!
//! If you have the `proc_macro` feature enabled and have code like this:
//!
//! ```compile_fail
//! #![feature(proc_macro)]
//!
//! #[macro_use] extern crate derive_error_chain;
//! #[macro_use] extern crate error_chain; // Want to use `bail!` and `quick_main!`
//!
//! #[derive(Debug, ErrorChain)]
//! #[error_chain(result = "MyResult")]
//! enum ErrorKind {
//!     Msg(String),
//! }
//!
//! quick_main!(|| -> MyResult<()> {
//!     bail!("failed");
//! });
//! ```
//!
//! it'll fail to compile with:
//!
//! ```text,ignore
//! error: macro `error_chain` may not be used in attributes
//! ```
//!
//! This is because the compiler thinks `#[error_chain(result = "MyResult")]` is the invocation of an attribute macro, notices that `error_chain!` is
//! a `macro_rules` macro brought into scope from the `error-chain` crate, and thus complains that a `macro_rules` macro cannot be used as
//! an attribute macro. It does this even though there is no attribute macro named `error_chain` and that the custom derive from this crate
//! has registered `error_chain` as an attribute it supports.
//!
//! See <https://github.com/rust-lang/rust/issues/38356#issuecomment-324277403> for the discussion.
//!
//! To work around this, don't use `#[macro_use]` with the `error-chain` crate. Instead, either `use` the macros you need from it:
//!
//! ```
//! #![feature(proc_macro)]
//!
//! #[macro_use] extern crate derive_error_chain;
//! extern crate error_chain;
//!
//! use error_chain::{ bail, quick_main };
//!
//! #[derive(Debug, ErrorChain)]
//! #[error_chain(result = "MyResult")]
//! enum ErrorKind {
//!     Msg(String),
//! }
//!
//! quick_main!(|| -> MyResult<()> {
//!     bail!("failed");
//! });
//! ```
//!
//! or fully qualify their paths:
//!
//! ```
//! #![feature(proc_macro)]
//!
//! #[macro_use] extern crate derive_error_chain;
//! extern crate error_chain;
//!
//! #[derive(Debug, ErrorChain)]
//! #[error_chain(result = "MyResult")]
//! enum ErrorKind {
//!     Msg(String),
//! }
//!
//! error_chain::quick_main!(|| -> MyResult<()> {
//!     error_chain::bail!("failed");
//! });
//! ```
//!
//! `use`ing the `error_chain!` macro itself is more complicated: it must be renamed so that it doesn't just cause the above error again,
//! and other macros it uses must also be imported, even though they're an implementation detail:
//!
//! ```
//! #![feature(proc_macro)]
//!
//! #[macro_use] extern crate derive_error_chain;
//! extern crate error_chain;
//!
//! use error_chain::{ error_chain as error_chain_macro, error_chain_processing, impl_error_chain_kind, impl_error_chain_processed, impl_extract_backtrace };
//!
//! #[derive(Debug, ErrorChain)]
//! #[error_chain(error = "MyError", result = "MyResult", result_ext = "MyResultExt")]
//! enum MyErrorKind {
//!     Msg(String),
//! }
//!
//! error_chain_macro! {
//! }
//! ```
//!
//! To use it fully-qualified, the macros it depends on must still be `use`d to bring them into scope:
//!
//! ```
//! #![feature(proc_macro)]
//!
//! #[macro_use] extern crate derive_error_chain;
//! extern crate error_chain;
//!
//! use error_chain::{ error_chain_processing, impl_error_chain_kind, impl_error_chain_processed, impl_extract_backtrace };
//!
//! #[derive(Debug, ErrorChain)]
//! #[error_chain(error = "MyError", result = "MyResult", result_ext = "MyResultExt")]
//! enum MyErrorKind {
//!     Msg(String),
//! }
//!
//! error_chain::error_chain! {
//! }
//! ```
//!
//! It's possible this experience will be made better before the `proc_macro` feature stabilizes.

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;
extern crate syntex_fmt_macros;

#[proc_macro_derive(ErrorChain, attributes(error_chain))]
pub fn derive_error_chain(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let ast: syn::DeriveInput = syn::parse(input).unwrap();

	let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

	let mut generics_lifetime = ast.generics.clone();
	generics_lifetime.params = std::iter::once(parse_quote!('__a)).chain(generics_lifetime.params).collect();
	let (impl_generics_lifetime, _, _) = generics_lifetime.split_for_impl();

	let mut result_generics = ast.generics.clone();
	result_generics.params.push(parse_quote!(__T));
	let (_, result_ty_generics, _) = result_generics.split_for_impl();

	let mut result_ext_generics_t = ast.generics.clone();
	result_ext_generics_t.params.push(parse_quote!(__T));
	let (result_ext_impl_generics_t, result_ext_ty_generics_t, _) = result_ext_generics_t.split_for_impl();

	let mut result_ext_generics_t_e = result_ext_generics_t.clone();
	result_ext_generics_t_e.params.push(parse_quote!(__E: ::std::error::Error + ::std::marker::Send + 'static));
	let (result_ext_impl_generics_t_e, _, _) = result_ext_generics_t_e.split_for_impl();

	let generics: std::collections::HashSet<_> =
		ast.generics.params.iter()
		.filter_map(|param|
			if let syn::GenericParam::Type(syn::TypeParam { ident, .. }) = *param {
				Some(ident)
			}
			else {
				None
			})
		.collect();

	let TopLevelProperties {
		error_kind_name,
		error_kind_vis,
		error_name,
		result_ext_name,
		result_name,
		support_backtrace,
		error_chain_name,
	} = (&ast).into();

	let result = match ast.data {
		syn::Data::Enum(syn::DataEnum { variants, .. }) => {
			let links: Vec<Link> = variants.into_iter().map(Into::into).collect();

			let error_kind_description_cases = links.iter().map(|link| link.error_kind_description(&error_kind_name));

			let error_kind_display_cases = links.iter().map(|link| link.error_kind_display_case(&error_kind_name));

			let error_kind_from_impls =
				links.iter().filter_map(|link|
					link.error_kind_from_impl(
						&error_kind_name,
						&impl_generics, &impl_generics_lifetime, &ty_generics, where_clause,
					));

			let error_cause_cases = links.iter().filter_map(|link| link.error_cause_case(&error_kind_name));

			let error_doc_comment = format!(r"The Error type.

This struct is made of three things:

- `{0}` which is used to determine the type of the error.
- a backtrace, generated when the error is created.
- an error chain, used for the implementation of `Error::cause()`.", error_kind_name);

			let error_from_impls =
				links.iter().filter_map(|link|
					link.error_from_impl(
						&error_kind_name, &error_name,
						&generics,
						&impl_generics, &impl_generics_lifetime, &ty_generics, where_clause,
					));

			let extract_backtrace_fn = if support_backtrace {
				let chained_error_extract_backtrace_cases = links.iter().filter_map(Link::chained_error_extract_backtrace_case);

				Some(quote! {
					fn extract_backtrace(err: &(::std::error::Error + Send + 'static)) -> Option<::std::sync::Arc<#error_chain_name::Backtrace>> {
						if let Some(err) = err.downcast_ref::<Self>() {
							return err.1.backtrace.clone();
						}

						#(#chained_error_extract_backtrace_cases)*

						None
					}
				})
			}
			else {
				None
			};

			let result_ext_chain_err_doc_comment = format!("\
				If the `Result` is an `Err` then `chain_err` evaluates the closure, \
				which returns *some type that can be converted to `{}`*, \
				boxes the original error to store as the cause, then returns a new error \
				containing the original error.\
			", error_kind_name);

			let result_wrapper = result_name.map(|result_name| quote! {
				/// Convenient wrapper around `::std::result::Result`
				#error_kind_vis type #result_name #result_ty_generics = ::std::result::Result<__T, #error_name #ty_generics>;
			});

			quote! {
				extern crate error_chain as #error_chain_name;

				impl #impl_generics #error_kind_name #ty_generics #where_clause {
					/// A string describing the error kind.
					pub fn description(&self) -> &str {
						#[cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]
						match *self {
							#(#error_kind_description_cases)*
						}
					}
				}

				impl #impl_generics ::std::fmt::Display for #error_kind_name #ty_generics #where_clause {
					fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
						#[cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]
						match *self {
							#(#error_kind_display_cases)*
						}
					}
				}

				#(#error_kind_from_impls)*

				impl #impl_generics From<#error_name #ty_generics> for #error_kind_name #ty_generics #where_clause {
					fn from(err: #error_name #ty_generics) -> Self { err.0 }
				}

				#[doc = #error_doc_comment]
				#[derive(Debug)]
				#error_kind_vis struct #error_name #impl_generics (
					/// The kind of the error.
					pub #error_kind_name #ty_generics,

					/// Contains the error chain and the backtrace.
					pub #error_chain_name::State,
				) #where_clause ;

				#[allow(unused)]
				impl #impl_generics #error_name #ty_generics #where_clause {
					/// Constructs an error from a kind, and generates a backtrace.
					pub fn from_kind(kind: #error_kind_name #ty_generics) -> Self {
						#error_name(kind, #error_chain_name::State::default())
					}

					/// Constructs a chained error from another error and a kind, and generates a backtrace.
					pub fn with_chain<__E, __K>(error: __E, kind: __K) -> Self
						where __E: ::std::error::Error + Send + 'static, __K: Into<#error_kind_name #ty_generics>
					{
						#error_name::with_boxed_chain(Box::new(error), kind)
					}

					/// Constructs a chained error from another boxed error and a kind, and generates a backtrace
					pub fn with_boxed_chain<__K>(error: Box<::std::error::Error + Send>, kind: __K) -> #error_name #ty_generics
						where __K: Into<#error_kind_name #ty_generics>
					{
						#error_name(kind.into(), #error_chain_name::State::new::<Self>(error))
					}

					/// Returns the kind of the error.
					pub fn kind(&self) -> &#error_kind_name #ty_generics { &self.0 }

					/// Iterates over the error chain.
					pub fn iter(&self) -> #error_chain_name::Iter {
						#error_chain_name::ChainedError::iter(self)
					}

					/// Returns the backtrace associated with this error.
					pub fn backtrace(&self) -> Option<&#error_chain_name::Backtrace> {
						self.1.backtrace()
					}

					/// Extends the error chain with a new entry.
					pub fn chain_err<__F, __EK>(self, error: __F) -> Self where __F: FnOnce() -> __EK, __EK: Into<#error_kind_name #ty_generics> {
						#error_name::with_chain(self, Self::from_kind(error().into()))
					}
				}

				impl #impl_generics ::std::error::Error for #error_name #ty_generics #where_clause {
					fn description(&self) -> &str { self.0.description() }

					fn cause(&self) -> Option<&::std::error::Error> {
						#[cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]
						match self.1.next_error {
							Some(ref c) => Some(&**c),
							None => match self.0 {
								#(#error_cause_cases)*

								_ => None,
							},
						}
					}
				}

				impl #impl_generics ::std::fmt::Display for #error_name #ty_generics #where_clause {
					fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
						::std::fmt::Display::fmt(&self.0, f)
					}
				}

				#(#error_from_impls)*

				impl #impl_generics From<#error_kind_name #ty_generics> for #error_name #ty_generics #where_clause {
					fn from(kind: #error_kind_name #ty_generics) -> Self { Self::from_kind(kind) }
				}

				impl #impl_generics ::std::ops::Deref for #error_name #ty_generics #where_clause {
					type Target = #error_kind_name #ty_generics;

					fn deref(&self) -> &Self::Target { &self.0 }
				}

				impl #impl_generics #error_chain_name::ChainedError for #error_name #ty_generics #where_clause {
					type ErrorKind = #error_kind_name #ty_generics;

					fn new(kind: Self::ErrorKind, state: #error_chain_name::State) -> Self {
						#error_name(kind, state)
					}

					fn from_kind(kind: Self::ErrorKind) -> Self {
						Self::from_kind(kind)
					}

					fn with_chain<__E, __K>(error: __E, kind: __K) -> Self
						where __E: ::std::error::Error + Send + 'static, __K: Into<Self::ErrorKind> {

						Self::with_chain(error, kind)
					}

					fn kind(&self) -> &Self::ErrorKind {
						self.kind()
					}

					fn iter(&self) -> #error_chain_name::Iter {
						#error_chain_name::Iter::new(Some(self))
					}

					fn backtrace(&self) -> Option<&#error_chain_name::Backtrace> {
						self.backtrace()
					}

					fn chain_err<__F, __EK>(self, error: __F) -> Self where __F: FnOnce() -> __EK, __EK: Into<Self::ErrorKind> {
						self.chain_err(error)
					}

					#extract_backtrace_fn
				}

				/// Additional methods for `Result` and `Option`, for easy interaction with this crate.
				#error_kind_vis trait #result_ext_name #result_ext_impl_generics_t #where_clause {
					#[doc = #result_ext_chain_err_doc_comment]
					fn chain_err<__F, __EK>(self, callback: __F) -> ::std::result::Result<__T, #error_name #ty_generics>
						where __F: FnOnce() -> __EK, __EK: Into<#error_kind_name #ty_generics>;
				}

				impl #result_ext_impl_generics_t_e #result_ext_name #result_ext_ty_generics_t for ::std::result::Result<__T, __E> #where_clause {
					fn chain_err<__F, __EK>(self, callback: __F) -> ::std::result::Result<__T, #error_name #ty_generics>
						where __F: FnOnce() -> __EK, __EK: Into<#error_kind_name #ty_generics> {
						self.map_err(move |e| {
							let state = #error_chain_name::State::new::<#error_name #ty_generics>(Box::new(e));
							#error_chain_name::ChainedError::new(callback().into(), state)
						})
					}
				}

				impl #result_ext_impl_generics_t #result_ext_name #result_ext_ty_generics_t for ::std::option::Option<__T> #where_clause {
					fn chain_err<__F, __EK>(self, callback: __F) -> ::std::result::Result<__T, #error_name #ty_generics>
						where __F: FnOnce() -> __EK, __EK: Into<#error_kind_name #ty_generics> {
						self.ok_or_else(move || {
							#error_chain_name::ChainedError::from_kind(callback().into())
						})
					}
				}

				#result_wrapper
			}
		},

		_ => panic!("#[derive(ErrorChain] can only be used with enums."),
	};

	result.into()
}

struct TopLevelProperties {
	error_kind_name: syn::Ident,
	error_kind_vis: syn::Visibility,
	error_name: syn::Ident,
	result_ext_name: syn::Ident,
	result_name: Option<syn::Ident>,
	error_chain_name: syn::Ident,
	support_backtrace: bool,
}

impl<'a> From<&'a syn::DeriveInput> for TopLevelProperties {
	fn from(ast: &'a syn::DeriveInput) -> Self {
		let mut error_name: syn::Ident = "Error".into();
		let mut result_ext_name: syn::Ident = "ResultExt".into();
		let mut result_name: Option<syn::Ident> = Some("Result".into());
		let mut support_backtrace = true;

		for attr in &ast.attrs {
			if !is_error_chain_attribute(attr) {
				continue;
			}

			match attr.interpret_meta() {
				Some(syn::Meta::List(syn::MetaList { nested, .. })) => {
					for nested_meta in nested {
						match nested_meta {
							syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue { ident, lit: syn::Lit::Str(value), .. })) => {
								let value = &value.value();

								match ident.as_ref() {
									"error" => error_name = syn::parse_str(value).unwrap_or_else(|err|
										panic!("Could not parse `error` value as an identifier - {}", err)),

									"result_ext" => result_ext_name = syn::parse_str(value).unwrap_or_else(|err|
										panic!("Could not parse `result_ext` value as an identifier - {}", err)),

									"result" => result_name =
										if value == "" {
											None
										}
										else {
											Some(syn::parse_str(value).unwrap_or_else(|err|
												panic!("Could not parse `result` value as an identifier - {}", err)))
										},

									"backtrace" => support_backtrace = value.parse().unwrap_or_else(|err|
										panic!("Could not parse `backtrace` value - {}", err)),

									_ =>
										panic!("Could not parse `error_chain` attribute - expected one of `error`, `result_ext`, `result`, `backtrace` but got {}", ident),
								}
							},

							syn::NestedMeta::Meta(syn::Meta::NameValue(
								syn::MetaNameValue { ref ident, lit: syn::Lit::Bool(syn::LitBool { value, .. }), .. }))
								if ident == "backtrace" => support_backtrace = value,

							_ => panic!("Could not parse `error_chain` attribute - expected one of `error`, `result_ext`, `result`, `backtrace`"),
						}
					}
				},

				_ => panic!("Could not parse `error_chain` attribute - expected one of `error`, `result_ext`, `result`, `backtrace`"),
			}
		}

		let error_chain_name = syn::parse_str(&format!("{}_error_chain", error_name)).unwrap_or_else(|err|
			panic!("Could not generate error_chain crate name as a valid ident - {}", err));

		TopLevelProperties {
			error_kind_name: ast.ident,
			error_kind_vis: ast.vis.clone(),
			error_name,
			result_ext_name,
			result_name,
			error_chain_name,
			support_backtrace,
		}
	}
}

struct Link {
	variant_ident: syn::Ident,
	variant_fields: syn::Fields,
	link_type: LinkType,
	custom_description: Option<CustomFormatter>,
	custom_display: Option<CustomFormatter>,
	custom_cause: Option<syn::Expr>,
}

enum LinkType {
	Msg,
	Chainable(syn::Type, syn::Type),
	Foreign(syn::Type),
	Custom,
}

impl From<syn::Variant> for Link {
	fn from(syn::Variant { ident: variant_ident, attrs, fields: variant_fields, .. }: syn::Variant) -> Self {
		let is_msg = loop {
			if variant_ident != "Msg" {
				break false;
			}

			if let syn::Fields::Unnamed(syn::FieldsUnnamed { ref unnamed, .. }) = variant_fields {
				if unnamed.len() == 1 {
					if let syn::Type::Path(syn::TypePath { ref path, .. }) = unnamed[0].ty {
						if !path.global() && path.segments.len() == 1 && path.segments[0].ident == "String" {
							break true;
						}
					}
				}
			}

			panic!("Expected Msg member to be a tuple of String");
		};

		if is_msg {
			return Link {
				variant_ident,
				variant_fields,
				link_type: LinkType::Msg,
				custom_description: None,
				custom_display: None,
				custom_cause: None,
			};
		}

		let mut link_type = None;
		let mut custom_description = None;
		let mut custom_display = None;
		let mut custom_cause: Option<syn::Expr> = None;

		for attr in attrs {
			if !is_error_chain_attribute(&attr) {
				continue;
			}

			if let Some(syn::Meta::List(syn::MetaList { nested, .. })) = attr.interpret_meta() {
				for nested_meta in nested {
					match nested_meta {
						syn::NestedMeta::Meta(syn::Meta::Word(ident)) => match ident.as_ref() {
							"foreign" => match variant_fields {
								syn::Fields::Unnamed(syn::FieldsUnnamed { ref unnamed, .. }) if unnamed.len() == 1 =>
									link_type = Some(LinkType::Foreign(unnamed[0].ty.clone())),

								_ => panic!("Foreign link {} must be a tuple of one element (the foreign error type).", variant_ident),
							},

							"custom" => link_type = Some(LinkType::Custom),

							_ => panic!(
								"Could not parse `error_chain` attribute of member {} - expected one of `foreign`, `custom` but got {}",
								variant_ident, ident),
						},

						syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue { ident, lit: syn::Lit::Str(value), .. })) => {
							let value = &value.value();

							match ident.as_ref() {
								"link" => match variant_fields {
									syn::Fields::Unnamed(syn::FieldsUnnamed { ref unnamed, .. }) if unnamed.len() == 1 =>
										link_type = Some(LinkType::Chainable(
											syn::parse_str(value).unwrap_or_else(|err|
												panic!("Could not parse `link` attribute of member {} as a type - {}", variant_ident, err)),
											unnamed[0].ty.clone())),

									_ => panic!("Chainable link {} must be a tuple of one element (the chainable error kind).", variant_ident),
								},

								"description" => custom_description = Some(CustomFormatter::Expr(syn::parse_str(value).unwrap_or_else(|err|
									panic!("Could not parse `description` attribute of member {} as an expression - {}", variant_ident, err)))),

								"display" => custom_display = Some(CustomFormatter::Expr(syn::parse_str(value).unwrap_or_else(|err|
									panic!("Could not parse `display` attribute of member {} as an expression - {}", variant_ident, err)))),

								"cause" => custom_cause = Some(syn::parse_str(value).unwrap_or_else(|err|
									panic!("Could not parse `cause` attribute of member {} as an expression - {}", variant_ident, err))),

								_ => panic!(
									"Could not parse `error_chain` attribute of member {} - expected one of `link`, `description`, `display`, `cause` but got {}",
									variant_ident, ident),
							}
						},

						_ => panic!("Could not parse `error_chain` attribute of member {} - expected term or name-value meta item", variant_ident),
					}
				}
			}
			else {
				let mut tts = {
					let mut tts = attr.tts.into_iter();

					let tt = match tts.next() {
						Some(proc_macro2::TokenTree::Group(ref group)) if group.delimiter() == proc_macro2::Delimiter::Parenthesis => group.stream(),
						Some(tt) => panic!("Could not parse `error_chain` attribute of member {} - expected `(tokens)` but found {}", variant_ident, tt),
						None => panic!("Could not parse `error_chain` attribute of member {} - expected `(tokens)`", variant_ident),
					};

					if let Some(tt) = tts.next() {
						panic!("Could not parse `error_chain` attribute of member {} - unexpected token {} after `(tokens)`", variant_ident, tt);
					}

					tt.into_iter()
				};

				let ident = match tts.next() {
					Some(proc_macro2::TokenTree::Term(ident)) => ident,
					Some(tt) => panic!("Could not parse `error_chain` attribute of member {} - expected a term but got {}", variant_ident, tt),
					None => panic!("Could not parse `error_chain` attribute of member {} - expected a term", variant_ident),
				};
				let ident = ident.as_str();

				match tts.next() {
					Some(proc_macro2::TokenTree::Op(op)) if op.op() == '=' => (),
					Some(tt) => panic!("Could not parse `error_chain` attribute of member {} - expected `=` but got {}", variant_ident, tt),
					None => panic!("Could not parse `error_chain` attribute of member {} - expected `=`", variant_ident),
				}

				let value: proc_macro2::TokenStream = tts.collect();
				if value.is_empty() {
					panic!("Could not parse `error_chain` attribute of member {} - expected tokens after `=`", variant_ident);
				}

				match ident {
					"link" => match variant_fields {
						syn::Fields::Unnamed(syn::FieldsUnnamed { ref unnamed, .. }) if unnamed.len() == 1 =>
							link_type = Some(LinkType::Chainable(
								syn::parse2(value).unwrap_or_else(|err|
									panic!("Could not parse `link` attribute of member {} as a type - {}", variant_ident, err)),
								unnamed[0].ty.clone())),

						_ => panic!("Chainable link {} must be a tuple of one element (the chainable error kind).", variant_ident),
					},

					"description" => custom_description = Some(CustomFormatter::parse(value, "description", &variant_ident, &variant_fields)),

					"display" => custom_display = Some(CustomFormatter::parse(value, "display", &variant_ident, &variant_fields)),

					"cause" => custom_cause = Some(syn::parse2(value).unwrap_or_else(|err|
						panic!("Could not parse `cause` attribute of member {} as an expression - {}", variant_ident, err))),

					_ => panic!(
						"Could not parse `error_chain` attribute of member {} - expected one of `link`, `description`, `display`, `cause` but got {}",
						variant_ident, ident),
				}
			}
		}

		let link_type = link_type.unwrap_or_else(||
			panic!(r#"Member {} does not have any of #[error_chain(link = "...")] or #[error_chain(foreign)] or #[error_chain(custom)]."#, variant_ident));

		Link {
			variant_ident,
			variant_fields,
			link_type,
			custom_description,
			custom_display,
			custom_cause,
		}
	}
}

impl Link {
	fn error_kind_description(&self, error_kind_name: &syn::Ident) -> quote::Tokens {
		let variant_ident = &self.variant_ident;

		match (self.custom_description.as_ref(), &self.link_type) {
			(_, &LinkType::Msg) => quote! {
				#error_kind_name::#variant_ident(ref s) => s,
			},

			(Some(&CustomFormatter::FormatString { ref format_string, .. }), &LinkType::Chainable(_, _)) |
			(Some(&CustomFormatter::FormatString { ref format_string, .. }), &LinkType::Foreign(_)) => quote! {
				#error_kind_name::#variant_ident(_) => #format_string,
			},

			(Some(&CustomFormatter::Expr(ref custom_description)), &LinkType::Chainable(_, _)) |
			(Some(&CustomFormatter::Expr(ref custom_description)), &LinkType::Foreign(_)) if is_closure(custom_description) => quote! {
				#error_kind_name::#variant_ident(ref err) => {
					#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
					{ (#custom_description)(err) }
				},
			},

			(Some(&CustomFormatter::Expr(ref custom_description)), &LinkType::Chainable(_, _)) |
			(Some(&CustomFormatter::Expr(ref custom_description)), &LinkType::Foreign(_)) => quote! {
				#error_kind_name::#variant_ident(ref err) => #custom_description(err),
			},

			(Some(&CustomFormatter::FormatString { ref format_string, .. }), &LinkType::Custom) => {
				let pattern = fields_pattern_ignore(&self.variant_fields);

				quote! {
					#error_kind_name::#variant_ident #pattern => #format_string,
				}
			},

			(Some(&CustomFormatter::Expr(ref custom_description)), &LinkType::Custom) => {
				let pattern = fields_pattern(&self.variant_fields);
				let args = args(&self.variant_fields);

				if is_closure(custom_description) {
					quote! {
						#error_kind_name::#variant_ident #pattern => {
							#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
							{ (#custom_description)(#args) }
						},
					}
				}
				else {
					quote! {
						#error_kind_name::#variant_ident #pattern => #custom_description(#args),
					}
				}
			},

			(None, &LinkType::Chainable(_, _)) => quote! {
				#error_kind_name::#variant_ident(ref kind) => kind.description(),
			},

			(None, &LinkType::Foreign(_)) => quote! {
				#error_kind_name::#variant_ident(ref err) => ::std::error::Error::description(err),
			},

			(None, &LinkType::Custom) => {
				let pattern = fields_pattern_ignore(&self.variant_fields);

				quote! {
					#error_kind_name::#variant_ident #pattern => stringify!(#variant_ident),
				}
			},
		}
	}

	fn error_kind_display_case(
		&self,
		error_kind_name: &syn::Ident,
	) -> quote::Tokens {
		let variant_ident = &self.variant_ident;

		match (self.custom_display.as_ref(), &self.link_type) {
			(_, &LinkType::Msg) => quote! {
				#error_kind_name::#variant_ident(ref s) => ::std::fmt::Display::fmt(s, f),
			},

			(Some(&CustomFormatter::FormatString { ref format_string, ref pattern, ref args }), &LinkType::Chainable(_, _)) => quote! {
				#error_kind_name::#variant_ident #pattern => write!(f, #format_string, #args),
			},

			(Some(&CustomFormatter::Expr(ref custom_display)), &LinkType::Chainable(_, _)) if is_closure(custom_display) => quote! {
				#error_kind_name::#variant_ident(ref kind) => {
					#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
					{ (#custom_display)(kind) }
				},
			},

			(Some(&CustomFormatter::Expr(ref custom_display)), &LinkType::Chainable(_, _)) => quote! {
				#error_kind_name::#variant_ident(ref kind) => #custom_display(f, kind),
			},

			(Some(&CustomFormatter::FormatString { ref format_string, ref pattern, ref args }), &LinkType::Foreign(_)) => quote! {
				#error_kind_name::#variant_ident #pattern => write!(f, #format_string, #args),
			},

			(Some(&CustomFormatter::Expr(ref custom_display)), &LinkType::Foreign(_)) if is_closure(custom_display) => quote! {
				#error_kind_name::#variant_ident(ref err) => {
					#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
					{ (#custom_display)(err) }
				},
			},

			(Some(&CustomFormatter::Expr(ref custom_display)), &LinkType::Foreign(_)) => quote! {
				#error_kind_name::#variant_ident(ref err) => #custom_display(f, err),
			},

			(Some(&CustomFormatter::FormatString { ref format_string, ref pattern, ref args }), &LinkType::Custom) => quote! {
				#error_kind_name::#variant_ident #pattern => write!(f, #format_string, #args),
			},

			(Some(&CustomFormatter::Expr(ref custom_display)), &LinkType::Custom) => {
				let pattern = fields_pattern(&self.variant_fields);
				let args = args(&self.variant_fields);

				if is_closure(custom_display) {
					quote! {
						#error_kind_name::#variant_ident #pattern => {
							#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
							{ (#custom_display)(#args) }
						},
					}
				}
				else {
					quote! {
						#error_kind_name::#variant_ident #pattern => #custom_display(f, #args),
					}
				}
			},

			(None, &LinkType::Chainable(_, _)) => quote! {
				#error_kind_name::#variant_ident(ref kind) => ::std::fmt::Display::fmt(kind, f),
			},

			(None, &LinkType::Foreign(_)) => quote! {
				#error_kind_name::#variant_ident(ref err) => ::std::fmt::Display::fmt(err, f),
			},

			(None, &LinkType::Custom) => {
				let pattern = fields_pattern_ignore(&self.variant_fields);

				quote! {
					#error_kind_name::#variant_ident #pattern => ::std::fmt::Display::fmt(self.description(), f),
				}
			},
		}
	}

	fn error_kind_from_impl(
		&self,
		error_kind_name: &syn::Ident,
		impl_generics: &syn::ImplGenerics, impl_generics_lifetime: &syn::ImplGenerics, ty_generics: &syn::TypeGenerics, where_clause: Option<&syn::WhereClause>,
	) -> Option<quote::Tokens> {
		let variant_ident = &self.variant_ident;

		match self.link_type {
			LinkType::Msg => Some(quote! {
				impl #impl_generics_lifetime From<&'__a str> for #error_kind_name #ty_generics #where_clause {
					fn from(s: &'__a str) -> Self { #error_kind_name::#variant_ident(s.to_string()) }
				}

				impl #impl_generics From<String> for #error_kind_name #ty_generics #where_clause {
					fn from(s: String) -> Self { #error_kind_name::#variant_ident(s) }
				}
			}),

			LinkType::Chainable(_, ref error_kind_ty) => Some(quote! {
				impl #impl_generics From<#error_kind_ty> for #error_kind_name #ty_generics #where_clause {
					fn from(kind: #error_kind_ty) -> Self {
						#error_kind_name::#variant_ident(kind)
					}
				}
			}),

			LinkType::Foreign(_) |
			LinkType::Custom => None,
		}
	}

	fn error_cause_case(
		&self,
		error_kind_name: &syn::Ident,
	) -> Option<quote::Tokens> {
		let variant_ident = &self.variant_ident;

		#[cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]
		match (self.custom_cause.as_ref(), &self.link_type) {
			(_, &LinkType::Msg) => None,

			(Some(custom_cause), _) => Some({
				let pattern = fields_pattern(&self.variant_fields);
				let args = args(&self.variant_fields);

				if is_closure(custom_cause) {
					quote! {
						#error_kind_name::#variant_ident #pattern => {
							#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
							let result = (#custom_cause)(#args);
							Some(result)
						},
					}
				}
				else {
					quote! {
						#error_kind_name::#variant_ident #pattern => Some(#custom_cause(#args)),
					}
				}
			}),

			(None, &LinkType::Foreign(_)) => Some(quote! {
				#error_kind_name::#variant_ident(ref err) => ::std::error::Error::cause(err),
			}),

			(None, &LinkType::Chainable(_, _)) |
			(None, &LinkType::Custom) => None,
		}
	}

	fn error_from_impl(
		&self,
		error_kind_name: &syn::Ident, error_name: &syn::Ident,
		generics: &std::collections::HashSet<syn::Ident>,
		impl_generics: &syn::ImplGenerics, impl_generics_lifetime: &syn::ImplGenerics, ty_generics: &syn::TypeGenerics, where_clause: Option<&syn::WhereClause>,
	) -> Option<quote::Tokens> {
		let variant_ident = &self.variant_ident;

		match self.link_type {
			LinkType::Msg => Some(quote! {
				impl #impl_generics_lifetime From<&'__a str> for #error_name #ty_generics #where_clause {
					fn from(s: &'__a str) -> Self { Self::from_kind(s.into()) }
				}

				impl #impl_generics From<String> for #error_name #ty_generics #where_clause {
					fn from(s: String) -> Self { Self::from_kind(s.into()) }
				}
			}),

			LinkType::Chainable(ref error_ty, _) => Some(quote! {
				impl #impl_generics From<#error_ty> for #error_name #ty_generics #where_clause {
					fn from(err: #error_ty) -> Self {
						#error_name(#error_kind_name::#variant_ident(err.0), err.1)
					}
				}
			}),

			// Don't emit From impl for any generics of the errorkind because they cause conflicting trait impl errors.
			// ie don't emit `impl From<T> for Error<T>` even if there's a variant `SomeError(T)`
			LinkType::Foreign(syn::Type::Path(syn::TypePath { ref path, .. }))
				if !path.global() && path.segments.len() == 1 && generics.contains(&path.segments[0].ident) => None,

			LinkType::Foreign(ref ty) => Some(quote! {
				impl #impl_generics From<#ty> for #error_name #ty_generics #where_clause {
					fn from(err: #ty) -> Self {
						Self::from_kind(#error_kind_name::#variant_ident(err))
					}
				}
			}),

			LinkType::Custom => None,
		}
	}

	fn chained_error_extract_backtrace_case(&self) -> Option<quote::Tokens> {
		match self.link_type {
			LinkType::Chainable(ref error_ty, _) => Some(quote! {
				if let Some(err) = err.downcast_ref::<#error_ty>() {
					return err.1.backtrace.clone();
				}
			}),

			LinkType::Msg |
			LinkType::Foreign(_) |
			LinkType::Custom => None,
		}
	}
}

enum CustomFormatter {
	FormatString { format_string: String, pattern: quote::Tokens, args: quote::Tokens },
	Expr(syn::Expr),
}

impl CustomFormatter {
	fn parse(tokens: proc_macro2::TokenStream, attr_name: &str, variant_ident: &syn::Ident, variant_fields: &syn::Fields) -> Self {
		let err = match syn::parse(tokens.clone().into()) {
			Ok(expr) => return CustomFormatter::Expr(expr),
			Err(err) => err,
		};

		let mut tts = tokens.into_iter();

		match tts.next() {
			Some(proc_macro2::TokenTree::Term(term)) if term.as_str() == "const" => (),

			Some(tt) => panic!(
				"Could not parse `{}` attribute of member {}. Expression - {}. Format string - expected `const` but got {}",
				attr_name, variant_ident, err, tt),

			_ => panic!(
				"Could not parse `{}` attribute of member {}. Expression - {}. Format string - expected `const`",
				attr_name, variant_ident, err),
		}

		let value = match tts.next() {
			Some(proc_macro2::TokenTree::Group(ref group)) if group.delimiter() == proc_macro2::Delimiter::Parenthesis => group.stream(),

			Some(tt) => panic!(
				"Could not parse `{}` attribute of member {} - expected `(string literal)` but got {}",
				attr_name, variant_ident, tt),

			_ => panic!(
				"Could not parse `{}` attribute of member {} - expected `(string literal)`",
				attr_name, variant_ident),
		};

		let format_string = match syn::parse2(value) {
			Ok(syn::Lit::Str(value)) => value.value(),

			Ok(lit) => panic!(
				"Could not parse `{}` attribute of member {} - expected string literal but got {}",
				attr_name, variant_ident, quote!(#lit).to_string()),

			Err(err) => panic!(
				"Could not parse `{}` attribute of member {} - {}",
				attr_name, variant_ident, err),
		};

		if let Some(tt) = tts.next() {
			panic!(
				"Could not parse `{}` attribute of member {} - unexpected token {} after string literal",
				attr_name, variant_ident, tt);
		}

		match *variant_fields {
			syn::Fields::Named(syn::FieldsNamed { ref named, .. }) => {
				let referenced_names = get_parameter_names(&format_string).unwrap_or_else(|err| panic!(
					"Could not parse `{}` attribute of member {} - {}",
					attr_name, variant_ident, err));

				let (patterns, args): (Vec<_>, Vec<_>) = named.into_iter().map(|f| {
					let field_name = f.ident.as_ref().unwrap();
					if referenced_names.contains(field_name) {
						(quote!(ref #field_name), quote!(#field_name = #field_name,))
					}
					else {
						let ignored_field_name: syn::Ident = format!("_{}", field_name).into();
						(quote!(#field_name: ref #ignored_field_name), quote!())
					}
				}).unzip();

				CustomFormatter::FormatString {
					format_string,
					pattern: quote!({ #(#patterns,)* }),
					args: quote!(#(#args)*),
				}
			},

			syn::Fields::Unnamed(syn::FieldsUnnamed { ref unnamed, .. }) => {
				let referenced_positions = get_parameter_positions(&format_string).unwrap_or_else(|err| panic!(
					"Could not parse `{}` attribute of member {} - {}",
					attr_name, variant_ident, err));

				let (patterns, args): (Vec<_>, Vec<_>) = unnamed.into_iter().enumerate().map(|(i, _)| {
					if referenced_positions.contains(&i) {
						let field_name: syn::Ident = format!("value{}", i).into();
						(quote!(ref #field_name), quote!(#field_name,))
					}
					else {
						(quote!(_), quote!())
					}
				}).unzip();

				CustomFormatter::FormatString {
					format_string,
					pattern: quote!((#(#patterns,)*)),
					args: quote!(#(#args)*),
				}
			},

			syn::Fields::Unit => {
				ensure_no_parameters(&format_string).unwrap_or_else(|err| panic!(
					"Could not parse `{}` attribute of member {} - {}",
					attr_name, variant_ident, err));

				CustomFormatter::FormatString {
					format_string,
					pattern: quote!(),
					args: quote!(),
				}
			},
		}
	}
}

fn is_error_chain_attribute(attr: &syn::Attribute) -> bool {
	if !attr.path.global() && attr.path.segments.len() == 1 {
		let segment = &attr.path.segments[0];
		return segment.ident == "error_chain" && segment.arguments.is_empty();
	}

	false
}

fn is_closure(expr: &syn::Expr) -> bool {
	if let syn::Expr::Closure(..) = *expr {
		true
	}
	else {
		false
	}
}

fn fields_pattern(variant_fields: &syn::Fields) -> quote::Tokens {
	match *variant_fields {
		syn::Fields::Named(syn::FieldsNamed { ref named, .. }) => {
			let fields = named.into_iter().map(|f| {
				let field_name = f.ident.as_ref().unwrap();
				quote!(ref #field_name)
			});
			quote!({ #(#fields,)* })
		},

		syn::Fields::Unnamed(syn::FieldsUnnamed { ref unnamed, .. }) => {
			let fields = unnamed.into_iter().enumerate().map(|(i, _)| {
				let field_name: syn::Ident = format!("value{}", i).into();
				quote!(ref #field_name)
			});
			quote!((#(#fields,)*))
		},

		syn::Fields::Unit => quote!(),
	}
}

fn fields_pattern_ignore(variant_fields: &syn::Fields) -> quote::Tokens {
	match *variant_fields {
		syn::Fields::Named(syn::FieldsNamed { .. }) => quote!({ .. }),
		syn::Fields::Unnamed(_) => quote!((..)),
		syn::Fields::Unit => quote!(),
	}
}

fn args(variant_fields: &syn::Fields) -> quote::Tokens {
	match *variant_fields {
		syn::Fields::Named(syn::FieldsNamed { ref named, .. }) => {
			let fields = named.into_iter().map(|f| {
				let field_name = f.ident.as_ref().unwrap();
				quote!(#field_name)
			});
			quote!(#(#fields,)*)
		},

		syn::Fields::Unnamed(syn::FieldsUnnamed { ref unnamed, .. }) => {
			let fields = unnamed.into_iter().enumerate().map(|(i, _)| {
				let field_name: syn::Ident = format!("value{}", i).into();
				quote!(#field_name)
			});
			quote!(#(#fields,)*)
		},

		syn::Fields::Unit => quote!(),
	}
}

fn get_parameter_names(format_string: &str) -> Result<std::collections::HashSet<syn::Ident>, String> {
	let parser = syntex_fmt_macros::Parser::new(format_string);

	parser
	.filter_map(|piece| match piece {
		syntex_fmt_macros::Piece::String(_) => None,

		syntex_fmt_macros::Piece::NextArgument(syntex_fmt_macros::Argument { position, .. }) => match position {
			syntex_fmt_macros::Position::ArgumentNext => Some(Err("expected named parameter but found `{}`".to_string())),
			syntex_fmt_macros::Position::ArgumentIs(index) => Some(Err(format!("expected named parameter but found `{{{}}}`", index))),
			syntex_fmt_macros::Position::ArgumentNamed(name) => Some(syn::parse_str(name).map_err(|err| format!("could not parse named parameter `{{{}}}` - {}", name, err))),
		},
	})
	.collect()
}

fn get_parameter_positions(format_string: &str) -> Result<std::collections::HashSet<usize>, String> {
	let parser = syntex_fmt_macros::Parser::new(format_string);

	parser
	.filter_map(|piece| match piece {
		syntex_fmt_macros::Piece::String(_) => None,

		syntex_fmt_macros::Piece::NextArgument(syntex_fmt_macros::Argument { position, .. }) => match position {
			syntex_fmt_macros::Position::ArgumentNext => Some(Err("expected positional parameter but found `{}`".to_string())),
			syntex_fmt_macros::Position::ArgumentIs(index) => Some(Ok(index)),
			syntex_fmt_macros::Position::ArgumentNamed(name) => Some(Err(format!("expected positional parameter but found `{{{}}}`", name))),
		},
	})
	.collect()
}

fn ensure_no_parameters(format_string: &str) -> Result<(), String> {
	let parser = syntex_fmt_macros::Parser::new(format_string);

	for piece in parser {
		match piece {
			syntex_fmt_macros::Piece::String(_) => (),

			syntex_fmt_macros::Piece::NextArgument(syntex_fmt_macros::Argument { position, .. }) => match position {
				syntex_fmt_macros::Position::ArgumentNext => return Err("expected no parameters but found `{}`".to_string()),
				syntex_fmt_macros::Position::ArgumentIs(index) => return Err(format!("expected no parameters but found `{{{}}}`", index)),
				syntex_fmt_macros::Position::ArgumentNamed(name) => return Err(format!("expected no parameters but found `{{{}}}`", name)),
			},
		}
	}

	Ok(())
}
