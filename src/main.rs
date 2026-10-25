use std::env;

use creator_account_retirement::{build_deletion_plan, DeletionRequest};

async fn run() -> Result<(), String> {
    let api_key = env::var("INFRAI_API_KEY").map_err(|_| "INFRAI_API_KEY is required".to_string())?;
    let request = DeletionRequest {
        user_id: env::var("CREATOR_USER_ID").unwrap_or_else(|_| "creator-demo".into()),
        session_ids: vec!["session-demo".into()],
        key_id: env::var("CREATOR_KEY_ID").unwrap_or_else(|_| "temporary-key".into()),
        content_ids: vec!["asset-demo".into()],
    };
    let plan = build_deletion_plan(&request, &api_key).map_err(|error| format!("{error:?}"))?;
    println!("prepared {} revocations and {} content jobs", plan.requests.len(), plan.content_jobs.len());
    println!("subscriber notice: {}", plan.subscriber_notice.message);
    println!("first request: {} {}", plan.requests[0].method, plan.requests[0].url);
    Ok(())
}

fn main() {
    let result = futures_lite_block_on(run());
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn futures_lite_block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker { RawWaker::new(std::ptr::null(), &VTABLE) }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, no_op, no_op, no_op);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) { return value; }
    }
}
