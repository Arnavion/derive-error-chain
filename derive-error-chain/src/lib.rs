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
//! error_chain {
//!     types { Error, ErrorKind, ResultExt, Result; }
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
//!     	InvalidToolchainName(t: String) {
//!     		description("invalid toolchain name")
//!     		display("invalid toolchain name: '{}'", t)
//!     	}
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
//!         Msg(String), // A special variant that must always be present.
//!     }
//! }
//!
//! #[derive(Debug, error_chain)]
//! // This attribute is optional if using the default names "Error", "ResultExt" and "Result".
//! #[error_chain(error = "Error", result_ext = "ResultExt", result = "Result")]
//! pub enum ErrorKind {
//!     Msg(String), // A special variant that must always be present.
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
//!     #[error_chain(custom, description = "invalid_toolchain_name_description", display = "invalid_toolchain_name_display")]
//!     InvalidToolchainName(String),
//! }
//!
//! // A description function receives refs to all the variant constituents, and should return a &str
//! fn invalid_toolchain_name_description(_: &str) -> &str {
//!     "invalid toolchain name"
//! }
//!
//! // A display function receives a formatter and refs to all the variant constituents, and should return a ::std::fmt::Result
//! fn invalid_toolchain_name_display(f: &mut ::std::fmt::Formatter, t: &str) -> ::std::fmt::Result {
//!     write!(f, "invalid toolchain name: '{}'", t)
//! }
//! ```
//!
//! Notes:
//!
//! - This library requires the nightly compiler to be able to use the `proc_macro` rust feature.
//! - The macro output can be used with `#[deny(missing_docs)]` since it allows doc comments on the ErrorKind variants.
//! - The result wrapper can be disabled by setting `result = ""` in the `#[error_chain]` attribute on the ErrorKind.
//! - The backtrace functionality can be disabled by setting `backtrace = "false"` or `backtrace = false` in the `#[error_chain]` attribute on the ErrorKind.
//! - The ErrorKind must have a special `Msg(String)` member. Unlike error_chain which adds this member implicitly, this macro requires it explicitly.
//! - The description and display functions can be inlined like this:
//!
//!    ```ignore
//!     #[error_chain(custom)]
//!     #[error_chain(description = r#"(|_| "invalid toolchain name")"#)]
//!     #[error_chain(display = r#"(|f: &mut ::std::fmt::Formatter, t| write!(f, "invalid toolchain name: '{}'", t))"#)]
//!     InvalidToolchainName(String),
//!    ```

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

