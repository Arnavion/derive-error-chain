#![allow(dead_code)]

//! Test crate for derive-error-chain. If it runs, it's tested.

extern crate error_chain;
#[macro_use]
extern crate derive_error_chain;

fn main() {
	smoke_test_1();
	smoke_test_2();
	smoke_test_4();
	smoke_test_8();
	has_backtrace_depending_on_env();
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

	public_api_test();
	inlined_description_and_display();
}

// Upstream tests

fn smoke_test_1() {
	#[derive(Debug, error_chain)]
	#[error_chain(error = "Error", result_ext = "ResultExt", result = "Result")]
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

	let original_value = env::var_os("RUST_BACKTRACE");

	// missing RUST_BACKTRACE
	env::remove_var("RUST_BACKTRACE");
	let err = Error::from(ErrorKind::MyError);
	assert!(err.backtrace().is_none());

	// RUST_BACKTRACE=0
	env::set_var("RUST_BACKTRACE", "0");
	let err = Error::from(ErrorKind::MyError);
	assert!(err.backtrace().is_none());

	// RUST_BACKTRACE set to anything but 0
	env::set_var("RUST_BACKTRACE", "yes");
	let err = Error::from(ErrorKind::MyError);
	assert!(err.backtrace().is_some());

	if let Some(var) = original_value {
		env::set_var("RUST_BACKTRACE", var);
	}
}

fn chain_err() {
	use std::fmt;

	#[derive(Debug, error_chain)]
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
		#[derive(Debug, error_chain)]
		pub enum ErrorKind {
			Msg(String),
		}
	}

	#[derive(Debug, error_chain)]
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

	#[derive(Debug, error_chain)]
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
		#[derive(Debug, error_chain)]
		pub enum ErrorKind {
			Msg(String),
		}
	}

	#[derive(Debug, error_chain)]
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
	#[derive(Debug, error_chain)]
	pub enum ErrorKind {
		Msg(String),
	}

	let _: Result<()> = Ok(());
}

fn without_result() {
	#[derive(Debug, error_chain)]
	#[error_chain(result = "")]
	pub enum ErrorKind {
		Msg(String),
	}

	let _: Result<(), ()> = Ok(());
}

fn documentation() {
	mod inner {
		#[derive(Debug, error_chain)]
		pub enum ErrorKind {
			Msg(String),
		}
	}

	#[derive(Debug, error_chain)]
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
	#[derive(Debug, error_chain)]
	#[error_chain(error = "MyError", result_ext = "MyResultExt", result = "MyResult")]
	pub enum MyErrorKind {
		Msg(String),
	}

	#[derive(Debug, error_chain)]
	pub enum ErrorKind {
		Msg(String),
	}
}

#[deny(dead_code)]
mod allow_dead_code {
	#[derive(Debug, error_chain)]
	#[error_chain(result = "")]
	pub enum ErrorKind {
		Msg(String),
	}
}

// Make sure links actually work!
fn rustup_regression() {
	mod mock {
		#[derive(Debug, error_chain)]
		pub enum ErrorKind {
			Msg(String),
		}
	}

	#[derive(Debug, error_chain)]
	pub enum ErrorKind {
		Msg(String),

		#[error_chain(link = "mock::Error")]
		Download(mock::ErrorKind),

		#[error_chain(custom)]
		#[error_chain(description = r#"(|| "could not locate working directory")"#)]
		LocatingWorkingDir,
	}
}

fn error_patterns() {
	#[derive(Debug, error_chain)]
	pub enum ErrorKind {
		Msg(String),
	}

	// Tuples look nice when matching errors
	match Error::from("Test") {
		Error(ErrorKind::Msg(_), _) => {
		}
	}
}

// Own tests

mod test2 {
	#[derive(Debug, error_chain)]
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
