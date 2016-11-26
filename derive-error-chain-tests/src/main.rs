#![allow(dead_code)]
#![feature(conservative_impl_trait, proc_macro)]

//! Test crate for derive-error-chain. If it runs, it's tested.

extern crate backtrace;
#[macro_use]
extern crate derive_error_chain;

fn main() {
	smoke_test_1();
	smoke_test_2();
	smoke_test_4();
	smoke_test_8();

	has_backtrace_depending_on_env();
	chain_err();

	foreign_link_test::display_underlying_error();
	foreign_link_test::finds_cause();
	foreign_link_test::iterates();

	can_disable_backtrace();
	can_override_backtrace();
	public_api_test();
	inlined_description_and_display();
}

// Upstream tests

fn smoke_test_1() {
	#[derive(Debug, error_chain)]
	#[error_chain(error = "Error", result = "Result", chain_err = "ChainErr")]
	pub enum ErrorKind {
		Msg(String),
	}
}

fn smoke_test_2() {
	#[derive(Debug, error_chain)]
	pub enum ErrorKind {
		Msg(String),
	}
}

fn smoke_test_4() {
	#[derive(Debug, error_chain)]
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
	#[derive(Debug, error_chain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		FileNotFound,

		#[error_chain(custom)]
		AccessDenied,
	}
}

fn has_backtrace_depending_on_env() {
	use std::env;

	#[derive(Debug, error_chain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		MyError,
	}

	// missing RUST_BACKTRACE and RUST_BACKTRACE=0
	env::remove_var("RUST_BACKTRACE");
	let err = Error::from(ErrorKind::MyError);
	assert!(err.backtrace().is_none());
	env::set_var("RUST_BACKTRACE", "0");
	let err = Error::from(ErrorKind::MyError);
	assert!(err.backtrace().is_none());

	// RUST_BACKTRACE set to anything but 0
	env::set_var("RUST_BACKTRACE", "yes");
	let err = Error::from(ErrorKind::MyError);
	assert!(err.backtrace().is_some());
}

fn chain_err() {
	#[derive(Debug, error_chain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		HttpStatus(u32),
	}

	let err: Error = ErrorKind::HttpStatus(5).into();
	let result: Result<()> = Err(err);
	let result = result.chain_err(|| "An HTTP error occurred");
	let chained_error = result.err().unwrap();
	let mut error_iter = chained_error.iter();
	assert_eq!(
		"An HTTP error occurred".to_string(),
		format!("{}", error_iter.next().unwrap())
	);
	assert_eq!(
		format!("{}", ErrorKind::HttpStatus(5)),
		format!("{}", error_iter.next().unwrap())
	);
	assert_eq!(
		format!("{:?}", None as Option<&::std::error::Error>),
		format!("{:?}", error_iter.next())
	);
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
	pub struct ForeignErrorCause {}

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

	#[derive(Debug, error_chain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(foreign)]
		Foreign(ForeignError),
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

	mod inner {
		#[derive(Debug, error_chain)]
		pub enum ErrorKind {
			Msg(String),

			#[cfg(foo)]
			Inner(inner::Error, inner::ErrorKind),
		}
	}

	#[derive(Debug, error_chain)]
	#[error_chain(error = "Error", chain_err = "ErrorChain", result = "Result")]
	pub enum ErrorKind {
		Msg(String),

		#[cfg(foo)]
		Inner(inner::Error, inner::ErrorKind),

		#[cfg(foo)]
		#[error_chain(foreign)]
		Io(io::Error),

		#[cfg(foo)]
		#[error_chain(custom)]
		AnError,
	}
}

// Own tests

fn can_disable_backtrace() {
	#[derive(Debug, error_chain)]
	#[error_chain(backtrace = "")]
	pub enum ErrorKind {
		Msg(String),
	}

	let err: Error = ErrorKind::Msg("foo".to_string()).into();
	assert!(err.backtrace().is_none());
	assert_eq!(
		r#"Error(Msg("foo"), (None, None))"#,
		format!("{:?}", err)
	);
}

fn can_override_backtrace() {
	#[derive(Debug)]
	pub struct MyBacktrace;
	impl MyBacktrace {
		fn new() -> MyBacktrace { MyBacktrace }
	}

	#[derive(Debug, error_chain)]
	#[error_chain(backtrace = "MyBacktrace")]
	pub enum ErrorKind {
		Msg(String),
	}

	let err: Error = ErrorKind::Msg("foo".to_string()).into();
	let _: &MyBacktrace = err.backtrace().unwrap();
	assert_eq!(
		r#"Error(Msg("foo"), (None, Some(MyBacktrace)))"#,
		format!("{:?}", err)
	);
}

mod test {
	#[derive(Debug, error_chain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		HttpStatus(u32),
	}
}

fn public_api_test() {
	use test::{ Error, ErrorKind, ChainErr, Result };

	let err: Error = ErrorKind::HttpStatus(5).into();
	let result: Result<()> = Err(err);

	result.chain_err(|| "An HTTP error occurred").err().unwrap();
}

fn inlined_description_and_display() {
	#[derive(Debug, error_chain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(custom)]
		#[error_chain(description = r#"(|_| "http request returned an unsuccessful status code")"#)]
		#[error_chain(display = r#"(|f: &mut ::std::fmt::Formatter, e| write!(f, "http request returned an unsuccessful status code: {}", e))"#)]
		HttpStatus(u32),
	}

	let err: Error = ErrorKind::HttpStatus(5).into();
	assert_eq!("http request returned an unsuccessful status code", ::std::error::Error::description(&err));
	assert_eq!("http request returned an unsuccessful status code: 5".to_string(), format!("{}", err));
}
