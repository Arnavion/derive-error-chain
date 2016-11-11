#![crate_type = "proc-macro"]
#![feature(proc_macro, proc_macro_lib, slice_patterns)]
#![recursion_limit = "300"]

//! A Macros 1.1 implementation of https://crates.io/crates/error-chain

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

#[proc_macro_derive(error_chain)]
pub fn derive_error_chain(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let source = input.to_string();
	let ast = syn::parse_macro_input(&source).unwrap();
	let error_kind_name = ast.ident;

	let mut error_kind_attrs = vec![];
	let mut error_name = syn::Ident::from("Error");
	let mut result_name = syn::Ident::from("Result");

	for attr in ast.attrs {
		let mut suppress_attr = false;

		match &attr.value {
			&syn::MetaItem::List(ref ident, ref nested_meta_items) if ident.to_string() == "error_chain" => {
				suppress_attr = true;

				for nested_meta_item in nested_meta_items {
					if let syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref ident, syn::Lit::Str(ref value, _))) = *nested_meta_item {
						let ident = ident.to_string();
						if ident == "error" {
							error_name = syn::Ident::from(value.clone());
						}
						else if ident == "result" {
							result_name = syn::Ident::from(value.clone());
						}
					}
				}
			},

			_ => { },
		}

		if !suppress_attr {
			error_kind_attrs.push(attr);
		}
	}

	let error_chain_iter_name = syn::Ident::from(error_name.to_string() + "ChainIter");
	let make_backtrace_name = syn::Ident::from(error_name.to_string() + "_make_backtrace");

	struct Link {
		variant: syn::Variant,
		link_type: LinkType,
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
				let mut attrs = vec![];
				let mut link_type = LinkType::Chainable;

				for attr in variant.attrs {
					let mut suppress_attr = false;

					if let syn::MetaItem::List(ref ident, ref nested_meta_items) = attr.value {
						if ident.to_string() == "error_chain" {
							suppress_attr = true;

							if nested_meta_items.len() == 1 {
								match nested_meta_items[0] {
									syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref ident)) if ident.to_string() == "foreign" => {
										link_type = LinkType::Foreign;
									},

									syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref ident)) if ident.to_string() == "custom" => {
										link_type = LinkType::Custom;
									},

									_ => { },
								}
							}
						}
					}

					if !suppress_attr {
						attrs.push(attr);
					}
				}

				let variant = syn::Variant { attrs: attrs, .. variant };

				links.push(Link { variant: variant, link_type: link_type });
			}

			let variants = links.iter().map(|link| &link.variant);

			let error_kind_description_cases = links.iter().map(|link| match link.link_type {
				LinkType::Chainable => {
					let variant_name = &link.variant.ident;
					quote! {
						#error_kind_name::#variant_name(ref err) => err.description(),
					}
				},

				LinkType::Foreign => {
					let variant_name = &link.variant.ident;
					quote! {
						#error_kind_name::#variant_name(ref err) => ::std::error::Error::description(err),
					}
				},

				LinkType::Custom => {
					let variant_name = &link.variant.ident;
					let fields_match = pattern(&link.variant);
					quote! {
						#error_kind_name::#variant_name #fields_match => stringify!(#variant_name),
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
				#(#error_kind_attrs)*
				pub enum #error_kind_name {
					/// A generic error
					Msg(String),

					#(#variants,)*
				}

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
						write!(f, "{}", self.description())
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

fn pattern(variant: &syn::Variant) -> quote::Tokens {
	match variant.data {
		syn::VariantData::Struct(_) => quote!({ .. }),

		syn::VariantData::Tuple(_) => quote!((..)),

/*
		syn::VariantData::Tuple(ref fields) => {
			let fields = fields.iter().map(|_| quote!(_));
			quote!((#(#fields,)*))
		},
*/
		syn::VariantData::Unit => quote!(),
	}
}
