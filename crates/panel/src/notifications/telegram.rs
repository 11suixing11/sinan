use reqwest::{Client, redirect::Policy};
use serde_json::{Value, json};
use std::time::Duration;

pub(super) struct Failure {
    pub message: String,
    pub retry_after: Option<i64>,
}

impl Failure {
    fn request() -> Self {
        Self {
            message: "Telegram 请求失败，请检查网络、机器人令牌与会话 ID".into(),
            retry_after: None,
        }
    }
}

pub(super) async fn send(token: &str, chat: &str, text: &str) -> Result<(), Failure> {
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    deliver(&url, chat, text).await
}

async fn deliver(url: &str, chat: &str, text: &str) -> Result<(), Failure> {
    let client = Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|_| Failure::request())?;
    // Do not log reqwest errors or remote descriptions: either can contain the bot token.
    let mut response = client
        .post(url)
        .json(&json!({"chat_id":chat,"text":text}))
        .send()
        .await
        .map_err(|_| Failure::request())?;
    let status = response.status();
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Failure::request())? {
        if bytes.len() + chunk.len() > 64 * 1024 {
            return Err(Failure::request());
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| Failure::request())?;
    if status.is_success() && value.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok(());
    }
    Err(Failure {
        message: format!(
            "Telegram 未接受通知（HTTP {}），请检查配置或稍后重试",
            status.as_u16()
        ),
        retry_after: value
            .pointer("/parameters/retry_after")
            .and_then(Value::as_i64)
            .map(|delay| delay.clamp(1, 86400)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn rate_limit_honors_retry_and_never_exposes_remote_secrets() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}/botTEST_SECRET/sendMessage",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 4096];
            let count = stream.read(&mut request).await.unwrap();
            assert!(count > 0);
            let body =
                r#"{"ok":false,"description":"TEST_SECRET","parameters":{"retry_after":123}}"#;
            stream.write_all(format!("HTTP/1.1 429 Too Many Requests\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        });
        let error = deliver(&url, "-100000", "测试通知").await.err().unwrap();
        assert_eq!(error.retry_after, Some(123));
        assert!(!error.message.contains("TEST_SECRET"));
        server.await.unwrap();
    }
}
