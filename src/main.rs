use libnewsletter::{config, startup::Application, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = telemetry::get_subscriber("newsletter".into(), "info".into(), std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let config = config::settings().expect("Failed to read configuration");
    let application = Application::build(config).await?;

    application.run_until_stopped().await?;

    Ok(())
}
