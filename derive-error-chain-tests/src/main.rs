#![allow(dead_code)]

//! Test crate for derive-error-chain. If it runs, it's tested.

#![cfg_attr(feature = "cargo-clippy", deny(clippy, clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", allow(
	missing_docs_in_private_items,
	option_unwrap_used,
	similar_names,
	use_debug,
))]

#[macro_use]
extern crate derive_error_chain;
#[macro_use]
extern crate error_chain;

fn main() {
	smoke_test_1();
	smoke_test_2();
	smoke_test_4();
	smoke_test_8();
	chain_err();
	links();

	foreign_link_test::display_underlying_error();
	foreign_link_test::finds_cause();
	foreign_link_test::iterates();

	with_result();
	without_result();
	documentation();
	rustup_regression();
	error_patterns();
	rewrapping();

	public_api_test();
	cause();
	inlined_description_and_display_and_cause();
	test_without_msg_1();
	test_without_msg_2();
	macro_conflicts();
}

// Upstream tests

fn smoke_test_1() {
	#[derive(Debug, ErrorChain)]
	#[error_chain(error = "Error", result_ext = "ResultExt", result = "Result")]
	pub enum ErrorKind {
		Msg(String),
	}
}

fn smoke_test_2() {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),
	}
}

#[cfg_attr(feature = "cargo-clippy", allow(items_after_statements))]
fn smoke_test_4() {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom, description = "http_status_description", display = "http_status_display")]
		HttpStatus(u32),
	}

	let err: Error = ErrorKind::HttpStatus(5).into();
	assert_eq!("http request returned an unsuccessful status code", ::std::error::Error::description(&err));
	assert_eq!("http request returned an unsuccessful status code: 5".to_string(), format!("{}", err));

	fn http_status_description(_: &u32) -> &str {
		"http request returned an unsuccessful status code"
	}

	fn http_status_display(f: &mut ::std::fmt::Formatter, e: &u32) -> ::std::fmt::Result {
		write!(f, "http request returned an unsuccessful status code: {}", e)
	}
}

fn smoke_test_8() {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		FileNotFound,

		#[error_chain(custom)]
		AccessDenied,
	}
}

fn chain_err() {
	use std::fmt;

	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		Test,
	}

	let _: Result<()> = Err(fmt::Error).chain_err(|| "");
	let _: Result<()> = Err(Error::from_kind(ErrorKind::Test)).chain_err(|| "");
}

fn links() {
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
		Test(test::ErrorKind),
	}
}

mod foreign_link_test {
	use std::fmt;

	#[derive(Debug)]
	pub struct ForeignError {
		cause: ForeignErrorCause,
	}

	impl ::std::error::Error for ForeignError {
		fn description(&self) -> &'static str {
			"Foreign error description"
		}

		fn cause(&self) -> Option<&::std::error::Error> { Some(&self.cause) }
	}

	impl fmt::Display for ForeignError {
		fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
			write!(formatter, "Foreign error display")
		}
	}

	#[derive(Debug)]
	pub struct ForeignErrorCause { }

	impl ::std::error::Error for ForeignErrorCause {
		fn description(&self) -> &'static str {
			"Foreign error cause description"
		}

		fn cause(&self) -> Option<&::std::error::Error> { None }
	}

	impl fmt::Display for ForeignErrorCause {
		fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
			write!(formatter, "Foreign error cause display")
		}
	}

	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(foreign)]
		Foreign(ForeignError),

		#[error_chain(foreign)]
		Io(::std::io::Error),
	}

	pub fn display_underlying_error() {
		let chained_error = try_foreign_error().err().unwrap();
		assert_eq!(
			format!("{}", ForeignError { cause: ForeignErrorCause { } }),
			format!("{}", chained_error)
		);
	}

	pub fn finds_cause() {
		let chained_error = try_foreign_error().err().unwrap();
		assert_eq!(
			format!("{}", ForeignErrorCause { }),
			format!("{}", ::std::error::Error::cause(&chained_error).unwrap())
		);
	}

	pub fn iterates() {
		let chained_error = try_foreign_error().err().unwrap();
		let mut error_iter = chained_error.iter();
		assert_eq!(
			format!("{}", ForeignError { cause: ForeignErrorCause { } }),
			format!("{}", error_iter.next().unwrap())
		);
		assert_eq!(
			format!("{}", ForeignErrorCause { }),
			format!("{}", error_iter.next().unwrap())
		);
		assert_eq!(
			format!("{:?}", None as Option<&::std::error::Error>),
			format!("{:?}", error_iter.next())
		);
	}

	fn try_foreign_error() -> Result<()> {
		try!(Err(ForeignError {
			cause: ForeignErrorCause { }
		}));
		Ok(())
	}
}

