use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;
// Temporarily disabled rate limiting due to complex configuration
// use tower_governor::{
//     governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
// };
use tracing::info_span;
use uuid::Uuid;

pub async fn trace_request(mut request: Request, next: Next) -> Response {
    let start = Instant::now();
    
    let request_id = Uuid::new_v4().to_string();
    
    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();
    
    request.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid"))
    );

    let span = info_span!(
        "request",
        method = %method,
        uri = %uri,
        version = ?version,
        request_id = %request_id,
    );

    let _enter = span.enter();

    let response = next.run(request).await;
    
    let latency = start.elapsed();
    let status = response.status();
    
    tracing::info!(
        latency_ms = latency.as_millis(),
        status = %status,
        "Request completed"
    );

    response
}

// Temporarily disabled rate limiting due to complex configuration
// pub fn rate_limiting_layer() -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware> {
//     let governor_conf = GovernorConfigBuilder::default()
//         .per_second(30)
//         .burst_size(60)
//         .finish()
//         .unwrap();

//     GovernorLayer {
//         config: std::sync::Arc::new(governor_conf),
//     }
// }