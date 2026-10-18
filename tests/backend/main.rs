//! Integration tests against every supported database backend.
//!
//! The scenarios live in `common` and run unchanged against each backend
//! through a small `Harness` implementation per backend. `SQLite` tests
//! always run. Postgres and MySQL tests are marked
//! `#[ignore]` and run with `cargo test --test backend -- --include-ignored`
//! once `POSTGRES_DATABASE_URL` and `MYSQL_DATABASE_URL` point at databases,
//! which is what CI does.
#![cfg(test)]

mod common;

#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "sqlite")]
mod sqlite;
