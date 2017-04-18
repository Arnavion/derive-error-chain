#![crate_type = "proc-macro"]
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
//!     #[derive(Debug, error_chain)]
//!     pub enum ErrorKind {
//!         Msg(String),
//!     }
//! }
//!
//! #[derive(Debug, error_chain)]
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
//! - The ErrorKind must have a special `Msg(String)` member. `error_chain!` does this implicitly.
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
//! # Notes
//!
//! If you want to use other macros from the `error_chain` like `bail!`, note that the following code:
//!
//! ```ignore
//! #[macro_use] extern crate derive_error_chain;
//! #[macro_use] extern crate error_chain;
//!
//! #[derive(Debug, error_chain)]
//! enum ErrorKind {
//!     Msg(String),
//! }
//! ```
//!
//! will fail to compile with:
//!
//! ```ignore
//! error: macro `error_chain` may not be used for derive attributes
//! ```
//!
//! This is because both crates export a macro named `error_chain` and the macro from the second crate overrides the first.
//!
//! To fix this, import `error_chain` before `derive_error_chain`:
//!
//! ```ignore
//! #[macro_use] extern crate error_chain;
//! #[macro_use] extern crate derive_error_chain;
//! ```
//!
//! or use a fully-qualified path for the custom derive (nightly only):
//!
//! ```ignore
//! #![feature(proc_macro)]
//!
//! extern crate derive_error_chain;
//! #[macro_use] extern crate error_chain;
//!
//! #[derive(Debug, derive_error_chain::error_chain)]
//! enum ErrorKind {
//!     Msg(String),
//! }
//! ```

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

