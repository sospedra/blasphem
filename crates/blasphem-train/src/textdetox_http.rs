use std::{io::Read, thread, time::Duration};

use reqwest::blocking::Client;

use crate::{
    TextDetoxHttpClient, TextDetoxHttpResponse, TextDetoxTransportError,
    acquisition::MAX_SOURCE_DOWNLOAD_BYTES,
};

pub const MAX_TEXTDETOX_SLEEP: Duration = Duration::from_secs(59);
pub const DEFAULT_TEXTDETOX_MAX_ATTEMPTS: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDetoxHttpRawResponse {
    pub status: u16,
    pub revision: Option<String>,
    pub retry_after: Option<String>,
    pub body: Vec<u8>,
}

pub trait TextDetoxRequestBoundary {
    fn get(&mut self, url: &str) -> Result<TextDetoxHttpRawResponse, TextDetoxTransportError>;
}

pub trait TextDetoxSleeper {
    fn sleep(&mut self, delay: Duration);
}

pub struct ThreadSleeper;

impl TextDetoxSleeper for ThreadSleeper {
    fn sleep(&mut self, delay: Duration) {
        thread::sleep(delay);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDetoxHttpPolicy {
    pub max_attempts: usize,
    pub base_retry_delay: Duration,
    pub maximum_retry_delay: Duration,
    pub minimum_page_interval: Duration,
}

impl Default for TextDetoxHttpPolicy {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_TEXTDETOX_MAX_ATTEMPTS,
            base_retry_delay: Duration::from_secs(5),
            maximum_retry_delay: MAX_TEXTDETOX_SLEEP,
            minimum_page_interval: Duration::from_millis(750),
        }
    }
}

pub struct RetryingTextDetoxClient<Requester, Sleeper> {
    requester: Requester,
    sleeper: Sleeper,
    policy: TextDetoxHttpPolicy,
    has_requested_page: bool,
}

impl<Requester, Sleeper> RetryingTextDetoxClient<Requester, Sleeper> {
    #[must_use]
    pub fn with_policy(
        requester: Requester,
        sleeper: Sleeper,
        mut policy: TextDetoxHttpPolicy,
    ) -> Self {
        policy.max_attempts = policy.max_attempts.max(1);
        policy.maximum_retry_delay = policy.maximum_retry_delay.min(MAX_TEXTDETOX_SLEEP);
        policy.minimum_page_interval = policy.minimum_page_interval.min(MAX_TEXTDETOX_SLEEP);
        Self {
            requester,
            sleeper,
            policy,
            has_requested_page: false,
        }
    }

    #[must_use]
    pub const fn requester(&self) -> &Requester {
        &self.requester
    }

    #[must_use]
    pub const fn sleeper(&self) -> &Sleeper {
        &self.sleeper
    }
}

impl<Requester, Sleeper> TextDetoxHttpClient for RetryingTextDetoxClient<Requester, Sleeper>
where
    Requester: TextDetoxRequestBoundary,
    Sleeper: TextDetoxSleeper,
{
    fn get(&mut self, url: &str) -> Result<TextDetoxHttpResponse, TextDetoxTransportError> {
        if is_page_request(url) && self.has_requested_page {
            sleep_if_positive(&mut self.sleeper, self.policy.minimum_page_interval);
        }
        if is_page_request(url) {
            self.has_requested_page = true;
        }

        for attempt in 1..=self.policy.max_attempts {
            let response = match self.requester.get(url) {
                Ok(response) => response,
                Err(error) if error.is_transient() && attempt < self.policy.max_attempts => {
                    let delay = fallback_retry_delay(&self.policy, attempt - 1);
                    eprintln!(
                        "TextDetox transport retry {attempt}/{} after {} seconds",
                        self.policy.max_attempts - 1,
                        delay.as_secs()
                    );
                    sleep_if_positive(&mut self.sleeper, delay);
                    continue;
                }
                Err(error) => return Err(error),
            };
            if (200..300).contains(&response.status) {
                return Ok(TextDetoxHttpResponse {
                    revision: response.revision,
                    body: response.body,
                });
            }
            if response.status != 429 || attempt == self.policy.max_attempts {
                return Err(http_status_error(response.status));
            }

            let delay = retry_delay(&response, &self.policy, attempt - 1);
            eprintln!(
                "TextDetox HTTP 429 retry {attempt}/{} after {} seconds",
                self.policy.max_attempts - 1,
                delay.as_secs()
            );
            sleep_if_positive(&mut self.sleeper, delay);
        }

        Err(TextDetoxTransportError::new("retry attempts exhausted"))
    }
}

pub struct ReqwestTextDetoxClient {
    inner: RetryingTextDetoxClient<ReqwestTextDetoxRequester, ThreadSleeper>,
}

impl ReqwestTextDetoxClient {
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self {
            inner: RetryingTextDetoxClient::with_policy(
                ReqwestTextDetoxRequester { client },
                ThreadSleeper,
                TextDetoxHttpPolicy::default(),
            ),
        }
    }
}

impl TextDetoxHttpClient for ReqwestTextDetoxClient {
    fn get(&mut self, url: &str) -> Result<TextDetoxHttpResponse, TextDetoxTransportError> {
        self.inner.get(url)
    }
}

struct ReqwestTextDetoxRequester {
    client: Client,
}