mod attributes_test {
	#[allow(unused_imports)]
	use std::io;

	#[cfg(foo)]
	mod inner {
		#[derive(Debug, ErrorChain)]
		pub enum ErrorKind {
			Msg(String),
		}
	}

	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[cfg(foo)]
		#[error_chain(link = "inner::Error")]
		Inner(inner::ErrorKind),

		#[cfg(foo)]
		#[error_chain(foreign)]
		Io(io::Error),

		#[cfg(foo)]
		#[error_chain(custom)]
		AnError,
	}
}

fn with_result() {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),
	}

	let _: Result<()> = Ok(());
}

fn without_result() {
	#[derive(Debug, ErrorChain)]
	#[error_chain(result = "")]
	pub enum ErrorKind {
		Msg(String),
	}

	let _: Result<(), ()> = Ok(());
}

fn documentation() {
	mod inner {
		#[derive(Debug, ErrorChain)]
		pub enum ErrorKind {
			Msg(String),
		}
	}

	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		/// Doc
		#[error_chain(link = "inner::Error")]
		Inner(inner::ErrorKind),

		/// Doc
		#[error_chain(foreign)]
		Io(::std::io::Error),

		/// Doc
		#[error_chain(custom)]
		Variant,
	}
}

mod multiple_error_same_mod {
	#[derive(Debug, ErrorChain)]
	#[error_chain(error = "MyError", result_ext = "MyResultExt", result = "MyResult")]
	pub enum MyErrorKind {
		Msg(String),
	}

	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),
	}
}

#[deny(dead_code)]
mod allow_dead_code {
	#[derive(Debug, ErrorChain)]
	#[error_chain(result = "")]
	pub enum ErrorKind {
		Msg(String),
	}
}

// Make sure links actually work!
fn rustup_regression() {
	mod mock {
		#[derive(Debug, ErrorChain)]
		pub enum ErrorKind {
			Msg(String),
		}
	}

	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(link = "mock::Error")]
		Download(mock::ErrorKind),

		#[error_chain(custom)]
		#[error_chain(description = r#"|| "could not locate working directory""#)]
		LocatingWorkingDir,
	}
}

fn error_patterns() {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),
	}

	// Tuples look nice when matching errors
	match Error::from("Test") {
		Error(ErrorKind::Msg(_), _) => {
		}
	}
}

#[cfg_attr(feature = "cargo-clippy", allow(non_ascii_literal))]
fn rewrapping() {
	use std::env::VarError::{self, NotPresent, NotUnicode};

	#[derive(Debug, ErrorChain)]
	#[error_chain(error = "MyError", result_ext = "MyResultExt", result = "MyResult")]
	pub enum MyErrorKind {
		Msg(String),

		#[error_chain(foreign)]
		VarErr(VarError),
	}

	let result_a_from_func: Result<String, _> = Err(VarError::NotPresent);
	let result_b_from_func: Result<String, _> = Err(VarError::NotPresent);

	let our_error_a = result_a_from_func.map_err(|e| match e {
		NotPresent => MyError::with_chain(e, "env var wasn't provided"),
		NotUnicode(_) => MyError::with_chain(e, "env var was borkæ–‡å­—åŒ–ã"),
	});

	let our_error_b = result_b_from_func.or_else(|e| match e {
		NotPresent => Err(e).chain_err(|| "env var wasn't provided"),
		NotUnicode(_) => Err(e).chain_err(|| "env var was borkæ–‡å­—åŒ–ã"),
	});

	assert_eq!(
		format!("{}", our_error_a.unwrap_err()),
		format!("{}", our_error_b.unwrap_err())
	);
}

// Own tests

mod test2 {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		HttpStatus(u32),
	}
}

fn public_api_test() {
	use test2::{ Error, ErrorKind, ResultExt, Result };

	let err: Error = ErrorKind::HttpStatus(5).into();
	let result: Result<()> = Err(err);

	let _: Result<()> = result.chain_err(|| "An HTTP error occurred");
}