#[proc_macro_derive(error_chain, attributes(error_chain))]
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
	let mut result_ext_generics = ast.generics.clone();
	result_ext_generics.ty_params.push(syn::TyParam {
		attrs: vec![],
		ident: syn::Ident::from("__T"),
		bounds: vec![],
		default: None,
	});
	result_ext_generics.ty_params.push(syn::TyParam {
		attrs: vec![],
		ident: syn::Ident::from("__E"),
		bounds: vec![
			syn::parse_ty_param_bound("::std::error::Error").unwrap(),
			syn::parse_ty_param_bound("::std::marker::Send").unwrap(),
			syn::parse_ty_param_bound("'static").unwrap(),
		],
		default: None,
	});
	let (result_ext_impl_generics, result_ext_ty_generics, _) = result_ext_generics.split_for_impl();

	let generics: std::collections::HashSet<_> = ast.generics.ty_params.iter().map(|ty_param| ty_param.ident.clone()).collect();

	let mut error_name = syn::parse_ident("Error").unwrap();
	let mut result_ext_name = syn::parse_ident("ResultExt").unwrap();
	let mut result_name = Some(syn::parse_ident("Result").unwrap());
	let mut support_backtrace = true;

	for attr in ast.attrs {
		match attr.value {
			syn::MetaItem::List(ref ident, ref nested_meta_items) if ident == "error_chain" => {
				for nested_meta_item in nested_meta_items {
					match *nested_meta_item {
						syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref ident, syn::Lit::Str(ref value, _))) => {
							if ident == "error" {
								error_name = syn::parse_ident(value).map_err(|err| format!("couldn't parse error attribute as an identifier - {}", err)).unwrap()
							}
							else if ident == "result_ext" {
								result_ext_name = syn::parse_ident(value).map_err(|err| format!("couldn't parse result_ext attribute as an identifier - {}", err)).unwrap()
							}
							else if ident == "result" {
								result_name =
									if value == "" {
										None
									}
									else {
										Some(syn::parse_ident(value).map_err(|err| format!("couldn't parse result attribute as an identifier - {}", err)).unwrap())
									}
							}
							else if ident == "backtrace" && value == "false" {
								support_backtrace = false
							}
						},

						syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref ident, syn::Lit::Bool(false))) if ident == "backtrace" =>
							support_backtrace = false,

						_ => { },
					}
				}
			},

			_ => { },
		}
	}

	let error_chain_name = syn::parse_ident(&(error_name.to_string() + "_error_chain")).unwrap();

	struct Link {
		variant: syn::Variant,
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

	let result = match ast.body {
		syn::Body::Enum(variants) => {
			let mut links = vec![];

			for variant in variants {
				if &variant.ident == "Msg" {
					continue;
				}

				let mut link_type = None;
				let mut custom_description = None;
				let mut custom_display = None;
				let mut custom_cause = None;

				for attr in &variant.attrs {
					if let syn::MetaItem::List(ref ident, ref nested_meta_items) = attr.value {
						if ident == "error_chain" {
							for nested_meta_item in nested_meta_items {
								match *nested_meta_item {
									syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref ident)) => {
										if ident == "foreign" {
											match variant.data {
												syn::VariantData::Tuple(ref fields) if fields.len() == 1 =>
													link_type = Some(LinkType::Foreign(fields[0].ty.clone())),

												_ => panic!("Foreign link {} must be a tuple of one element (the foreign error type).", variant.ident),
											}
										}
										else if ident == "custom" {
											link_type = Some(LinkType::Custom);
										}
									},

									syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref ident, syn::Lit::Str(ref value, _))) => {
										if ident == "link" {
											match variant.data {
												syn::VariantData::Tuple(ref fields) if fields.len() == 1 =>
													link_type = Some(LinkType::Chainable(
														syn::parse_type(value).unwrap_or_else(|err| {
															let variant_name = &variant.ident;
															panic!("Could not parse link attribute of member {} as a type - {}", variant_name, err)
														}),
														fields[0].ty.clone())),

												_ => panic!("Chainable link {} must be a tuple of one element (the chainable error kind).", variant.ident),
											}
										}
										else if ident == "description" {
											custom_description = Some(syn::parse_expr(value).unwrap());
										}
										else if ident == "display" {
											custom_display = Some(syn::parse_expr(value).unwrap());
										}
										else if ident == "cause" {
											custom_cause = Some(syn::parse_expr(value).unwrap());
										}
									},

									_ => { },
								}
							}
						}
					}
				}

				let link_type =
					link_type.unwrap_or_else(|| {
						let variant_name = &variant.ident;
						panic!(r#"Member {} does not have any of #[error_chain(link = "...")] or #[error_chain(foreign)] or #[error_chain(custom)]."#, variant_name)
					});

				links.push(Link {
					variant: variant,
					link_type: link_type,
					custom_description: custom_description,
					custom_display: custom_display,
					custom_cause: custom_cause,
				});
			}

			let error_kind_description_cases = links.iter().map(|link| {
				let variant_name = &link.variant.ident;

				match (link.custom_description.as_ref(), &link.link_type) {
					(Some(custom_description), &LinkType::Chainable(_, _)) |
					(Some(custom_description), &LinkType::Foreign(_)) if is_closure(custom_description) => quote! {
						#error_kind_name::#variant_name(ref err) => {
							#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
							let result = (#custom_description)(err);
							result
						},
					},

					(Some(custom_description), &LinkType::Chainable(_, _)) |
					(Some(custom_description), &LinkType::Foreign(_)) => quote! {
						#error_kind_name::#variant_name(ref err) => #custom_description(err),
					},

					(Some(custom_description), &LinkType::Custom) => {
						let pattern = fields_pattern(&link.variant);
						let args = args(&link.variant);

						if is_closure(custom_description) {
							quote! {
								#error_kind_name::#variant_name #pattern => {
									#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
									let result = (#custom_description)(#args);
									result
								},
							}
						}
						else {
							quote! {
								#error_kind_name::#variant_name #pattern => #custom_description(#args),
							}
						}
					},

					(None, &LinkType::Chainable(_, _)) => quote! {
						#error_kind_name::#variant_name(ref kind) => kind.description(),
					},

					(None, &LinkType::Foreign(_)) => quote! {
						#error_kind_name::#variant_name(ref err) => ::std::error::Error::description(err),
					},

					(None, &LinkType::Custom) => {
						let pattern = fields_pattern_ignore(&link.variant);

						quote! {
							#error_kind_name::#variant_name #pattern => stringify!(#variant_name),
						}
					},
				}
			});

			let error_kind_display_cases = links.iter().map(|link| {
				let variant_name = &link.variant.ident;

				match (link.custom_display.as_ref(), &link.link_type) {
					(Some(custom_display), &LinkType::Chainable(_, _)) if is_closure(custom_display) => quote! {
						#error_kind_name::#variant_name(ref kind) => {
							#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
							let result = (#custom_display)(kind);
							result
						},
					},

					(Some(custom_display), &LinkType::Chainable(_, _)) => quote! {
						#error_kind_name::#variant_name(ref kind) => #custom_display(f, kind),
					},

					(Some(custom_display), &LinkType::Foreign(_)) if is_closure(custom_display) => quote! {
						#error_kind_name::#variant_name(ref err) => {
							#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
							let result = (#custom_display)(err);
							result
						},
					},

					(Some(custom_display), &LinkType::Foreign(_)) => quote! {
						#error_kind_name::#variant_name(ref err) => #custom_display(f, err),
					},

					(Some(custom_display), &LinkType::Custom) => {
						let pattern = fields_pattern(&link.variant);
						let args = args(&link.variant);

						if is_closure(custom_display) {
							quote! {
								#error_kind_name::#variant_name #pattern => {
									#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
									let result = (#custom_display)(#args);
									result
								},
							}
						}
						else {
							quote! {
								#error_kind_name::#variant_name #pattern => #custom_display(f, #args),
							}
						}
					},

					(None, &LinkType::Chainable(_, _)) => quote! {
						#error_kind_name::#variant_name(ref kind) => ::std::fmt::Display::fmt(kind, f),
					},

					(None, &LinkType::Foreign(_)) => quote! {
						#error_kind_name::#variant_name(ref err) => ::std::fmt::Display::fmt(err, f),
					},

					(None, &LinkType::Custom) => {
						let pattern = fields_pattern_ignore(&link.variant);

						quote! {
							#error_kind_name::#variant_name #pattern => ::std::fmt::Display::fmt(self.description(), f),
						}
					},
				}
			});

			let error_kind_from_impls = links.iter().map(|link| match link.link_type {
				LinkType::Chainable(_, ref error_kind_ty) => {
					let variant_name = &link.variant.ident;
					Some(quote! {
						impl #impl_generics From<#error_kind_ty> for #error_kind_name #ty_generics #where_clause {
							fn from(kind: #error_kind_ty) -> Self {
								#error_kind_name::#variant_name(kind)
							}
						}
					})
				},

				LinkType::Foreign(_) |
				LinkType::Custom => None,
			});

			let error_cause_cases = links.iter().filter_map(|link| {
				let variant_name = &link.variant.ident;

				match (link.custom_cause.as_ref(), &link.link_type) {
					(Some(ref custom_cause), _) => Some({
						let pattern = fields_pattern(&link.variant);
						let args = args(&link.variant);

						if is_closure(custom_cause) {
							quote! {
								#error_kind_name::#variant_name #pattern => {
									#[cfg_attr(feature = "cargo-clippy", allow(redundant_closure_call))]
									let result = (#custom_cause)(#args);
									Some(result)
								},
							}
						}
						else {
							quote! {
								#error_kind_name::#variant_name #pattern => Some(#custom_cause(#args)),
							}
						}
					}),

					(None, &LinkType::Foreign(_)) => Some(quote! {
						#error_kind_name::#variant_name(ref err) => ::std::error::Error::cause(err),
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

			let error_from_impls = links.iter().filter_map(|link| match link.link_type {
				LinkType::Chainable(ref error_ty, _) => {
					let variant_name = &link.variant.ident;
					Some(quote! {
						impl #impl_generics From<#error_ty> for #error_name #ty_generics #where_clause {
							fn from(err: #error_ty) -> Self {
								#error_name(#error_kind_name::#variant_name(err.0), err.1)
							}
						}
					})
				},

				// Don't emit From impl for any generics of the errorkind because they cause conflicting trait impl errors.
				// ie don't emit `impl From<T> for Error<T>` even if there's a variant `SomeError(T)`
				LinkType::Foreign(syn::Ty::Path(_, syn::Path { global: false, ref segments })) if segments.len() == 1 && generics.contains(&segments[0].ident) => None,

				LinkType::Foreign(ref ty) => {
					let variant_name = &link.variant.ident;
					Some(quote! {
						impl #impl_generics From<#ty> for #error_name #ty_generics #where_clause {
							fn from(err: #ty) -> Self {
								Self::from_kind(#error_kind_name::#variant_name(err))
							}
						}
					})
				},

				LinkType::Custom => None,
			});

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
							#error_kind_name::Msg(ref s) => s,

							#(#error_kind_description_cases)*
						}
					}
				}

				impl #impl_generics ::std::fmt::Display for #error_kind_name #ty_generics #where_clause {
					fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
						match *self {
							#error_kind_name::Msg(ref s) => ::std::fmt::Display::fmt(s, f),

							#(#error_kind_display_cases)*
						}
					}
				}

				#(#error_kind_from_impls)*

				impl #impl_generics_lifetime From<&'__a str> for #error_kind_name #ty_generics #where_clause {
					fn from(s: &'__a str) -> Self { #error_kind_name::Msg(s.to_string()) }
				}

				impl #impl_generics From<String> for #error_kind_name #ty_generics #where_clause {
					fn from(s: String) -> Self { #error_kind_name::Msg(s) }
				}

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
					pub fn from_kind(kind: #error_kind_name #ty_generics) -> #error_name #ty_generics {
						#error_name(kind, #error_chain_name::State::default())
					}

					/// Constructs a chained error from another error and a kind, and generates a backtrace.
					pub fn with_chain<__E, __K>(error: __E, kind: __K) -> Self
						where __E: ::std::error::Error + Send + 'static, __K: Into<#error_kind_name #ty_generics> {

						#error_name(kind.into(), #error_chain_name::State::new::<#error_name #ty_generics>(Box::new(error)))
					}

					/// Returns the kind of the error.
					pub fn kind(&self) -> &#error_kind_name #ty_generics { &self.0 }

					/// Iterates over the error chain.
					pub fn iter(&self) -> #error_chain_name::ErrorChainIter {
						#error_chain_name::ChainedError::iter(self)
					}

					/// Returns the backtrace associated with this error.
					pub fn backtrace(&self) -> Option<&#error_chain_name::Backtrace> {
						self.1.backtrace()
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

				impl #impl_generics_lifetime From<&'__a str> for #error_name #ty_generics #where_clause {
					fn from(s: &'__a str) -> Self { Self::from_kind(s.into()) }
				}

				impl #impl_generics From<String> for #error_name #ty_generics #where_clause {
					fn from(s: String) -> Self { Self::from_kind(s.into()) }
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

					fn iter(&self) -> #error_chain_name::ErrorChainIter {
						#error_chain_name::ErrorChainIter(Some(self))
					}

					fn backtrace(&self) -> Option<&#error_chain_name::Backtrace> {
						self.backtrace()
					}

					#extract_backtrace_fn
				}

				/// Additional methods for `Result`, for easy interaction with this crate.
				pub trait #result_ext_name #result_ext_impl_generics #where_clause {
					#[doc = #result_ext_chain_err_doc_comment]
					fn chain_err<__F, __EK>(self, callback: __F) -> ::std::result::Result<__T, #error_name #ty_generics>
						where __F: FnOnce() -> __EK, __EK: Into<#error_kind_name #ty_generics>;
				}

				impl #result_ext_impl_generics #result_ext_name #result_ext_ty_generics for ::std::result::Result<__T, __E> #where_clause {
					fn chain_err<__F, __EK>(self, callback: __F) -> ::std::result::Result<__T, #error_name #ty_generics>
						where __F: FnOnce() -> __EK, __EK: Into<#error_kind_name #ty_generics> {
						self.map_err(move |e| {
							let state = #error_chain_name::State::new::<#error_name #ty_generics>(Box::new(e));
							#error_chain_name::ChainedError::new(callback().into(), state)
						})
					}
				}

				#result_wrapper
			}
		},

		_ => panic!("#[derive(error_chain)] can only be used with enums."),
	};

	result.parse().unwrap()
}

