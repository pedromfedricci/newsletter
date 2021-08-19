// temporarly workaround for clippy incorrect
// lint at crate::routes::subscription::subscribe
#![allow(clippy::async_yields_async)]

pub mod config;
mod domain;
mod routes;
pub mod startup;
pub mod telemetry;
