use axum::{
    extract::{ConnectInfo, Request},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use std::{net::SocketAddr, time::Instant};

use crate::logging::HttpLogFormatter;

/// 获取真实的客户端 IP 地址
fn get_real_ip(addr: Option<SocketAddr>, headers: &HeaderMap) -> Option<String> {
    // 首先尝试从常见的代理头中获取IP
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(header_value) = forwarded_for.to_str() {
            // X-Forwarded-For 可能包含多个IP，取第一个
            if let Some(first_ip) = header_value.split(',').next() {
                let ip = first_ip.trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
    }

    // 尝试 X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(header_value) = real_ip.to_str() {
            let ip = header_value.trim();
            if !ip.is_empty() {
                return Some(ip.to_string());
            }
        }
    }

    // 最后使用连接的地址
    addr.map(|a| a.to_string())
}

/// HTTP 请求日志中间件
pub async fn http_logging_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let headers = request.headers().clone();

    // 获取真实的客户端IP
    let real_ip = get_real_ip(Some(addr), &headers);

    // 处理请求
    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status().as_u16();

    // 记录 HTTP 请求日志
    let log_message =
        HttpLogFormatter::format_request(&method, &uri, status, duration, real_ip.as_deref());

    tracing::info!("{}", log_message);

    response
}

/// 简化版本的 HTTP 日志中间件（不需要 ConnectInfo）
pub async fn simple_http_logging_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let headers = request.headers().clone();

    // 尝试从头部获取真实IP
    let real_ip = get_real_ip(None, &headers);

    // 处理请求
    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status().as_u16();

    // 记录 HTTP 请求日志
    let log_message =
        HttpLogFormatter::format_request(&method, &uri, status, duration, real_ip.as_deref());

    tracing::info!("{}", log_message);

    response
}
