use std::{collections::VecDeque, time::Duration};

use toxtrain::{
    RetryingTextDetoxClient, TextDetoxHttpClient, TextDetoxHttpPolicy, TextDetoxHttpRawResponse,
    TextDetoxRequestBoundary, TextDetoxSleeper, TextDetoxTransportError,
};

#[test]
fn retries_a_429_then_returns_the_exact_success_response() {
    let requester = FixtureRequester::new([
        response(429, None, b"rate limited"),
        response(200, Some("rev-a"), b"exact page bytes"),
    ]);
    let sleeper = RecordingSleeper::default();
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, test_policy());

    let response = client
        .get("https://fixture.test/rows?offset=0")
        .expect("retried response");

    assert_eq!(response.revision.as_deref(), Some("rev-a"));
    assert_eq!(response.body, b"exact page bytes");
    assert_eq!(client.requester().calls, 2);
    assert_eq!(client.sleeper().delays, vec![Duration::from_secs(5)]);
}

#[test]
fn retries_a_429_for_one_logical_pinned_parquet_download() {
    let requester = FixtureRequester::new([
        response(429, None, b"rate limited"),
        response(200, None, b"exact Parquet bytes"),
    ]);
    let sleeper = RecordingSleeper::default();
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, test_policy());

    let response = client
        .get("https://fixture.test/data/en-00000-of-00001.parquet")
        .expect("retried Parquet response");

    assert_eq!(response.body, b"exact Parquet bytes");
    assert_eq!(client.requester().calls, 2);
    assert_eq!(client.sleeper().delays, vec![Duration::from_secs(5)]);
}

#[test]
fn retries_a_transient_transport_error_then_returns_the_exact_success_response() {
    let requester = SequencedRequester::new([
        Err(TextDetoxTransportError::transient("connection reset")),
        Ok(response(200, Some("rev-a"), b"exact page bytes")),
    ]);
    let sleeper = RecordingSleeper::default();
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, test_policy());

    let response = client
        .get("https://fixture.test/rows?offset=0")
        .expect("retried response");

    assert_eq!(response.revision.as_deref(), Some("rev-a"));
    assert_eq!(response.body, b"exact page bytes");
    assert_eq!(client.requester().calls, 2);
    assert_eq!(client.sleeper().delays, vec![Duration::from_secs(5)]);
}

#[test]
fn retries_a_transient_error_for_one_logical_pinned_parquet_download() {
    let requester = SequencedRequester::new([
        Err(TextDetoxTransportError::transient("connection reset")),
        Ok(response(200, None, b"exact Parquet bytes")),
    ]);
    let sleeper = RecordingSleeper::default();
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, test_policy());

    let response = client
        .get("https://fixture.test/data/en-00000-of-00001.parquet")
        .expect("retried Parquet response");

    assert_eq!(response.body, b"exact Parquet bytes");
    assert_eq!(client.requester().calls, 2);
    assert_eq!(client.sleeper().delays, vec![Duration::from_secs(5)]);
}

#[test]
fn does_not_retry_a_non_429_status() {
    let requester = FixtureRequester::new([response(400, None, b"bad request")]);
    let sleeper = RecordingSleeper::default();
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, test_policy());

    let error = client
        .get("https://fixture.test/rows?offset=0")
        .expect_err("HTTP 400 must fail");

    assert_eq!(error.to_string(), "HTTP 400");
    assert_eq!(client.requester().calls, 1);
    assert!(client.sleeper().delays.is_empty());
}

#[test]
fn selects_retry_after_then_bounded_fallback_delays() {
    let requester = FixtureRequester::new([
        response(429, Some("120"), b"rate limited"),
        response(200, Some("rev-a"), b"one"),
        response(429, None, b"rate limited"),
        response(429, None, b"rate limited"),
        response(200, Some("rev-a"), b"two"),
    ]);
    let sleeper = RecordingSleeper::default();
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, test_policy());

    client
        .get("https://fixture.test/rows?offset=0")
        .expect("Retry-After response");
    client
        .get("https://fixture.test/rows?offset=100")
        .expect("fallback response");

    assert_eq!(
        client.sleeper().delays,
        vec![
            Duration::from_secs(59),
            Duration::from_secs(5),
            Duration::from_secs(10),
        ]
    );
}

