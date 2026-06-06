use axum::body::Body;
use axum::http::header::{HeaderName, HeaderValue};
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

const PERMISSIONS_POLICY: &str = "geolocation=(), microphone=(self), camera=()";

const HEADERS: &[(&str, &str)] = &[
    ("x-content-type-options", "nosniff"),
    ("x-frame-options", "DENY"),
    ("referrer-policy", "strict-origin-when-cross-origin"),
    ("permissions-policy", PERMISSIONS_POLICY),
    // API responses serve JSON; the SvelteKit client serves its own CSP.
    // This policy denies everything by default so a malicious response
    // body can't be rendered as HTML by a misconfigured client.
    (
        "content-security-policy",
        "default-src 'none'; frame-ancestors 'none'; base-uri 'none'",
    ),
];

pub async fn security_headers_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    for (name, value) in HEADERS {
        let header_name = HeaderName::from_static(name);
        let header_value = HeaderValue::from_static(value);
        // Don't overwrite headers a handler explicitly set.
        if !headers.contains_key(&header_name) {
            headers.insert(header_name, header_value);
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_policy_allows_same_origin_microphone_for_voice() {
        assert_eq!(
            PERMISSIONS_POLICY,
            "geolocation=(), microphone=(self), camera=()"
        );
        assert!(!PERMISSIONS_POLICY.contains("microphone=()"));
    }
}
