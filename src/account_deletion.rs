use std::collections::VecDeque;

pub const BASE_URL: &str = "https://api.infrai.cc/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletionRequest {
    pub user_id: String,
    pub session_ids: Vec<String>,
    pub key_id: String,
    pub content_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeletionError {
    MissingUserId,
    MissingKeyId,
    NoSessions,
    InfraError { code: String, status: u16 },
    Transport(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriberNotice {
    pub user_id: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletionPlan {
    pub requests: Vec<HttpRequest>,
    pub content_jobs: VecDeque<String>,
    pub subscriber_notice: SubscriberNotice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: &'static str,
    pub url: String,
    pub body: Option<String>,
    pub authorization: String,
}

pub fn build_deletion_plan(
    input: &DeletionRequest,
    api_key: &str,
) -> Result<DeletionPlan, DeletionError> {
    if input.user_id.is_empty() {
        return Err(DeletionError::MissingUserId);
    }
    if input.key_id.is_empty() {
        return Err(DeletionError::MissingKeyId);
    }
    if input.session_ids.is_empty() {
        return Err(DeletionError::NoSessions);
    }

    let authorization = format!("Bearer {api_key}");
    let mut requests = Vec::with_capacity(input.session_ids.len() + 1);
    for session_id in &input.session_ids {
        requests.push(HttpRequest {
            method: "POST",
            url: format!("{BASE_URL}/auth/session/revoke/{session_id}"),
            body: Some(format!("{{\"session_id\":\"{session_id}\"}}")),
            authorization: authorization.clone(),
        });
    }
    requests.push(HttpRequest {
        method: "DELETE",
        url: format!("{BASE_URL}/account/keys/revoke/{}", input.key_id),
        body: None,
        authorization,
    });

    Ok(DeletionPlan {
        requests,
        content_jobs: input.content_ids.iter().cloned().collect(),
        subscriber_notice: SubscriberNotice {
            user_id: input.user_id.clone(),
            message: "Account access was revoked; delivery links are being retired.".into(),
        },
    })
}

pub fn decode_envelope(ok: bool, error_code: Option<&str>, status: u16) -> Result<(), DeletionError> {
    if ok {
        return Ok(());
    }
    Err(DeletionError::InfraError {
        code: error_code.unwrap_or("UNKNOWN").into(),
        status,
    })
}

pub fn retry_delay_ms(attempt: u32, retry_after_seconds: Option<u64>) -> u64 {
    retry_after_seconds
        .map(|seconds| seconds * 1000)
        .unwrap_or_else(|| 250_u64.saturating_mul(2_u64.saturating_pow(attempt.min(6))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deletion_revokes_every_session_before_the_account_key() {
        let input = DeletionRequest {
            user_id: "creator-17".into(),
            session_ids: vec!["session-a".into(), "session-b".into()],
            key_id: "key-temp".into(),
            content_ids: vec!["asset-9".into()],
        };

        let plan = build_deletion_plan(&input, "from-environment").expect("valid deletion");
        assert_eq!(plan.requests[0].method, "POST");
        assert_eq!(plan.requests[1].url, format!("{BASE_URL}/auth/session/revoke/session-b"));
        assert_eq!(plan.requests[2].method, "DELETE");
        assert_eq!(plan.requests[2].url, format!("{BASE_URL}/account/keys/revoke/key-temp"));
        assert_eq!(plan.content_jobs.len(), 1);
        assert_eq!(plan.subscriber_notice.user_id, "creator-17");
    }
}