#[test]
fn stops_after_the_bounded_429_attempts() {
    let requester =
        FixtureRequester::new(std::iter::repeat_n(response(429, None, b"rate limited"), 6));
    let sleeper = RecordingSleeper::default();
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, test_policy());

    let error = client
        .get("https://fixture.test/rows?offset=0")
        .expect_err("bounded retries must fail");

    assert_eq!(error.to_string(), "HTTP 429");
    assert_eq!(client.requester().calls, 6);
    assert_eq!(client.sleeper().delays.len(), 5);
}

#[test]
fn spaces_consecutive_page_requests() {
    let requester = FixtureRequester::new([
        response(200, Some("rev-a"), b"first"),
        response(200, Some("rev-a"), b"second"),
    ]);
    let sleeper = RecordingSleeper::default();
    let mut policy = test_policy();
    policy.minimum_page_interval = Duration::from_millis(750);
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, policy);

    client
        .get("https://fixture.test/rows?offset=0")
        .expect("first page");
    client
        .get("https://fixture.test/rows?offset=100")
        .expect("second page");

    assert_eq!(client.sleeper().delays, vec![Duration::from_millis(750)]);
}

#[test]
fn caps_the_page_interval_below_sixty_seconds() {
    let requester = FixtureRequester::new([
        response(200, Some("rev-a"), b"first"),
        response(200, Some("rev-a"), b"second"),
    ]);
    let sleeper = RecordingSleeper::default();
    let mut policy = test_policy();
    policy.minimum_page_interval = Duration::from_secs(60);
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, policy);

    client
        .get("https://fixture.test/rows?offset=0")
        .expect("first page");
    client
        .get("https://fixture.test/rows?offset=100")
        .expect("second page");

    assert_eq!(client.sleeper().delays, vec![Duration::from_secs(59)]);
}

#[test]
fn does_not_retry_a_permanent_transport_error() {
    let requester = FailingRequester { calls: 0 };
    let sleeper = RecordingSleeper::default();
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, test_policy());

    let error = client
        .get("https://fixture.test/rows?offset=0")
        .expect_err("transport error must fail");

    assert_eq!(error.to_string(), "fixture transport failure");
    assert_eq!(client.requester().calls, 1);
    assert!(client.sleeper().delays.is_empty());
}

#[test]
fn stops_after_bounded_transient_transport_attempts() {
    let requester = SequencedRequester::new((1..=6).map(|attempt| {
        Err(TextDetoxTransportError::transient(format!(
            "transport failure {attempt}"
        )))
    }));
    let sleeper = RecordingSleeper::default();
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, test_policy());

    let error = client
        .get("https://fixture.test/rows?offset=0")
        .expect_err("bounded retries must fail");

    assert_eq!(error.to_string(), "transport failure 6");
    assert_eq!(client.requester().calls, 6);
    assert_eq!(
        client.sleeper().delays,
        vec![
            Duration::from_secs(5),
            Duration::from_secs(10),
            Duration::from_secs(20),
            Duration::from_secs(40),
            Duration::from_secs(59),
        ]
    );
}

#[test]
fn shares_one_attempt_budget_between_429_and_transient_errors() {
    let requester = SequencedRequester::new([
        Ok(response(429, None, b"rate limited")),
        Ok(response(429, None, b"rate limited")),
        Ok(response(429, None, b"rate limited")),
        Err(TextDetoxTransportError::transient("transport failure 1")),
        Err(TextDetoxTransportError::transient("transport failure 2")),
        Err(TextDetoxTransportError::transient("transport failure 3")),
        Ok(response(200, Some("rev-a"), b"must not be reached")),
    ]);
    let sleeper = RecordingSleeper::default();
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, test_policy());

    let error = client
        .get("https://fixture.test/rows?offset=0")
        .expect_err("the shared attempt budget must fail");

    assert_eq!(error.to_string(), "transport failure 3");
    assert_eq!(client.requester().calls, 6);
    assert_eq!(
        client.sleeper().delays,
        vec![
            Duration::from_secs(5),
            Duration::from_secs(10),
            Duration::from_secs(20),
            Duration::from_secs(40),
            Duration::from_secs(59),
        ]
    );
}

