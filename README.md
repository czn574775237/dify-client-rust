# dify-client-rust

dify client sdk for `Rust`

## Installation

```Cargo.toml
[dependencies]
dify-client-rust = { git ="https://github.com/czn574775237/dify-client-rust.git" }
```

## Quick Start


`streaming` call

```rs
#[tokio::test]
async fn test_streaming_chat() {
    use futures_util::StreamExt;
    
    let client = ChatClient::new("api_key", Some(""));
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

```

`blocking` call

```rs
use dify_client_rust::{ChatClient, DifyClient, ResponseMode};

#[tokio::test]
async fn test_blocking_chat() {
    
    let client = ChatClient::new("api_key", Some(""));
    let result = client
        .create_chat_message(json!({}), "hi", "zhining", ResponseMode::Block, None, None)
        .await
        .unwrap();
    let status = result.status();
    let res = result.text().await.unwrap();
    tracing::debug!("result {:?}", res);
    assert_eq!(status, 200);
}
```

