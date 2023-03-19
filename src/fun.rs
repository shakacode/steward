use std::{fmt::Display, future::Future};

use crate::fmt;

/// A function that prints a headline of a task and runs the task ([`Fn`](std::ops::Fn)).
///
/// ```
/// steward::run("Seeding database", || async { Migrator::up().await }).await
/// ```
pub async fn run<Fun, Fut, Ok, Err>(msg: impl Display, f: Fun) -> Result<Ok, Err>
where
    Fun: Fn() -> Fut,
    Fut: Future<Output = Result<Ok, Err>>,
{
    eprintln!("{}", fmt::plain_headline(msg));
    f().await
}

/// A function that prints a headline of a task and runs the task ([`FnMut`](std::ops::FnMut)).
///
/// ```
/// steward::run("Stopping server", || async { process.stop().await }).await
/// ```
pub async fn run_mut<Fun, Fut, Ok, Err>(msg: impl Display, mut f: Fun) -> Result<Ok, Err>
where
    Fun: FnMut() -> Fut,
    Fut: Future<Output = Result<Ok, Err>>,
{
    eprintln!("{}", fmt::plain_headline(msg));
    f().await
}

/// A function that prints a headline of a task and runs the task ([`FnOnce`](std::ops::FnOnce)).
///
/// ```
/// steward::run("Stopping server", || async { process.stop().await }).await
/// ```
pub async fn run_once<Fun, Fut, Ok, Err>(msg: impl Display, f: Fun) -> Result<Ok, Err>
where
    Fun: FnOnce() -> Fut,
    Fut: Future<Output = Result<Ok, Err>>,
{
    eprintln!("{}", fmt::plain_headline(msg));
    f().await
}
