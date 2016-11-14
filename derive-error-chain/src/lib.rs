#![crate_type = "proc-macro"]
#![feature(proc_macro, proc_macro_lib)]
#![recursion_limit = "300"]

//! A Macros 1.1 implementation of https://crates.io/crates/error-chain
//!
//! The error-chain example
//!
//! ```ignore
//! error_chain {
//!     types { Error, ErrorKind, ChainErr, Result; }
//!
//!     links {
//!     	rustup_dist::Error, rustup_dist::ErrorKind, Dist;
//!     	rustup_utils::Error, rustup_utils::ErrorKind, Utils;
//!     }
//!
//!     foreign_links {
//!     	temp::Error, Temp;
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
//! #[derive(Debug, error_chain)]
//! // This attribute is optional if using the default names "Error", "Result" and "ChainErr"
//! #[error_chain(error = "Error", result = "Result", chain_err = "ChainErr")]
//! pub enum ErrorKind {
//!     Msg(String), // A special variant that must always be present.
//!
//!     Dist(rustup_dist::Error, rustup_dist::ErrorKind),
//!
//!     Utils(rustup_utils::Error, rustup_utils::ErrorKind),
//!
//!     #[error_chain(foreign)]
//!     Temp(temp::Error),
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
//! - This library requires the nightly compiler to be able to use the `proc_macro` and `conservative_impl_trait` rust features.
//! - The macro output can be used with `#[deny(missing_docs)]` since it allows doc comments on the ErrorKind variants.
//! - The macro output uses `::backtrace::Backtrace` unlike error-chain which uses `$crate::Backtrace`. Thus you need to link to `backtrace` in your own crate.
//! - The enum must always have a special `Msg(String)` member. Unlike error_chain which adds this member implicitly, this macro requires it explicitly.

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

