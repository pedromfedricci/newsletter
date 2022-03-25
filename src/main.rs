use std::fmt::{Debug, Display};

use tokio::task::JoinError;

use libnewsletter::config;
use libnewsletter::issue_delivery_worker::run_worker_until_stopped;
use libnewsletter::startup::Application;
use libnewsletter::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("newsletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = config::settings().expect("Failed to read configuration");
    let app = Application::build(config.clone()).await?;
    let server = tokio::spawn(app.run_until_stopped());
    let delivery = tokio::spawn(run_worker_until_stopped(config));

    tokio::select! {
        out = server => report_exit("API server", out),
        out = delivery => report_exit("Delivery worker", out),
    };

    Ok(())
}

fn report_exit(task: &str, out: Result<Result<(), impl Debug + Display>, JoinError>) {
    match out {
        Ok(Ok(())) => tracing::info!("{task} has exited"),
        Ok(Err(err)) => {
            tracing::error!(error.cause_chain = ?err, error.message = %err, "{task} failed")
        }
        Err(err) => {
            tracing::error!(error.cause_chain = ?err, error_message = %err, "{task} failed to complete")
        }
    }
}
