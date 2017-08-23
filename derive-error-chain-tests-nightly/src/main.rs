#![allow(dead_code)]
#![feature(proc_macro)]

//! Test crate for derive-error-chain. If it runs, it's tested.

#[macro_use]
extern crate derive_error_chain;
extern crate error_chain;

fn main() {
	macro_conflicts_use();
	macro_conflicts_fully_qualified();
}

fn macro_conflicts_use() {
	use error_chain::{ bail, error_chain as error_chain_macro, error_chain_processed, error_chain_processing, impl_extract_backtrace, quick_error, quick_main };

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
	use error_chain::{ error_chain_processed, error_chain_processing, impl_extract_backtrace, quick_error };

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
