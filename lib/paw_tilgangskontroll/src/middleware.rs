use crate::policy::Policy;
use axum::extract::{Request, State};
use axum::middleware::{FromFnLayer, Next, from_fn_with_state};
use axum::response::Response;
use paw_error_handling::problem_details::ProblemDetails;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub type TilgangskontrollLayer<P> = FromFnLayer<
    fn(State<Arc<P>>, Request, Next) -> BoxedFuture<Result<Response, ProblemDetails>>,
    Arc<P>,
    (State<Arc<P>>, Request),
>;

pub fn tilgangskontroll<P: Policy>(policy: Arc<P>) -> TilgangskontrollLayer<P> {
    from_fn_with_state(
        policy,
        tilgangskontroll_middleware_boxed::<P>
            as fn(State<Arc<P>>, Request, Next) -> BoxedFuture<Result<Response, ProblemDetails>>,
    )
}

fn tilgangskontroll_middleware_boxed<P: Policy>(
    state: State<Arc<P>>,
    request: Request,
    next: Next,
) -> BoxedFuture<Result<Response, ProblemDetails>> {
    Box::pin(tilgangskontroll_middleware(state, request, next))
}

#[tracing::instrument(skip_all)]
async fn tilgangskontroll_middleware<P: Policy>(
    State(policy): State<Arc<P>>,
    request: Request,
    next: Next,
) -> Result<Response, ProblemDetails> {
    let start = std::time::Instant::now();
    let result = policy.evaluate(request).await;
    tracing::trace!(
        elapsed_ms = start.elapsed().as_millis() as u64,
        "Policy-evaluering fullført"
    );
    match result {
        Ok(request) => Ok(next.run(request).await),
        Err(error) => Err(error),
    }
}