#[proc_macro_derive(error_chain, attributes(error_chain))]
pub fn derive_error_chain(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let source = input.to_string();
	let ast = syn::parse_macro_input(&source).unwrap();
	let error_kind_name = ast.ident;

	let mut error_name = syn::parse_ident("Error").unwrap();
	let mut result_ext_name = syn::parse_ident("ResultExt").unwrap();
	let mut result_name = Some(syn::parse_ident("Result").unwrap());
	let mut support_backtrace = true;

	for attr in ast.attrs {
		match &attr.value {
			&syn::MetaItem::List(ref ident, ref nested_meta_items) if ident == "error_chain" => {
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

				for attr in &variant.attrs {
					if let syn::MetaItem::List(ref ident, ref nested_meta_items) = attr.value {
						if ident == "error_chain" {
							for nested_meta_item in nested_meta_items {
								match *nested_meta_item {
									syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref ident)) => {
										if ident == "foreign" {
											match &variant.data {
												&syn::VariantData::Tuple(ref fields) if fields.len() == 1 =>
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
											match &variant.data {
												&syn::VariantData::Tuple(ref fields) if fields.len() == 1 =>
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
				});
			}

			let error_kind_description_cases = links.iter().map(|link| match link.link_type {
				LinkType::Chainable(_, _) => {
					let variant_name = &link.variant.ident;
					match link.custom_description {
						Some(ref custom_description) => quote! {
							#error_kind_name::#variant_name(ref kind) => #custom_description(err),
						},

						None => quote! {
							#error_kind_name::#variant_name(ref kind) => kind.description(),
						},
					}
				},

				LinkType::Foreign(_) => {
					let variant_name = &link.variant.ident;
					match link.custom_description {
						Some(ref custom_description) => quote! {
							#error_kind_name::#variant_name(ref err) => #custom_description(err),
						},

						None => quote! {
							#error_kind_name::#variant_name(ref err) => ::std::error::Error::description(err),
						},
					}
				},

				LinkType::Custom => {
					let variant_name = &link.variant.ident;
					match link.custom_description {
						Some(ref custom_description) => {
							let pattern = fields_pattern(&link.variant);
							let args = args(&link.variant);
							quote! {
								#error_kind_name::#variant_name #pattern => #custom_description(#args),
							}
						},

						None => {
							let pattern = fields_pattern_ignore(&link.variant);
							quote! {
								#error_kind_name::#variant_name #pattern => stringify!(#variant_name),
							}
						},
					}
				},
			});

			let error_kind_display_cases = links.iter().map(|link| match link.link_type {
				LinkType::Chainable(_, _) => {
					let variant_name = &link.variant.ident;
					match link.custom_display {
						Some(ref custom_display) => quote! {
							#error_kind_name::#variant_name(ref kind) => #custom_display(f, kind),
						},

						None => quote! {
							#error_kind_name::#variant_name(ref kind) => ::std::fmt::Display::fmt(kind, f),
						},
					}
				},

				LinkType::Foreign(_) => {
					let variant_name = &link.variant.ident;
					match link.custom_display {
						Some(ref custom_display) => quote! {
							#error_kind_name::#variant_name(ref err) => #custom_display(f, err),
						},

						None => quote! {
							#error_kind_name::#variant_name(ref err) => ::std::fmt::Display::fmt(err, f),
						},
					}
				},

				LinkType::Custom => {
					let variant_name = &link.variant.ident;
					match link.custom_display {
						Some(ref custom_display) => {
							let pattern = fields_pattern(&link.variant);
							let args = args(&link.variant);
							quote! {
								#error_kind_name::#variant_name #pattern => #custom_display(f, #args),
							}
						},

						None => {
							let pattern = fields_pattern_ignore(&link.variant);
							quote! {
								#error_kind_name::#variant_name #pattern => ::std::fmt::Display::fmt(self.description(), f),
							}
						},
					}
				},
			});

			let error_kind_from_impls = links.iter().map(|link| match link.link_type {
				LinkType::Chainable(_, ref error_kind_ty) => {
					let variant_name = &link.variant.ident;
					Some(quote! {
						impl From<#error_kind_ty> for #error_kind_name {
							fn from(kind: #error_kind_ty) -> Self {
								#error_kind_name::#variant_name(kind)
							}
						}
					})
				},

				LinkType::Foreign(_) |
				LinkType::Custom => None,
			});

			let error_doc_comment = format!(r"The Error type.

This struct is made of three things:

- `{0}` which is used to determine the type of the error.
- a backtrace, generated when the error is created.
- an error chain, used for the implementation of `Error::cause()`.", error_kind_name);

			let error_cause_cases = links.iter().filter_map(|link| match link.link_type {
				LinkType::Foreign(_) => {
					let variant_name = &link.variant.ident;
					Some(quote! {
						#error_kind_name::#variant_name(ref err) => err.cause(),
					})
				},

				LinkType::Chainable(_, _) |
				LinkType::Custom => None,
			});

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
						impl From<#error_ty> for #error_name {
							fn from(err: #error_ty) -> Self {
								#error_name(#error_kind_name::#variant_name(err.0), err.1)
							}
						}
					})
				},

				LinkType::Foreign(ref ty) => {
					let variant_name = &link.variant.ident;
					Some(quote! {
						impl From<#ty> for #error_name {
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

			let result_wrapper = result_name.map(|result_name| quote! {
				/// Convenient wrapper around `std::Result`.
				pub type #result_name<T> = ::std::result::Result<T, #error_name>;
			});

			quote! {
				extern crate error_chain as #error_chain_name;

				impl #error_kind_name {
					/// A string describing the error kind.
					pub fn description(&self) -> &str {
						match *self {
							#error_kind_name::Msg(ref s) => s,

							#(#error_kind_description_cases)*
						}
					}
				}

				impl ::std::fmt::Display for #error_kind_name {
					fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
						match *self {
							#error_kind_name::Msg(ref s) => ::std::fmt::Display::fmt(s, f),

							#(#error_kind_display_cases)*
						}
					}
				}

				#(#error_kind_from_impls)*

				impl <'a> From<&'a str> for #error_kind_name {
					fn from(s: &'a str) -> Self { #error_kind_name::Msg(s.to_string()) }
				}

				impl From<String> for #error_kind_name {
					fn from(s: String) -> Self { #error_kind_name::Msg(s) }
				}

				impl From<#error_name> for #error_kind_name {
					fn from(err: #error_name) -> Self { err.0 }
				}

				#[doc = #error_doc_comment]
				#[derive(Debug)]
				pub struct #error_name(
					/// The kind of the error.
					pub #error_kind_name,

					/// Contains the error chain and the backtrace.
					pub #error_chain_name::State,
				);

				#[allow(unused)]
				impl #error_name {
					/// Constructs an error from a kind, and generates a backtrace.
					pub fn from_kind(kind: #error_kind_name) -> #error_name {
						#error_name(kind, #error_chain_name::State::default())
					}

					/// Returns the kind of the error.
					pub fn kind(&self) -> &#error_kind_name { &self.0 }

					/// Iterates over the error chain.
					pub fn iter(&self) -> #error_chain_name::ErrorChainIter {
						#error_chain_name::ChainedError::iter(self)
					}

					/// Returns the backtrace associated with this error.
					pub fn backtrace(&self) -> Option<&#error_chain_name::Backtrace> {
						self.1.backtrace()
					}
				}

				impl ::std::error::Error for #error_name {
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

				impl ::std::fmt::Display for #error_name {
					fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
						::std::fmt::Display::fmt(&self.0, f)
					}
				}

				#(#error_from_impls)*

				impl From<#error_kind_name> for #error_name {
					fn from(kind: #error_kind_name) -> Self { Self::from_kind(kind) }
				}

				impl <'a> From<&'a str> for #error_name {
					fn from(s: &'a str) -> Self { Self::from_kind(s.into()) }
				}

				impl From<String> for #error_name {
					fn from(s: String) -> Self { Self::from_kind(s.into()) }
				}

				impl ::std::ops::Deref for #error_name {
					type Target = #error_kind_name;

					fn deref(&self) -> &Self::Target { &self.0 }
				}

				impl #error_chain_name::ChainedError for #error_name {
					type ErrorKind = #error_kind_name;

					fn new(kind: Self::ErrorKind, state: #error_chain_name::State) -> Self {
						#error_name(kind, state)
					}

					fn from_kind(kind: Self::ErrorKind) -> Self {
						Self::from_kind(kind)
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
				pub trait #result_ext_name<T, E> {
					/// If the `Result` is an `Err` then `chain_err` evaluates the closure,
					/// which returns *some type that can be converted to `ErrorKind`*,
					/// boxes the original error to store as the cause, then returns a new error
					/// containing the original error.
					fn chain_err<F, EK>(self, callback: F) -> ::std::result::Result<T, Error>
						where F: FnOnce() -> EK, EK: Into<ErrorKind>;
				}

				impl<T, E> #result_ext_name<T, E> for ::std::result::Result<T, E>
					where E: ::std::error::Error + Send + 'static {
					fn chain_err<F, EK>(self, callback: F) -> ::std::result::Result<T, Error>
						where F: FnOnce() -> EK, EK: Into<ErrorKind> {
						self.map_err(move |e| {
							let state = #error_chain_name::State::new::<Error>(Box::new(e));
							#error_chain_name::ChainedError::new(callback().into(), state)
						})
					}
				}

				#result_wrapper
			}
		},

		_ => panic!("#[derive(error_chain)] can only be used with enums."),
	};

	result.to_string().parse().unwrap()
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
			quote!((#(#fields),*))
		},

		syn::VariantData::Tuple(ref fields) => {
			let fields = fields.iter().enumerate().map(|(i, _)| {
				let field_name = syn::parse_ident(&format!("value{}", i)).unwrap();
				quote!(#field_name)
			});
			quote!((#(#fields),*))
		},

		syn::VariantData::Unit => quote!(),
	}
}