#[test]
fn does_not_repeat_page_interval_inside_one_logical_request() {
    let requester = SequencedRequester::new([
        Ok(response(200, Some("rev-a"), b"first")),
        Err(TextDetoxTransportError::transient("connection reset")),
        Ok(response(200, Some("rev-a"), b"second")),
    ]);
    let sleeper = RecordingSleeper::default();
    let mut policy = test_policy();
    policy.minimum_page_interval = Duration::from_millis(750);
    let mut client = RetryingTextDetoxClient::with_policy(requester, sleeper, policy);

    client
        .get("https://fixture.test/rows?offset=0")
        .expect("first page");
    client
        .get("https://fixture.test/rows?offset=100")
        .expect("retried second page");

    assert_eq!(client.requester().calls, 3);
    assert_eq!(
        client.sleeper().delays,
        vec![Duration::from_millis(750), Duration::from_secs(5)]
    );
}

fn test_policy() -> TextDetoxHttpPolicy {
    TextDetoxHttpPolicy {
        max_attempts: 6,
        base_retry_delay: Duration::from_secs(5),
        maximum_retry_delay: Duration::from_secs(59),
        minimum_page_interval: Duration::ZERO,
    }
}

fn response(status: u16, retry_after: Option<&str>, body: &[u8]) -> TextDetoxHttpRawResponse {
    TextDetoxHttpRawResponse {
        status,
        revision: (status == 200).then(|| "rev-a".to_owned()),
        retry_after: retry_after.map(str::to_owned),
        body: body.to_vec(),
    }
}

struct FixtureRequester {
    responses: VecDeque<TextDetoxHttpRawResponse>,
    calls: usize,
}

impl FixtureRequester {
    fn new(responses: impl IntoIterator<Item = TextDetoxHttpRawResponse>) -> Self {
        Self {
            responses: responses.into_iter().collect(),
            calls: 0,
        }
    }
}

impl TextDetoxRequestBoundary for FixtureRequester {
    fn get(&mut self, _url: &str) -> Result<TextDetoxHttpRawResponse, TextDetoxTransportError> {
        self.calls += 1;
        self.responses
            .pop_front()
            .ok_or_else(|| TextDetoxTransportError::new("unexpected request"))
    }
}

struct SequencedRequester {
    responses: VecDeque<Result<TextDetoxHttpRawResponse, TextDetoxTransportError>>,
    calls: usize,
}

impl SequencedRequester {
    fn new(
        responses: impl IntoIterator<Item = Result<TextDetoxHttpRawResponse, TextDetoxTransportError>>,
    ) -> Self {
        Self {
            responses: responses.into_iter().collect(),
            calls: 0,
        }
    }
}

impl TextDetoxRequestBoundary for SequencedRequester {
    fn get(&mut self, _url: &str) -> Result<TextDetoxHttpRawResponse, TextDetoxTransportError> {
        self.calls += 1;
        self.responses
            .pop_front()
            .unwrap_or_else(|| Err(TextDetoxTransportError::new("unexpected request")))
    }
}

struct FailingRequester {
    calls: usize,
}

impl TextDetoxRequestBoundary for FailingRequester {
    fn get(&mut self, _url: &str) -> Result<TextDetoxHttpRawResponse, TextDetoxTransportError> {
        self.calls += 1;
        Err(TextDetoxTransportError::new("fixture transport failure"))
    }
}

#[derive(Default)]
struct RecordingSleeper {
    delays: Vec<Duration>,
}

impl TextDetoxSleeper for RecordingSleeper {
    fn sleep(&mut self, delay: Duration) {
        self.delays.push(delay);
    }
}
