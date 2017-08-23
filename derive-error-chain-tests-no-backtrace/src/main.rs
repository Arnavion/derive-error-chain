#![allow(dead_code)]

//! Test crate for derive-error-chain. If it runs, it's tested.

#[macro_use]
extern crate derive_error_chain;

fn main() {
	can_disable_backtrace();
}

fn can_disable_backtrace() {
	#[derive(Debug, ErrorChain)]
	#[error_chain(backtrace = "false")]
	pub enum ErrorKind {
		Msg(String),
	}

	let err: Error = ErrorKind::Msg("foo".to_string()).into();
	assert!(err.backtrace().is_none());
	assert_eq!(
		r#"Error(Msg("foo"), State { next_error: None })"#,
		format!("{:?}", err)
	);
}

#[deny(dead_code)]
mod allow_dead_code {
	#[derive(Debug, ErrorChain)]
	#[error_chain(result = "", backtrace = "false")]
	pub enum ErrorKind {
		Msg(String),
	}
}
