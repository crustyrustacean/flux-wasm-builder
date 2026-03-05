// src/bin/main.rs

use {{crate_name}}_backend::configuration::get_configuration;
use {{crate_name}}_backend::startup::Application;
use {{crate_name}}_backend::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber(
        "{{name}}-backend".into(),
        "info".into(),
        std::io::stdout,
    );
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;

    Ok(())
}