#[proc_macro_derive(error_chain, attributes(error_chain))]
pub fn derive_error_chain(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let source = input.to_string();
	let ast = syn::parse_macro_input(&source).unwrap();
	let error_kind_name = ast.ident;

	let mut error_name = syn::Ident::from("Error");
	let mut result_name = syn::Ident::from("Result");
	let mut chain_err_name = syn::Ident::from("ChainErr");

	for attr in ast.attrs {
		match &attr.value {
			&syn::MetaItem::List(ref ident, ref nested_meta_items) if ident == "error_chain" => {
				for nested_meta_item in nested_meta_items {
					if let syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref ident, syn::Lit::Str(ref value, _))) = *nested_meta_item {
						if ident == "error" {
							error_name = syn::Ident::from(value.clone());
						}
						else if ident == "result" {
							result_name = syn::Ident::from(value.clone());
						}
						else if ident == "chain_err" {
							chain_err_name = syn::Ident::from(value.clone());
						}
					}
				}
			},

			_ => { },
		}
	}

	let error_chain_iter_name = syn::Ident::from(error_name.to_string() + "ChainIter");
	let make_backtrace_name = syn::Ident::from(error_name.to_string() + "_make_backtrace");

	struct Link {
		variant: syn::Variant,
		link_type: LinkType,
		custom_description: Option<syn::Path>,
		custom_display: Option<syn::Path>,
	}

	enum LinkType {
		Chainable,
		Foreign,
		Custom,
	}

	let result = match ast.body {
		syn::Body::Enum(variants) => {
			let mut links = vec![];

			for variant in variants {
				if &variant.ident == "Msg" {
					continue;
				}

				let mut link_type = LinkType::Chainable;
				let mut custom_description = None;
				let mut custom_display = None;

				for attr in &variant.attrs {
					if let syn::MetaItem::List(ref ident, ref nested_meta_items) = attr.value {
						if ident == "error_chain" {
							for nested_meta_item in nested_meta_items {
								match *nested_meta_item {
									syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref ident)) => {
										if ident == "foreign" {
											link_type = LinkType::Foreign;
										}
										else if ident == "custom" {
											link_type = LinkType::Custom;
										}
									},

									syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref ident, syn::Lit::Str(ref value, _))) => {
										if ident == "description" {
											custom_description = Some(syn::Path::from(value.clone()));
										}
										else if ident == "display" {
											custom_display = Some(syn::Path::from(value.clone()));
										}
									},

									_ => { },
								}
							}
						}
					}
				}

				links.push(Link {
					variant: variant,
					link_type: link_type,
					custom_description: custom_description,
					custom_display: custom_display,
				});
			}

			let error_kind_description_cases = links.iter().map(|link| match link.link_type {
				LinkType::Chainable | LinkType::Foreign => {
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
				LinkType::Chainable | LinkType::Foreign => {
					let variant_name = &link.variant.ident;
					match link.custom_display {
						Some(ref custom_display) => quote! {
							#error_kind_name::#variant_name(ref err) => #custom_display(f, err),
						},

						None => quote! {
							#error_kind_name::#variant_name(ref err) => write!(f, "{}", err),
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
								#error_kind_name::#variant_name #pattern => write!(f, "{}", self.description()),
							}
						},
					}
				},
			});

			let error_kind_from_impls = links.iter().map(|link| match link.link_type {
				LinkType::Chainable => {
					let variant_name = &link.variant.ident;
					match &link.variant.data {
						&syn::VariantData::Tuple(ref fields) if fields.len() == 2 => {
							let kind_ty = &fields[1].ty;
							Some(quote! {
								impl From<#kind_ty> for #error_kind_name {
									fn from(err: #kind_ty) -> Self {
										#error_kind_name::#variant_name(err)
									}
								}
							})
						},

						_ => panic!("Chainable link {} must be a tuple of two elements (the chainable error type and the chainable errorkind type).", variant_name),
					}
				},

				LinkType::Foreign => None,

				LinkType::Custom => None,
			});

			let error_cause_cases = links.iter().filter_map(|link| match link.link_type {
				LinkType::Chainable => None,

				LinkType::Foreign => {
					let variant_name = &link.variant.ident;
					Some(quote! {
						#error_kind_name::#variant_name(ref err) => err.cause(),
					})
				},

				LinkType::Custom => None,
			});

			let error_from_impls = links.iter().filter_map(|link| match link.link_type {
				LinkType::Chainable => {
					let variant_name = &link.variant.ident;
					match &link.variant.data {
						&syn::VariantData::Tuple(ref fields) if fields.len() == 2 => {
							let ty = &fields[0].ty;
							Some(quote! {
								impl From<#ty> for #error_name {
									fn from(err: #ty) -> Self {
										#error_name(#error_kind_name::#variant_name(err.0), err.1)
									}
								}
							})
						},

						_ => panic!("Chainable link {} must be a tuple of two elements (the chainable error type and the chainable errorkind type).", variant_name),
					}
				},

				LinkType::Foreign => {
					let variant_name = &link.variant.ident;
					match &link.variant.data {
						&syn::VariantData::Tuple(ref fields) if fields.len() == 1 => {
							let ty = &fields[0].ty;
							Some(quote! {
								impl From<#ty> for #error_name {
									fn from(err: #ty) -> Self {
										#error_name(#error_kind_name::#variant_name(err), (None, #make_backtrace_name()))
									}
								}
							})
						},

						_ => panic!("Foreign link {} must be a tuple of one element (the foreign error type).", variant_name),
					}
				},

				LinkType::Custom => None,
			});

			quote! {
				impl #error_kind_name {
					/// Returns the description of this error kind.
					pub fn description(&self) -> &str {
						match *self {
							#error_kind_name::Msg(ref s) => { &s },

							#(#error_kind_description_cases)*
						}
					}
				}

				impl ::std::fmt::Display for #error_kind_name {
					fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
						match *self {
							#error_kind_name::Msg(ref s) => write!(f, "{}", s),

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

				/// Error type.
				#[derive(Debug)]
				pub struct #error_name(
					pub #error_kind_name,
					pub (
						Option<Box<::std::error::Error + Send>>,
						Option<::std::sync::Arc<::backtrace::Backtrace>>));

				#[allow(unused)]
				impl #error_name {
					/// Returns the kind of this error.
					pub fn kind(&self) -> &#error_kind_name { &self.0 }

					/// Converts this error into its kind.
					pub fn into_kind(self) -> #error_kind_name { self.0 }

					/// Constructs an iterator over the chained errors in this error.
					pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a ::std::error::Error> + 'a {
						#error_chain_iter_name(Some(self))
					}

					/// Gets the backtrace of this error.
					pub fn backtrace(&self) -> Option<&::backtrace::Backtrace> {
						(self.1).1.as_ref().map(|v| &**v)
					}
				}

				impl ::std::error::Error for #error_name {
					fn description(&self) -> &str { self.0.description() }

					fn cause(&self) -> Option<&::std::error::Error> {
						match (self.1).0 {
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
					fn from(e: #error_kind_name) -> Self { #error_name(e, (None, #make_backtrace_name())) }
				}

				impl <'a> From<&'a str> for #error_name {
					fn from(s: &'a str) -> Self { #error_name(s.into(), (None, #make_backtrace_name())) }
				}

				impl From<String> for #error_name {
					fn from(s: String) -> Self { #error_name(s.into(), (None, #make_backtrace_name())) }
				}

				/// Result type.
				pub type #result_name<T> = ::std::result::Result<T, #error_name>;

				/// ChainErr trait.
				pub trait #chain_err_name<T> {
					fn chain_err<F, EK>(self, callback: F) -> ::std::result::Result<T, #error_name> where F: FnOnce() -> EK, EK: Into<#error_kind_name>;
				}

				impl <T, E> #chain_err_name<T> for ::std::result::Result<T, E> where E: ::std::error::Error + Send + 'static {
					fn chain_err<F, EK>(self, callback: F) -> ::std::result::Result<T, Error> where F: FnOnce() -> EK, EK: Into<#error_kind_name> {
						self.map_err(move |err| {
							let err = Box::new(err) as Box<::std::error::Error+ Send + 'static>;

							let (err, backtrace) = match err.downcast::<#error_name>() {
								Ok(err) => {
									let backtrace = Some((err.1).1.clone());
									(err as Box<::std::error::Error + Send + 'static>, backtrace)
								},

								Err(err) => (err, None),
							};

							let backtrace = backtrace.unwrap_or_else(#make_backtrace_name);

							#error_name(callback().into(), (Some(err), backtrace))
						})
					}
				}

				#[allow(non_snake_case)]
				fn #make_backtrace_name() -> Option<::std::sync::Arc<::backtrace::Backtrace>> {
					match ::std::env::var_os("RUST_BACKTRACE") {
						Some(ref val) if val != "0" => Some(::std::sync::Arc::new(::backtrace::Backtrace::new())),
						_ => None
					}
				}

				struct #error_chain_iter_name<'a>(pub Option<&'a ::std::error::Error>);

				impl<'a> Iterator for #error_chain_iter_name<'a> {
					type Item = &'a ::std::error::Error;

					fn next<'b>(&'b mut self) -> Option<&'a ::std::error::Error> {
						match self.0.take() {
							Some(err) => {
								self.0 = err.cause();
								Some(err)
							},

							None => None,
						}
					}
				}
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
				let field_name = syn::Ident::from(format!("value{}", i));
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
				let field_name = syn::Ident::from(format!("value{}", i));
				quote!(#field_name)
			});
			quote!((#(#fields),*))
		},

		syn::VariantData::Unit => quote!(),
	}
}