impl TextDetoxRequestBoundary for ReqwestTextDetoxRequester {
    fn get(&mut self, url: &str) -> Result<TextDetoxHttpRawResponse, TextDetoxTransportError> {
        let response = self
            .client
            .get(url)
            .send()
            .map_err(classify_reqwest_send_error)?;
        let status = response.status().as_u16();
        let revision = response
            .headers()
            .get("x-revision")
            .map(|value| {
                value
                    .to_str()
                    .map(str::to_owned)
                    .map_err(|error| TextDetoxTransportError::new(error.to_string()))
            })
            .transpose()?;
        let retry_after = response
            .headers()
            .get("retry-after")
            .map(|value| {
                value
                    .to_str()
                    .map(str::to_owned)
                    .map_err(|error| TextDetoxTransportError::new(error.to_string()))
            })
            .transpose()?;
        let body = if (200..300).contains(&status) {
            if response
                .content_length()
                .is_some_and(|length| length > MAX_SOURCE_DOWNLOAD_BYTES as u64)
            {
                return Err(TextDetoxTransportError::new(format!(
                    "TextDetox response exceeds {MAX_SOURCE_DOWNLOAD_BYTES} bytes"
                )));
            }
            let mut bytes = Vec::new();
            response
                .take(MAX_SOURCE_DOWNLOAD_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(classify_response_read_error)?;
            if bytes.len() > MAX_SOURCE_DOWNLOAD_BYTES {
                return Err(TextDetoxTransportError::new(format!(
                    "TextDetox response exceeds {MAX_SOURCE_DOWNLOAD_BYTES} bytes"
                )));
            }
            bytes
        } else {
            Vec::new()
        };
        Ok(TextDetoxHttpRawResponse {
            status,
            revision,
            retry_after,
            body,
        })
    }
}

fn is_page_request(url: &str) -> bool {
    url.contains("/rows?")
}

fn retry_delay(
    response: &TextDetoxHttpRawResponse,
    policy: &TextDetoxHttpPolicy,
    retry_index: usize,
) -> Duration {
    let requested = response
        .retry_after
        .as_deref()
        .and_then(retry_after_seconds)
        .map(Duration::from_secs)
        .unwrap_or_else(|| fallback_delay(policy.base_retry_delay, retry_index));
    requested
        .min(policy.maximum_retry_delay)
        .min(MAX_TEXTDETOX_SLEEP)
}

fn fallback_retry_delay(policy: &TextDetoxHttpPolicy, retry_index: usize) -> Duration {
    fallback_delay(policy.base_retry_delay, retry_index)
        .min(policy.maximum_retry_delay)
        .min(MAX_TEXTDETOX_SLEEP)
}

fn classify_reqwest_send_error(error: reqwest::Error) -> TextDetoxTransportError {
    let message = error.to_string();
    if error.is_request() || error.is_connect() || error.is_timeout() {
        TextDetoxTransportError::transient(message)
    } else {
        TextDetoxTransportError::new(message)
    }
}

fn classify_response_read_error(error: std::io::Error) -> TextDetoxTransportError {
    let message = error.to_string();
    let transient_kind = matches!(
        error.kind(),
        std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::UnexpectedEof
            | std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::WouldBlock
    );
    let transient_source = error
        .get_ref()
        .and_then(|source| source.downcast_ref::<reqwest::Error>())
        .is_some_and(|source| source.is_body() || source.is_decode() || source.is_timeout());
    if transient_kind || transient_source {
        TextDetoxTransportError::transient(message)
    } else {
        TextDetoxTransportError::new(message)
    }
}

fn retry_after_seconds(value: &str) -> Option<u64> {
    value.trim().parse().ok()
}

fn fallback_delay(base: Duration, retry_index: usize) -> Duration {
    let multiplier = 1_u32 << retry_index.min(31);
    base.checked_mul(multiplier).unwrap_or(MAX_TEXTDETOX_SLEEP)
}

fn http_status_error(status: u16) -> TextDetoxTransportError {
    TextDetoxTransportError::new(format!("HTTP {status}"))
}

fn sleep_if_positive(sleeper: &mut impl TextDetoxSleeper, delay: Duration) {
    if !delay.is_zero() {
        sleeper.sleep(delay);
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Read as _, io::Write, net::TcpListener, thread, time::Duration};

    use reqwest::blocking::Client;

    use super::{classify_reqwest_send_error, classify_response_read_error};

    #[test]
    fn classifies_a_send_timeout_as_transient_and_preserves_its_message() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local server");
        let address = listener.local_addr().expect("local address");
        let server = thread::spawn(move || {
            let (_stream, _) = listener.accept().expect("accept request");
            thread::sleep(Duration::from_millis(100));
        });
        let client = Client::builder()
            .timeout(Duration::from_millis(20))
            .build()
            .expect("build client");

        let source = client
            .get(format!("http://{address}/"))
            .send()
            .expect_err("the local server does not respond");
        assert!(source.is_timeout());
        let expected_message = source.to_string();

        let classified = classify_reqwest_send_error(source);

        assert!(classified.is_transient());
        assert_eq!(classified.to_string(), expected_message);
        server.join().expect("join local server");
    }

    #[test]
    fn classifies_a_request_builder_error_as_permanent() {
        let source = Client::new()
            .get("::not-a-url::")
            .send()
            .expect_err("invalid request URL");
        assert!(source.is_builder());

        let classified = classify_reqwest_send_error(source);

        assert!(!classified.is_transient());
    }

    #[test]
    fn classifies_an_incomplete_response_body_as_transient() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local server");
        let address = listener.local_addr().expect("local address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).expect("read request headers");
                assert_ne!(read, 0, "request ended before its headers");
                request.extend_from_slice(&buffer[..read]);
            }
            assert!(request.starts_with(b"GET / HTTP/1.1\r\n"));
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\nshort",
                )
                .expect("write incomplete response");
        });
        let mut response = Client::new()
            .get(format!("http://{address}/"))
            .send()
            .expect("read response headers");
        let mut body = Vec::new();
        let source = response
            .read_to_end(&mut body)
            .expect_err("the response body is incomplete");

        let classified = classify_response_read_error(source);

        assert!(classified.is_transient());
        server.join().expect("join local server");
    }
}
