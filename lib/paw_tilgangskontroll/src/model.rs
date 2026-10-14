use axum::extract::Request;
use paw_error_handling::problem_details::ProblemDetails;
use std::future::Future;

pub trait Policy: Send + Sync + 'static {
    fn evaluate(
        &self,
        request: Request,
    ) -> impl Future<Output = Result<Request, ProblemDetails>> + Send;
}
