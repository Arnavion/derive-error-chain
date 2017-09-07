#![recursion_limit = "300"]

//! A Macros 1.1 implementation of https://crates.io/crates/error-chain
//!
//! The error-chain example
//!
//! ```ignore
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
//! ```ignore
//! mod other_error {
//!     #[derive(Debug, ErrorChain]
//!     pub enum ErrorKind {
//!         Msg(String),
//!     }
//! }
//!
//! #[derive(Debug, ErrorChain]
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
//! - The ErrorKind is an enum instead of a macro invocation.
//! - Error links are variants of the enum instead of lines inside the macro.
//! - Links have explicit annotations marking them as chainable / foreign / custom instead of being grouped into corresponding sections of the macro.
//! - Attributes like `#[cfg]` are applied to the variants directly instead of needing special syntax.
//! - `description` and `display` are defined as function expressions specified as attribute values, instead of shorthands integrated into the macro syntax.
//!
//! The less obvious differences are:
//!
//! - The ErrorKind must explicitly implement `::std::fmt::Debug`, either automatically using `#[derive]` or manually implemented separately. `error_chain!` does this implicitly.
//! - The ErrorKind must have `pub` visibility. `error_chain!` does this implicitly.
//! - The ErrorKind can have a special `Msg(String)` member for converting strings to the ErrorKind. `error_chain!` does this implicitly.
//! - Unlike `error-chain`, the `Msg(String)` member is optional. If absent, the ErrorKind and Error will not impl `From<String>` and `From<&str>`.
//! - Doc comments, since they're effectively attributes, can be applied on the enum variants without any special syntax like `error_chain!` has.
//! - The ErrorKind can be generic.
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
//!     ```ignore
//!     #[error_chain(link = "other_error::Error")]
//!     Another(other_error::ErrorKind),
//!     ```
//!
//!     A chainable link is an error and errorkind that have been generated using `error-chain` or `derive-error-chain`. The variant must have a single field
//!     to hold the chained errorkind, and the `link` attribute must specify a path to the chained error.
//!
//! - Foreign links
//!
//!     ```ignore
//!     #[error_chain(foreign)]
//!     Fmt(::std::fmt::Error),
//!     ```
//!
//!     A foreign link is an error that implements `::std::error::Error` but otherwise does not follow `error-chain`'s conventions. The variant must have
//!     a single field to hold the foreign error.
//!
//! - Custom links
//!
//!     ```ignore
//!     #[error_chain(custom)]
//!     InvalidToolchainName(String),
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
//!     ```ignore
//!         #[error_chain(description = r#"|_| "invalid toolchain name""#)]
//!         InvalidToolchainName(String),
//!     ```
//!
//!     or it can be a separate function:
//!
//!     ```ignore
//!         #[error_chain(description = "invalid_toolchain_name_error_description")]
//!         InvalidToolchainName(String),
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
//! - `#[error_chain(display = "some_function_expression")]`
//!
//!     Specifies a function expression to be used to implement `::std::fmt::Display::fmt()` on the `ErrorKind` and generated `Error`
//!
//!     This can be an inline lambda:
//!
//!     ```ignore
//!         #[error_chain(display = r#"|t| write!(f, "invalid toolchain name: '{}'", t)"#)]
//!         InvalidToolchainName(String),
//!     ```
//!
//!     or it can be a separate function:
//!
//!     ```ignore
//!         #[error_chain(display = "invalid_toolchain_name_error_display")]
//!         InvalidToolchainName(String),
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
//! - `#[error_chain(cause = "some_function_expression")]`
//!
//!     Specifies a function expression to be used to implement `::std::fmt::Error::cause()` on the generated `Error`
//!
//!     This can be an inline lambda:
//!
//!     ```ignore
//!         #[error_chain(cause = "|_, err| err")]
//!         JSON(::std::path::PathBuf, ::serde_json::Error),
//!     ```
//!
//!     or it can be a separate function:
//!
//!     ```ignore
//!         #[error_chain(cause = "parse_json_file_error_cause")]
//!         JSON(::std::path::PathBuf, ::serde_json::Error),
//!
//!     // <snip>
//!
//!     fn parse_json_file_error_cause<'a>(_: &::std::path::Path, err: &'a ::serde_json::Error) -> &'a ::std::error::Error {
//!         err
//!     }
//!     ```
//!
//!     The function expression must have the signature `(...) -> &::std::error::Error`. It should have one parameter for each field of the variant.
//!     The fields are passed in by reference. The result is wrapped in `Option::Some()` for returning from `::std::error::Error::cause()`
//!
//!     Thus in the above example, since `JSON` had two fields of type `::std::path::PathBuf` and `::serde_json::Error`, the function expression needed to be of type
//!     `(&::std::path::Path, &::serde_json::Error) -> &::std::error::Error`
//!
//!     If not specified, the default implementation of `::std::error::Error::cause()` behaves in this way:
//!
//!     - Chainable links: Returns `None`
//!     - Foreign links: Forwards to the foreign error's implementation of `::std::error::Error::cause()`
//!     - Custom links: Returns `None`
//!
//! # Conflicts with `error-chain` macros when the `proc_macro` feature is enabled
//!
//! If you have the `proc_macro` feature enabled and have code like this:
//!
//! ```ignore
//! #![feature(proc_macro)]
//!
//! #[macro_use] extern crate derive_error_chain;
//! #[macro_use] extern crate error_chain; // Want to use `bail!` and `quick_main!`
//!
//! #[derive(Debug, ErrorChain)]
//! enum ErrorKind {
//!     Msg(String),
//!
//!     #[error_chain(custom)]
//!     Code(i32),
//! }
//!
//! quick_main!(|| -> Result<()> {
//!     bail!("failed");
//! });
//! ```
//!
//! it'll fail to compile with:
//!
//! ```ignore
//! error: macro `error_chain` may not be used in attributes
//! ```
//!
//! This is because the compiler thinks `#[error_chain(custom)]` is the invocation of an attribute macro, notices that `error_chain!` is
//! a `macro_rules` macro brought into scope from the `error-chain` crate, and thus complains that a `macro_rules` macro cannot be used as
//! an attribute macro. It does this even though there is no attribute macro named `error_chain` and that the custom derive from this crate
//! has registered `error_chain` as an attribute it supports.
//!
//! See https://github.com/rust-lang/rust/issues/38356#issuecomment-324277403 for the discussion.
//!
//! To work around this, don't use `#[macro_use]` with the `error-chain` crate. Instead, either `use` the macros you need from it:
//!
//! ```ignore
//! #![feature(proc_macro)]
//!
//! #[macro_use] extern crate derive_error_chain;
//! extern crate error_chain;
//!
//! use error_chain::{ bail, quick_main };
//!
//! #[derive(Debug, ErrorChain)]
//! enum ErrorKind {
//!     Msg(String),
//!
//!     #[error_chain(custom)]
//!     Code(i32),
//! }
//!
//! quick_main!(|| -> Result<()> {
//!     bail!("failed")
//! });
//! ```
//!
//! or fully qualify their paths:
//!
//! ```ignore
//! #![feature(proc_macro)]
//!
//! #[macro_use] extern crate derive_error_chain;
//! extern crate error_chain;
//!
//! #[derive(Debug, ErrorChain)]
//! enum ErrorKind {
//!     Msg(String),
//!
//!     #[error_chain(custom)]
//!     Code(i32),
//! }
//!
//! error_chain::quick_main!(|| -> Result<()> {
//!     error_chain::bail!("failed")
//! });
//! ```
//!
//! `use`ing the `error_chain!` macro itself is more complicated: it must be renamed so that it doesn't just cause the above error again,
//! and other macros it uses must also be imported, even though they're an implementation detail:
//!
//! ```ignore
//! use error_chain::{ error_chain as error_chain_macro, error_chain_processing, impl_error_chain_kind, impl_error_chain_processed, impl_extract_backtrace };
//!
//! error_chain_macro! {
//! }
//! ```
//!
//! To use it fully-qualified, the macros it depends on must still be `use`d to bring them into scope:
//!
//! ```ignore
//! use error_chain::{ error_chain_processing, impl_error_chain_kind, impl_error_chain_processed, impl_extract_backtrace };
//!
//! error_chain::error_chain! {
//! }
//! ```
//!
//! It's possible this experience will be made better before the `proc_macro` feature stabilizes.

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

