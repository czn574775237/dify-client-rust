use dify_client_rust::{ChatClient, DifyClient, ResponseMode};
use serde_json::json;
use std::env;
use std::sync::Once;
use tokio;

static TRACING: Once = Once::new();

fn get_client() -> DifyClient {
    dotenvy::dotenv().expect("failed to load .env");

    DifyClient::new(
        &env::var("DIFY_API_KEY").unwrap(),
        Some(&env::var("DIFY_BASE_API").unwrap()),
    )
}

fn init_tracing_subscriber() {
    TRACING.call_once(|| {
        let subscriber = tracing_subscriber::fmt()
            .compact()
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_target(false)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set global subscriber");
    });
}

#[tokio::test]
async fn test_blocking_chat() {
    init_tracing_subscriber();

    let client = ChatClient::from(get_client());
    let result = client
        .create_chat_message(json!({}), "hi", "zhining", ResponseMode::Block, None, None)
        .await
        .unwrap();
    let status = result.status();
    let res = result.text().await.unwrap();
    tracing::debug!("result {:?}", res);
    assert_eq!(status, 200);
}

#[tokio::test]
async fn test_streaming_chat() {
    use futures_util::StreamExt;
    init_tracing_subscriber();

    let client = ChatClient::from(get_client());
    let result = client
        .create_chat_message(
            json!({}),
            "hi",
            "mock-user",
            ResponseMode::Stream,
            None,
            None,
        )
        .await
        .unwrap();
    let status = result.status();

    assert_eq!(status, 200);

    let mut stream = result.bytes_stream();
    while let Some(Ok(item)) = stream.next().await {
        tracing::debug!("{:?}", String::from_utf8(item.to_vec()).unwrap());
    }
}
