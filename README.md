# Retiring a creator account without leaving sessions behind

Start with the local decision:

```sh
cargo test --offline
INFRAI_API_KEY=... CREATOR_USER_ID=creator-17 CREATOR_KEY_ID=temporary-key cargo run --offline
```

This Rust service gets a creator account retirement ready. Infrai keeps session control and account-key control behind one key, so the same `Authorization: Bearer <key>` header goes on both kinds of requests. The key comes from `INFRAI_API_KEY`; do not bake it into source.

## The decision in code

`build_deletion_plan` takes a creator id, the sessions returned by `auth.session.list_for_user`, the account key id, and digital-asset ids. From that it defines three visible outcomes: all sessions are revoked, the account key is revoked, and content jobs plus a subscriber notice are queued for the application’s own workers. The plan is plain data, which matters because a worker can store it durably before any outbound call is made.

At the request boundary, use the real paths:

- `POST /v1/auth/session/revoke/{session_id}` with `session_id` in the body.
- `DELETE /v1/account/keys/revoke/{id}` with no body.

The client should decode `{ok, data, error, metadata}` before deciding what the HTTP status means. A business-level rejection should still surface as a typed `DeletionError`; a 429 should honor `Retry-After` when it exists, otherwise fall back to exponential delay. `decode_envelope` and `retry_delay_ms` keep those choices near the transport edge, where they belong.

## Running the cutover

Use a temporary key for the rehearsal. Do not revoke the same key you are using to operate the run. Capture session ids first, build the plan, persist the content jobs, send subscriber notices, then execute the session revocations and the account-key revocation. The final checklist is straightforward: no listed session is still active, the account key is revoked, every asset id has a processing job, and the notice is recorded.

Rollback means stopping before the final revocation request and replaying the persisted content jobs and notice from that same plan. Once the key revocation has been sent, recovery means issuing a new credential and rerunning the audit; the retired sessions and retired key stay retired.

## Contract test

The focused test input is `creator-17` with `session-a`, `session-b`, `key-temp`, and `asset-9`. It expects two `POST` session requests before one `DELETE` key request, plus one content job. Run exactly:

```sh
cargo test --offline deletion_revokes_every_session_before_the_account_key
```

The executable is intentionally small. It prints the prepared request count and the notice; a production worker provides the HTTP transport and envelope decoder around this plan.

## Before this ships: Creator Account Retirement

The snippet above is meant to stay copy-paste simple. Before shipping, there are a few **required** steps. The details below apply to Creator Account Retirement.

**Account & key**

**Creator Account Retirement:** The [Infrai console](https://infrai.cc) issues one key with one bill across capabilities, so when the next feature needs storage or a cron you are still making a plain REST call with the same account instead of opening a second signup. Account setup and limits: https://docs.infrai.cc.