#[proc_macro_derive(ErrorChain, attributes(error_chain))]
pub fn derive_error_chain(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let source = input.to_string();
	let ast = syn::parse_derive_input(&source).unwrap();
	let error_kind_name = ast.ident;

	let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
	let mut generics_lifetime = ast.generics.clone();
	generics_lifetime.lifetimes.push(syn::LifetimeDef::new("'__a"));
	let (impl_generics_lifetime, _, _) = generics_lifetime.split_for_impl();

	let mut result_generics = ast.generics.clone();
	result_generics.ty_params.push(syn::TyParam {
		attrs: vec![],
		ident: syn::Ident::from("__T"),
		bounds: vec![],
		default: None,
	});
	let (_, result_ty_generics, _) = result_generics.split_for_impl();

	let mut result_ext_generics_t = ast.generics.clone();
	result_ext_generics_t.ty_params.push(syn::TyParam {
		attrs: vec![],
		ident: syn::Ident::from("__T"),
		bounds: vec![],
		default: None,
	});
	let (result_ext_impl_generics_t, result_ext_ty_generics_t, _) = result_ext_generics_t.split_for_impl();

	let mut result_ext_generics_t_e = result_ext_generics_t.clone();
	result_ext_generics_t_e.ty_params.push(syn::TyParam {
		attrs: vec![],
		ident: syn::Ident::from("__E"),
		bounds: vec![
			syn::parse_ty_param_bound("::std::error::Error").unwrap(),
			syn::parse_ty_param_bound("::std::marker::Send").unwrap(),
			syn::parse_ty_param_bound("'static").unwrap(),
		],
		default: None,
	});
	let (result_ext_impl_generics_t_e, _, _) = result_ext_generics_t_e.split_for_impl();

	let generics: std::collections::HashSet<_> = ast.generics.ty_params.iter().map(|ty_param| ty_param.ident.clone()).collect();

	let mut error_name = syn::Ident::from("Error");
	let mut result_ext_name = syn::Ident::from("ResultExt");
	let mut result_name = Some(syn::Ident::from("Result"));
	let mut support_backtrace = true;
	let mut has_msg = false;

	for attr in ast.attrs {
		if attr.name() == "doc" {
			continue;
		}

		match attr.value {
			syn::MetaItem::List(ident, nested_meta_items) => {
				if ident != "error_chain" {
					continue;
				}

				for nested_meta_item in nested_meta_items {
					match nested_meta_item {
						syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ident, syn::Lit::Str(value, _))) => match ident.as_ref() {
							"error" => error_name = syn::parse_ident(&value).unwrap_or_else(|err|
								panic!("Could not parse `error` value as an identifier - {}", err)),

							"result_ext" => result_ext_name = syn::parse_ident(&value).unwrap_or_else(|err|
								panic!("Could not parse `result_ext` value as an identifier - {}", err)),

							"result" => result_name =
								if value == "" {
									None
								}
								else {
									Some(syn::parse_ident(&value).unwrap_or_else(|err|
										panic!("Could not parse `result` value as an identifier - {}", err)))
								},

							"backtrace" => support_backtrace = value.parse().unwrap_or_else(|err|
								panic!("Could not parse `backtrace` value - {}", err)),

							_ =>
								panic!("Could not parse `error_chain` attribute - expected one of `error`, `result_ext`, `result`, `backtrace` but got {}", ident),
						},

						syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref ident, syn::Lit::Bool(value))) if ident == "backtrace" =>
							support_backtrace = value,

						_ => panic!("Could not parse `error_chain` attribute - expected one of `error`, `result_ext`, `result`, `backtrace` with a string or boolean value"),
					}
				}
			},

			_ => panic!("Could not parse `error_chain` attribute - expected one of `error`, `result_ext`, `result`, `backtrace`"),
		}
	}

	let error_chain_name = syn::parse_ident(&format!("{}_error_chain", error_name)).unwrap_or_else(|err|
		panic!("Could not generate error_chain crate name as a valid ident - {}", err));

	let result = match ast.body {
		syn::Body::Enum(variants) => {
			let mut links = vec![];

			for variant in variants {
				let syn::Variant { ident: variant_ident, attrs, data: variant_data, .. } = variant;

				if variant_ident == "Msg" {
					if let syn::VariantData::Tuple(ref fields) = variant_data {
						if fields.len() == 1 {
							if let syn::Ty::Path(None, syn::Path { global: false, ref segments }) = fields[0].ty {
								if segments.len() == 1 && segments[0].ident == "String" {
									has_msg = true;
									continue;
								}
							}
						}
					}

					panic!("Expected Msg member to be a tuple of String");
				}

				let mut link_type = None;
				let mut custom_description = None;
				let mut custom_display = None;
				let mut custom_cause = None;

				for attr in attrs {
					if let syn::MetaItem::List(ref ident, ref nested_meta_items) = attr.value {
						if ident != "error_chain" {
							continue;
						}

						for nested_meta_item in nested_meta_items {
							match *nested_meta_item {
								syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref ident)) => match ident.as_ref() {
									"foreign" => match variant_data {
										syn::VariantData::Tuple(ref fields) if fields.len() == 1 =>
											link_type = Some(LinkType::Foreign(fields[0].ty.clone())),

										_ => panic!("Foreign link {} must be a tuple of one element (the foreign error type).", variant_ident),
									},

									"custom" => link_type = Some(LinkType::Custom),

									_ => panic!(
										"Could not parse `error_chain` attribute of member {} - expected one of `foreign`, `custom` but got {}",
										variant_ident, ident),
								},

								syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref ident, syn::Lit::Str(ref value, _))) => match ident.as_ref() {
									"link" => match variant_data {
										syn::VariantData::Tuple(ref fields) if fields.len() == 1 =>
											link_type = Some(LinkType::Chainable(
												syn::parse_type(value).unwrap_or_else(|err|
													panic!("Could not parse `link` attribute of member {} as a type - {}", variant_ident, err)),
												fields[0].ty.clone())),

										_ => panic!("Chainable link {} must be a tuple of one element (the chainable error kind).", variant_ident),
									},

									"description" => custom_description = Some(syn::parse_expr(value).unwrap_or_else(|err|
										panic!("Could not parse `description` attribute of member {} as an expression - {}", variant_ident, err))),

									"display" => custom_display = Some(syn::parse_expr(value).unwrap_or_else(|err|
										panic!("Could not parse `display` attribute of member {} as an expression - {}", variant_ident, err))),

									"cause" => custom_cause = Some(syn::parse_expr(value).unwrap_or_else(|err|
										panic!("Could not parse `cause` attribute of member {} as an expression - {}", variant_ident, err))),

									_ => panic!(
										"Could not parse `error_chain` attribute of member {} - expected one of `link`, `description`, `display`, `cause` but got {}",
										variant_ident, ident),
								},

								_ => panic!("Could not parse `error_chain` attribute of member {} - expected word or name-value meta item", variant_ident),
							}
						}
					}
				}

				let link_type =
					link_type.unwrap_or_else(||
						panic!(r#"Member {} does not have any of #[error_chain(link = "...")] or #[error_chain(foreign)] or #[error_chain(custom)]."#, variant_ident));

				links.push(Link {
					variant_ident,
					variant_data,
					link_type,
					custom_description,
					custom_display,
					custom_cause,
				});
			}

			let error_kind_description_cases =
				std::iter::once(quote! {
					#error_kind_name::Msg(ref s) => s,
				}).filter(|_| has_msg)
				.chain(links.iter().map(|link| {
					let variant_ident = &link.variant_ident;

					match (link.custom_description.as_ref(), &link.link_type) {
						(Some(custom_description), &LinkType::Chainable(_, _)) |
						(Some(custom_description), &LinkType::Foreign(_)) if is_closure(custom_description) => quote! {
							#error_kind_name::#variant_ident(ref err) => {
								#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
								{ (#custom_description)(err) }
							},
						},

						(Some(custom_description), &LinkType::Chainable(_, _)) |
						(Some(custom_description), &LinkType::Foreign(_)) => quote! {
							#error_kind_name::#variant_ident(ref err) => #custom_description(err),
						},

						(Some(custom_description), &LinkType::Custom) => {
							let pattern = fields_pattern(&link.variant_data);
							let args = args(&link.variant_data);

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
							let pattern = fields_pattern_ignore(&link.variant_data);

							quote! {
								#error_kind_name::#variant_ident #pattern => stringify!(#variant_ident),
							}
						},
					}
				}));

			let error_kind_display_cases =
				std::iter::once(quote! {
					#error_kind_name::Msg(ref s) => ::std::fmt::Display::fmt(s, f),
				}).filter(|_| has_msg)
				.chain(links.iter().map(|link| {
					let variant_ident = &link.variant_ident;

					match (link.custom_display.as_ref(), &link.link_type) {
						(Some(custom_display), &LinkType::Chainable(_, _)) if is_closure(custom_display) => quote! {
							#error_kind_name::#variant_ident(ref kind) => {
								#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
								{ (#custom_display)(kind) }
							},
						},

						(Some(custom_display), &LinkType::Chainable(_, _)) => quote! {
							#error_kind_name::#variant_ident(ref kind) => #custom_display(f, kind),
						},

						(Some(custom_display), &LinkType::Foreign(_)) if is_closure(custom_display) => quote! {
							#error_kind_name::#variant_ident(ref err) => {
								#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
								{ (#custom_display)(err) }
							},
						},

						(Some(custom_display), &LinkType::Foreign(_)) => quote! {
							#error_kind_name::#variant_ident(ref err) => #custom_display(f, err),
						},

						(Some(custom_display), &LinkType::Custom) => {
							let pattern = fields_pattern(&link.variant_data);
							let args = args(&link.variant_data);

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
							let pattern = fields_pattern_ignore(&link.variant_data);

							quote! {
								#error_kind_name::#variant_ident #pattern => ::std::fmt::Display::fmt(self.description(), f),
							}
						},
					}
				}));

			let error_kind_from_impls =
				std::iter::once(Some(quote! {
					impl #impl_generics_lifetime From<&'__a str> for #error_kind_name #ty_generics #where_clause {
						fn from(s: &'__a str) -> Self { #error_kind_name::Msg(s.to_string()) }
					}

					impl #impl_generics From<String> for #error_kind_name #ty_generics #where_clause {
						fn from(s: String) -> Self { #error_kind_name::Msg(s) }
					}
				})).filter(|_| has_msg)
				.chain(links.iter().map(|link| match link.link_type {
					LinkType::Chainable(_, ref error_kind_ty) => {
						let variant_ident = &link.variant_ident;
						Some(quote! {
							impl #impl_generics From<#error_kind_ty> for #error_kind_name #ty_generics #where_clause {
								fn from(kind: #error_kind_ty) -> Self {
									#error_kind_name::#variant_ident(kind)
								}
							}
						})
					},

					LinkType::Foreign(_) |
					LinkType::Custom => None,
				}));

			let error_cause_cases = links.iter().filter_map(|link| {
				let variant_ident = &link.variant_ident;

				match (link.custom_cause.as_ref(), &link.link_type) {
					(Some(ref custom_cause), _) => Some({
						let pattern = fields_pattern(&link.variant_data);
						let args = args(&link.variant_data);

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
			});

			let error_doc_comment = format!(r"The Error type.

This struct is made of three things:

- `{0}` which is used to determine the type of the error.
- a backtrace, generated when the error is created.
- an error chain, used for the implementation of `Error::cause()`.", error_kind_name);

			let chained_error_extract_backtrace_cases = links.iter().filter_map(|link| match link.link_type {
				LinkType::Chainable(ref error_ty, _) => {
					Some(quote! {
						if let Some(err) = err.downcast_ref::<#error_ty>() {
							return err.1.backtrace.clone();
						}
					})
				},

				LinkType::Foreign(_) |
				LinkType::Custom => None,
			});

			let error_from_impls =
				std::iter::once(quote! {
					impl #impl_generics_lifetime From<&'__a str> for #error_name #ty_generics #where_clause {
						fn from(s: &'__a str) -> Self { Self::from_kind(s.into()) }
					}

					impl #impl_generics From<String> for #error_name #ty_generics #where_clause {
						fn from(s: String) -> Self { Self::from_kind(s.into()) }
					}
				}).filter(|_| has_msg)
				.chain(links.iter().filter_map(|link| match link.link_type {
					LinkType::Chainable(ref error_ty, _) => {
						let variant_ident = &link.variant_ident;
						Some(quote! {
							impl #impl_generics From<#error_ty> for #error_name #ty_generics #where_clause {
								fn from(err: #error_ty) -> Self {
									#error_name(#error_kind_name::#variant_ident(err.0), err.1)
								}
							}
						})
					},

					// Don't emit From impl for any generics of the errorkind because they cause conflicting trait impl errors.
					// ie don't emit `impl From<T> for Error<T>` even if there's a variant `SomeError(T)`
					LinkType::Foreign(syn::Ty::Path(_, syn::Path { global: false, ref segments }))
						if segments.len() == 1 && generics.contains(&segments[0].ident) => None,

					LinkType::Foreign(ref ty) => {
						let variant_ident = &link.variant_ident;
						Some(quote! {
							impl #impl_generics From<#ty> for #error_name #ty_generics #where_clause {
								fn from(err: #ty) -> Self {
									Self::from_kind(#error_kind_name::#variant_ident(err))
								}
							}
						})
					},

					LinkType::Custom => None,
				}));

			let extract_backtrace_fn = if support_backtrace {
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
				pub type #result_name #result_ty_generics = ::std::result::Result<__T, #error_name #ty_generics>;
			});

			quote! {
				extern crate error_chain as #error_chain_name;

				impl #impl_generics #error_kind_name #ty_generics #where_clause {
					/// A string describing the error kind.
					pub fn description(&self) -> &str {
						match *self {
							#(#error_kind_description_cases)*
						}
					}
				}

				impl #impl_generics ::std::fmt::Display for #error_kind_name #ty_generics #where_clause {
					fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
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
				pub struct #error_name #impl_generics (
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
				pub trait #result_ext_name #result_ext_impl_generics_t #where_clause {
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

	result.parse().unwrap()
}

struct Link {
	variant_ident: syn::Ident,
	variant_data: syn::VariantData,
	link_type: LinkType,
	custom_description: Option<syn::Expr>,
	custom_display: Option<syn::Expr>,
	custom_cause: Option<syn::Expr>,
}

enum LinkType {
	Chainable(syn::Ty, syn::Ty),
	Foreign(syn::Ty),
	Custom,
}

fn is_closure(expr: &syn::Expr) -> bool {
	if let syn::ExprKind::Closure(..) = expr.node {
		true
	}
	else {
		false
	}
}

fn fields_pattern(variant_data: &syn::VariantData) -> quote::Tokens {
	match *variant_data {
		syn::VariantData::Struct(ref fields) => {
			let fields = fields.iter().map(|f| {
				let field_name = f.ident.as_ref().unwrap();
				quote!(ref #field_name)
			});
			quote!({ #(#fields,)* })
		},

		syn::VariantData::Tuple(ref fields) => {
			let fields = fields.iter().enumerate().map(|(i, _)| {
				let field_name = syn::Ident::from(format!("value{}", i));
				quote!(ref #field_name)
			});
			quote!((#(#fields,)*))
		},

		syn::VariantData::Unit => quote!(),
	}
}

fn fields_pattern_ignore(variant_data: &syn::VariantData) -> quote::Tokens {
	match *variant_data {
		syn::VariantData::Struct(_) => quote!({ .. }),
		syn::VariantData::Tuple(_) => quote!((..)),
		syn::VariantData::Unit => quote!(),
	}
}

fn args(variant_data: &syn::VariantData) -> quote::Tokens {
	match *variant_data {
		syn::VariantData::Struct(ref fields) => {
			let fields = fields.iter().map(|f| {
				let field_name = f.ident.as_ref().unwrap();
				quote!(#field_name)
			});
			quote!(#(#fields,)*)
		},

		syn::VariantData::Tuple(ref fields) => {
			let fields = fields.iter().enumerate().map(|(i, _)| {
				let field_name = syn::Ident::from(format!("value{}", i));
				quote!(#field_name)
			});
			quote!(#(#fields,)*)
		},

		syn::VariantData::Unit => quote!(),
	}
}