fn is_closure(expr: &syn::Expr) -> bool {
	if let syn::ExprKind::Closure(..) = expr.node {
		true
	}
	else {
		false
	}
}

fn fields_pattern(variant: &syn::Variant) -> quote::Tokens {
	match variant.data {
		syn::VariantData::Struct(ref fields) => {
			let fields = fields.iter().map(|f| {
				let field_name = &f.ident;
				quote!(ref #field_name)
			});
			quote!({ #(#fields),* })
		},

		syn::VariantData::Tuple(ref fields) => {
			let fields = fields.iter().enumerate().map(|(i, _)| {
				let field_name = syn::parse_ident(&format!("value{}", i)).unwrap();
				quote!(ref #field_name)
			});
			quote!((#(#fields),*))
		},

		syn::VariantData::Unit => quote!(),
	}
}

fn fields_pattern_ignore(variant: &syn::Variant) -> quote::Tokens {
	match variant.data {
		syn::VariantData::Struct(_) => quote!({ .. }),
		syn::VariantData::Tuple(_) => quote!((..)),
		syn::VariantData::Unit => quote!(),
	}
}

fn args(variant: &syn::Variant) -> quote::Tokens {
	match variant.data {
		syn::VariantData::Struct(ref fields) => {
			let fields = fields.iter().map(|f| {
				let field_name = &f.ident;
				quote!(#field_name)
			});
			quote!(#(#fields),*)
		},

		syn::VariantData::Tuple(ref fields) => {
			let fields = fields.iter().enumerate().map(|(i, _)| {
				let field_name = syn::parse_ident(&format!("value{}", i)).unwrap();
				quote!(#field_name)
			});
			quote!(#(#fields),*)
		},

		syn::VariantData::Unit => quote!(),
	}
}
