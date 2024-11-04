use anyhow::Result;
use reqwest::{header, Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{path::Path, str::FromStr};
use tokio::{fs::File, io::AsyncReadExt};

pub struct DifyClient {
    api_key: String,
    base_url: String,
    client: Client,
}

async fn async_read_file_to_vec(file_path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(file_path).await?;
    // MAX buffer size is 1M
    let mut buffer = [0; 1024];
    let mut content = Vec::new();

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        content.extend_from_slice(&buffer[..n]);
    }

    Ok(content)
}

impl DifyClient {
    pub fn new(api_key: &str, base_url: Option<&str>) -> Self {
        let client = Client::new();
        Self {
            api_key: api_key.to_string(),
            base_url: base_url.unwrap_or("https://api.dify.ai/v1").to_string(),
            client,
        }
    }

    async fn send_request(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        json: Option<Value>,
        params: Option<Value>,
        stream: bool,
    ) -> Result<Response> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            header::HeaderValue::from_static("application/json"),
        );

        let url = format!("{}{}", self.base_url, endpoint);

        tracing::debug!("request url: {}, method: {}", url, method);
        tracing::debug!("request payload: {:?}", json);

        let mut request = self
            .client
            .request(method, &url)
            .headers(headers)
            .bearer_auth(self.api_key.clone());

        if let Some(json) = json {
            request = request.json(&json);
        }

        tracing::debug!("{:?}", request);

        if let Some(params) = params {
            request = request.query(&params);
        }

        let request = request.build()?;

        Ok(self.client.execute(request).await?)
    }

    async fn send_request_with_files(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        data: Value,
        file_path: &Path,
    ) -> Result<Response> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(&format!("Bearer {}", self.api_key))?,
        );

        let url = format!("{}{}", self.base_url, endpoint);

        let file_data = async_read_file_to_vec(file_path).await?;

        let file_part = reqwest::multipart::Part::stream(file_data);

        let form = reqwest::multipart::Form::new()
            .text("data", data.to_string())
            .part("file", file_part);

        Ok(self
            .client
            .request(method, &url)
            .headers(headers)
            .multipart(form)
            .send()
            .await?)
    }

    pub async fn message_feedback(
        &self,
        message_id: &str,
        rating: bool,
        user: &str,
    ) -> Result<Response> {
        let data = json!({
            "rating": rating,
            "user": user
        });
        self.send_request(
            reqwest::Method::POST,
            &format!("/messages/{}/feedbacks", message_id),
            Some(data),
            None,
            false,
        )
        .await
    }

    pub async fn get_application_parameters(&self, user: &str) -> Result<Response> {
        let params = json!({
            "user": user
        });
        self.send_request(
            reqwest::Method::GET,
            "/parameters",
            None,
            Some(params),
            false,
        )
        .await
    }

    pub async fn file_upload(&self, user: &str, file_path: &Path) -> Result<Response> {
        let data = json!({
            "user": user
        });
        self.send_request_with_files(reqwest::Method::POST, "/files/upload", data, file_path)
            .await
    }
}

pub struct CompletionClient {
    dify_client: DifyClient,
}

impl CompletionClient {
    pub fn new(api_key: &str, base_url: Option<&str>) -> Self {
        Self {
            dify_client: DifyClient::new(api_key, base_url),
        }
    }

    pub async fn create_completion_message(
        &self,
        inputs: Value,
        response_mode: &str,
        user: &str,
        files: Option<Value>,
    ) -> Result<Response> {
        let mut data = json!({
            "inputs": inputs,
            "response_mode": response_mode,
            "user": user
        });

        if let Some(files) = files {
            data.as_object_mut()
                .unwrap()
                .insert("files".to_string(), files);
        }

        self.dify_client
            .send_request(
                reqwest::Method::POST,
                "/completion-messages",
                Some(data),
                None,
                response_mode == "streaming",
            )
            .await
    }
}

pub struct ChatClient {
    dify_client: DifyClient,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]

pub enum ResponseMode {
    #[serde(rename = "blocking")]
    Block,
    #[serde(rename = "streaming")]
    Stream,
}
impl std::fmt::Display for ResponseMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = serde_json::to_string(&self).expect("response_mode match error");
        write!(f, "{}", val)
    }
}

impl ChatClient {
    pub fn new(api_key: &str, base_url: Option<&str>) -> Self {
        Self {
            dify_client: DifyClient::new(api_key, base_url),
        }
    }

    pub async fn create_chat_message(
        &self,
        inputs: Value,
        query: &str,
        user: &str,
        response_mode: ResponseMode,
        conversation_id: Option<&str>,
        files: Option<Value>,
    ) -> Result<Response> {
        let streaming = response_mode == ResponseMode::Stream;
        let mut data = json!({
            "inputs": inputs,
            "query": query,
            "user": user,
            "response_mode": response_mode.to_string()
        });

        if let Some(conversation_id) = conversation_id {
            data.as_object_mut().unwrap().insert(
                "conversation_id".to_string(),
                Value::String(conversation_id.to_string()),
            );
        }

        if let Some(files) = files {
            data.as_object_mut()
                .unwrap()
                .insert("files".to_string(), files);
        }

        self.dify_client
            .send_request(
                reqwest::Method::POST,
                "/chat-messages",
                Some(data),
                None,
                streaming,
            )
            .await
    }
}

pub struct WorkflowClient {
    dify_client: DifyClient,
}

impl WorkflowClient {
    pub fn new(api_key: &str, base_url: Option<&str>) -> Self {
        Self {
            dify_client: DifyClient::new(api_key, base_url),
        }
    }

    pub async fn run(
        &self,
        inputs: Value,
        response_mode: ResponseMode,
        user: Option<&str>,
    ) -> Result<Response> {
        let data = json!({
            "inputs": inputs,
            "response_mode": response_mode,
            "user": user.unwrap_or("abc-123")
        });

        self.dify_client
            .send_request(
                reqwest::Method::POST,
                "/workflows/run",
                Some(data),
                None,
                false,
            )
            .await
    }
}

pub struct KnowledgeBaseClient {
    dify_client: DifyClient,
    dataset_id: Option<String>,
}

impl KnowledgeBaseClient {
    pub fn new(api_key: &str, base_url: Option<&str>, dataset_id: Option<&str>) -> Self {
        Self {
            dify_client: DifyClient::new(api_key, base_url),
            dataset_id: dataset_id.map(String::from),
        }
    }

    fn get_dataset_id(&self) -> Result<&str> {
        self.dataset_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("dataset_id is not set"))
    }

    pub async fn create_dataset(&self, name: &str) -> Result<Response> {
        let data = json!({
            "name": name
        });
        self.dify_client
            .send_request(reqwest::Method::POST, "/datasets", Some(data), None, false)
            .await
    }
}

impl From<DifyClient> for ChatClient {
    fn from(value: DifyClient) -> Self {
        ChatClient { dify_client: value }
    }
}

impl From<DifyClient> for CompletionClient {
    fn from(value: DifyClient) -> Self {
        CompletionClient { dify_client: value }
    }
}

impl From<DifyClient> for WorkflowClient {
    fn from(value: DifyClient) -> Self {
        WorkflowClient { dify_client: value }
    }
}
