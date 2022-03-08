use cjms::appconfig::{connect_to_database_and_migrate, run_server};
use cjms::settings::{get_settings, Settings};
use fake::{Fake, StringFaker};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;

pub struct TestApp {
    pub settings: Settings,
}
impl TestApp {
    pub fn build_url(&self, path: &str) -> String {
        format!("http://{}{}", self.settings.server_address(), path)
    }

    pub fn db_connection(&self) -> PgPool {
        PgPoolOptions::new()
            .connect_timeout(std::time::Duration::from_secs(2))
            .connect_lazy(&self.settings.database_url)
            .expect("Could not get DB connection for test")
    }
}

async fn create_test_database(database_url: &str) -> String {
    let randomized_test_database_url = format!("{}_test_{}", database_url, Uuid::new_v4());
    let url_parts: Vec<&str> = randomized_test_database_url.rsplit('/').collect();
    let database_name = url_parts.get(0).unwrap().to_string();
    let mut connection = PgConnection::connect(database_url)
        .await
        .expect("Failed to connect to postgres.");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, &database_name).as_str())
        .await
        .expect("Failed to create test database.");
    println!("Database is: {}", randomized_test_database_url);
    randomized_test_database_url
}

pub async fn spawn_app() -> TestApp {
    let mut settings = get_settings();
    let listener =
        TcpListener::bind(format!("{}:0", settings.host)).expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let test_database_url = create_test_database(&settings.database_url).await;
    let db_pool = connect_to_database_and_migrate(&test_database_url).await;
    let server = run_server(listener, db_pool).expect("Failed to start server");
    let _ = tokio::spawn(server);
    settings.database_url = test_database_url;
    settings.port = format!("{}", port);
    TestApp { settings }
}

pub fn random_ascii_string() -> String {
    const ASCII: &str =
        "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ!\"#$%&\'()*+,-./:;<=>?@";
    let f = StringFaker::with(Vec::from(ASCII), 8..90);
    f.fake()
}

pub async fn send_get_request(app: &TestApp, path: &str) -> reqwest::Response {
    let path = app.build_url(path);
    reqwest::get(&path).await.expect("Failed to GET")
}

pub async fn send_post_request(
    app: &TestApp,
    path: &str,
    data: serde_json::Value,
) -> reqwest::Response {
    let path = app.build_url(path);
    let client = reqwest::Client::new();
    client
        .post(&path)
        .json(&data)
        .send()
        .await
        .expect("Failed to POST")
}

pub async fn send_put_request(
    app: &TestApp,
    path: &str,
    data: serde_json::Value,
) -> reqwest::Response {
    let path = app.build_url(path);
    let client = reqwest::Client::new();
    client
        .put(&path)
        .json(&data)
        .send()
        .await
        .expect("Failed to PUT")
}