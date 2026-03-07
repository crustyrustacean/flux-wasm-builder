// tests/api/helpers.rs

use {{ crate_name }}_backend::configuration::get_configuration;
use {{ crate_name }}_backend::startup::Application;
use {{ crate_name }}_backend::telemetry::{get_subscriber, init_subscriber};
use std::sync::LazyLock;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber("test".into(), "info".into(), std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber("test".into(), "info".into(), std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
}

pub async fn spawn_app() -> TestApp {
    LazyLock::force(&TRACING);

    let mut configuration = get_configuration()
        .expect("Failed to read configuration.");
    configuration.application.port = 0; // random port

    let application = Application::build(configuration)
        .await
        .expect("Failed to build application.");
    let port = application.port();
    tokio::spawn(application.run_until_stopped());

    TestApp {
        address: format!("http://localhost:{}", port),
        port,
    }
}
