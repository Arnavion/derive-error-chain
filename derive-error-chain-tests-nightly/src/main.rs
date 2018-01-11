#![allow(dead_code)]
#![feature(proc_macro)]

//! Test crate for derive-error-chain. If it runs, it's tested.

#![cfg_attr(feature = "cargo-clippy", deny(clippy, clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", allow(
	missing_docs_in_private_items,
))]

#[macro_use]
extern crate derive_error_chain;
extern crate error_chain;

fn main() {
	macro_conflicts_use();
	macro_conflicts_fully_qualified();
	lambda_description_and_display_and_cause();
	const_format_string_tuple_variants();
	const_format_string_struct_variants();
}

fn macro_conflicts_use() {
	use error_chain::{ bail, error_chain as error_chain_macro, error_chain_processing, impl_error_chain_kind, impl_error_chain_processed, impl_extract_backtrace, quick_main };

	#[derive(Debug, ErrorChain)]
	#[error_chain(result = "MyResult")]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		Code(i32),
	}

	error_chain_macro! {
		types { ECError, ECErrorKind, ECResultExt, ECResult; }
	}

	quick_main!(|| -> MyResult<()> {
		bail!("failed")
	});

	fn foo() -> MyResult<()> {
		bail!("failed")
	}

	match foo() {
		Ok(_) => unreachable!(),
		Err(err) => match *err.kind() {
			ErrorKind::Msg(ref s) if s == "failed" => (),
			_ => unreachable!(),
		},
	}
}

fn macro_conflicts_fully_qualified() {
	use error_chain::{ error_chain_processing, impl_error_chain_kind, impl_error_chain_processed, impl_extract_backtrace };

	#[derive(Debug, ErrorChain)]
	#[error_chain(result = "MyResult")]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		Code(i32),
	}

	error_chain::error_chain! {
		types { ECError, ECErrorKind, ECResultExt, ECResult; }
	}

	error_chain::quick_main!(|| -> MyResult<()> {
		error_chain::bail!("failed")
	});

	fn foo() -> MyResult<()> {
		error_chain::bail!("failed")
	}

	match foo() {
		Ok(_) => unreachable!(),
		Err(err) => match *err.kind() {
			ErrorKind::Msg(ref s) if s == "failed" => (),
			_ => unreachable!(),
		},
	}
}

fn lambda_description_and_display_and_cause() {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		#[error_chain(description = |_| "http request returned an unsuccessful status code")]
		#[error_chain(display = |e| write!(f, "http request returned an unsuccessful status code: {}", e))]
		HttpStatus(u32),

		#[error_chain(custom)]
		#[error_chain(cause = |_, err| err)]
		FileIO(::std::path::PathBuf, ::std::io::Error),
	}

	let err: Error = ErrorKind::HttpStatus(5).into();
	assert_eq!("http request returned an unsuccessful status code", ::std::error::Error::description(&err));
	assert_eq!("http request returned an unsuccessful status code: 5".to_string(), format!("{}", err));

	let err: Error = ErrorKind::FileIO(::std::path::PathBuf::new(), ::std::io::Error::from_raw_os_error(1)).into();
	assert!(::std::error::Error::cause(&err).is_some());
}

fn const_format_string_tuple_variants() {
	mod test {
		#[derive(Debug, ErrorChain)]
		pub enum ErrorKind {
			Msg(String),
		}
	}

	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(link = "test::Error")]
		#[error_chain(description = const("Chainable's description"))]
		#[error_chain(display = const("Chainable's display: {0}"))]
		Chainable(test::ErrorKind),

		#[error_chain(foreign)]
		#[error_chain(description = const("Foreign's description"))]
		#[error_chain(display = const("Foreign's display: {0}"))]
		Foreign(::std::io::Error),

		#[error_chain(custom)]
		#[error_chain(description = const("Custom's description"))]
		#[error_chain(display = const("Custom's display: {0}"))]
		Custom(u32, u32),
	}

	let err: test::Error = "foo".into();
	let err: Error = err.into();
	assert_eq!("Chainable's description", ::std::error::Error::description(&err));
	assert_eq!("Chainable's display: foo".to_string(), format!("{}", err));

	let err: Error = ::std::io::Error::new(std::io::ErrorKind::NotFound, "abcde".to_string()).into();
	assert_eq!("Foreign's description", ::std::error::Error::description(&err));
	assert_eq!("Foreign's display: abcde".to_string(), format!("{}", err));

	let err: Error = ErrorKind::Custom(5, 6).into();
	assert_eq!("Custom's description", ::std::error::Error::description(&err));
	assert_eq!("Custom's display: 5".to_string(), format!("{}", err));
}

fn const_format_string_struct_variants() {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		#[error_chain(description = const("Custom's description"))]
		#[error_chain(display = const("Custom's display: {code}"))]
		Custom { code: u32, extra: u32, },
	}

	let err: Error = (ErrorKind::Custom { code: 5, extra: 6, }).into();
	assert_eq!("Custom's description", ::std::error::Error::description(&err));
	assert_eq!("Custom's display: 5".to_string(), format!("{}", err));
}