fn cause() {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		#[error_chain(cause = "file_io_error_cause")]
		FileIO(::std::path::PathBuf, ::std::io::Error),
	}

	fn file_io_error_cause<'a>(_: &::std::path::Path, err: &'a ::std::io::Error) -> &'a ::std::error::Error {
		err
	}

	let err: Error = ErrorKind::FileIO(::std::path::PathBuf::new(), ::std::io::Error::from_raw_os_error(1)).into();
	assert!(::std::error::Error::cause(&err).is_some());
}

fn inlined_description_and_display_and_cause() {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		#[error_chain(description = r#"|_| "http request returned an unsuccessful status code""#)]
		#[error_chain(display = r#"|e| write!(f, "http request returned an unsuccessful status code: {}", e)"#)]
		HttpStatus(u32),

		#[error_chain(custom)]
		#[error_chain(cause = "|_, err| err")]
		FileIO(::std::path::PathBuf, ::std::io::Error),
	}

	let err: Error = ErrorKind::HttpStatus(5).into();
	assert_eq!("http request returned an unsuccessful status code", ::std::error::Error::description(&err));
	assert_eq!("http request returned an unsuccessful status code: 5".to_string(), format!("{}", err));

	let err: Error = ErrorKind::FileIO(::std::path::PathBuf::new(), ::std::io::Error::from_raw_os_error(1)).into();
	assert!(::std::error::Error::cause(&err).is_some());
}

mod generics_test {
	use std::{error, fmt, io};

	mod inner1 {
		use std::fmt;
		#[derive(Debug, ErrorChain)]
		pub enum ErrorKind<T: Send + fmt::Debug + 'static> {
			Msg(String),

			#[error_chain(custom)]
			CustomGeneric(T)
		}
	}

	mod inner2 {
		#[derive(Debug, ErrorChain)]
		pub enum ErrorKind {
			Msg(String),
		}
	}

	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind<T: error::Error + Send + 'static, U>
		where U: Send + fmt::Debug + fmt::Display + 'static {
		Msg(String),

		#[error_chain(custom)]
		#[error_chain(description = r#"|_| "custom error""#)]
		#[error_chain(display = r#"|t| write!(f, "custom error: {}", t)"#)]
		Custom(String),

		#[error_chain(custom)]
		#[error_chain(description = r#"|_| "custom generic error""#)]
		#[error_chain(display = r#"|t| write!(f, "custom generic error: {}", t)"#)]
		CustomGeneric(T),

		#[error_chain(custom)]
		#[error_chain(description = r#"|_| "custom generic boxed error""#)]
		#[error_chain(display = r#"|t| write!(f, "custom generic boxed error: {}", t)"#)]
		CustomGenericBoxed(Box<U>),

		#[error_chain(link = "inner1::Error<U>")]
		LinkGeneric(inner1::ErrorKind<U>),

		#[error_chain(link = "inner2::Error")]
		Link(inner2::ErrorKind),

		#[error_chain(foreign)]
		ForeignGeneric(T),

		#[error_chain(foreign)]
		ForeignGenericBoxed(Box<T>),

		#[error_chain(foreign)]
		Foreign(io::Error),
	}
}

fn test_without_msg_1() {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		#[error_chain(custom)]
		HttpStatus(u32),
	}
}

fn test_without_msg_2() {
	#[derive(Debug, ErrorChain)]
	pub enum ErrorKind {
		#[error_chain(custom)]
		HttpStatus(u32),
	}

	// Should not conflict
	#[cfg_attr(feature = "cargo-clippy", allow(fallible_impl_from))]
	impl<'a> From<&'a str> for ErrorKind {
		fn from(_: &'a str) -> Self {
			unimplemented!()
		}
	}

	#[cfg_attr(feature = "cargo-clippy", allow(fallible_impl_from))]
	impl From<String> for ErrorKind {
		fn from(_: String) -> Self {
			unimplemented!()
		}
	}
}

fn macro_conflicts() {
	#[derive(Debug, ErrorChain)]
	#[error_chain(result = "MyResult")]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		Code(i32),
	}

	error_chain! {
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

mod need_not_be_pub {
	#[derive(Debug, ErrorChain)]
	enum ErrorKind {
		Msg(String),
	}
}
