use colored::*;
use console::measure_text_width;
use std::fmt;
use tracing::Level;
use tracing_subscriber::{
    fmt::{FmtContext, FormatEvent, FormatFields},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

pub struct CleanFormatter;

impl<S, N> FormatEvent<S, N> for CleanFormatter
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        let now = chrono::Local::now();
        let timestamp = now.format("%H:%M:%S").to_string();

        let level = *event.metadata().level();
        let level_str = format_level(&level);

        let target = event.metadata().target();
        let module = extract_module_name(target);

        // 计算各字段的实际显示宽度
        let timestamp_width = measure_text_width(&timestamp);
        let level_width = measure_text_width(&level_str.to_string());
        let module_width = measure_text_width(&module);

        // 固定宽度设置
        const TIMESTAMP_WIDTH: usize = 8; // HH:MM:SS
        const LEVEL_WIDTH: usize = 5; // ERROR, WARN , etc.
        const MODULE_WIDTH: usize = 5; // 可根据需要调整

        // 计算需要的填充空格
        let timestamp_padding = TIMESTAMP_WIDTH.saturating_sub(timestamp_width);
        let level_padding = LEVEL_WIDTH.saturating_sub(level_width);
        let module_padding = MODULE_WIDTH.saturating_sub(module_width);

        write!(
            writer,
            "{}{} {}{} {}{} {} ",
            timestamp.bright_black(),
            " ".repeat(timestamp_padding),
            level_str,
            " ".repeat(level_padding),
            module.bright_blue(),
            " ".repeat(module_padding),
            "│".bright_black()
        )?;

        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

pub struct HttpLogFormatter;

impl HttpLogFormatter {
    pub fn format_request(
        method: &str,
        uri: &str,
        status: u16,
        duration: std::time::Duration,
        remote_addr: Option<&str>,
    ) -> String {
        let method_colored = format_method(method);
        let status_colored = format_status(status);
        let duration_colored = format_duration(duration);
        let remote_info = format_remote_addr(remote_addr);

        format!(
            "{} {} {} {} {}",
            status_colored,
            duration_colored,
            method_colored,
            uri.bright_white(),
            remote_info
        )
    }
}

fn format_level(level: &Level) -> ColoredString {
    match *level {
        Level::ERROR => "ERROR".bright_red(),
        Level::WARN => "WARN ".bright_yellow(),
        Level::INFO => "INFO ".bright_green(),
        Level::DEBUG => "DEBUG".bright_blue(),
        Level::TRACE => "TRACE".bright_magenta(),
    }
}

fn extract_module_name(target: &str) -> String {
    let module = if target.starts_with("server_api_rt") {
        let stripped = target.strip_prefix("server_api_rt::").unwrap_or(target);
        if stripped == "server_api_rt" {
            "main"
        } else {
            stripped.split("::").last().unwrap_or("app")
        }
    } else {
        target.split("::").last().unwrap_or(target)
    };

    // 限制模块名最大长度
    const MAX_MODULE_LEN: usize = 8;
    if module.len() > MAX_MODULE_LEN {
        format!("{:.len$}", module, len = MAX_MODULE_LEN - 2) + ".."
    } else {
        format!("{module:<MAX_MODULE_LEN$}")
    }
}

fn format_method(method: &str) -> ColoredString {
    let padded = format!("{method:<6}");
    match method {
        "GET" => padded.bright_green(),
        "POST" => padded.bright_blue(),
        "PUT" => padded.bright_yellow(),
        "DELETE" => padded.bright_red(),
        "PATCH" => padded.bright_magenta(),
        _ => padded.bright_white(),
    }
}

fn format_status(status: u16) -> ColoredString {
    match status {
        200..=299 => status.to_string().bright_green(),
        300..=399 => status.to_string().bright_yellow(),
        400..=499 => status.to_string().bright_red(),
        500..=599 => status.to_string().on_red().bright_white(),
        _ => status.to_string().bright_white(),
    }
}

fn format_duration(duration: std::time::Duration) -> ColoredString {
    let duration_ms = duration.as_secs_f64() * 1000.0;
    let duration_str = format!("{duration_ms:>7.1}ms");

    if duration_ms < 100.0 {
        duration_str.bright_green()
    } else if duration_ms < 500.0 {
        duration_str.bright_yellow()
    } else {
        duration_str.bright_red()
    }
}

fn format_remote_addr(remote_addr: Option<&str>) -> ColoredString {
    match remote_addr {
        Some(addr) => format!("from {addr}").bright_black(),
        None => "".normal(),
    }
}

pub fn init_logging() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().or_else(|_| {
        EnvFilter::try_new(
            "info,sqlx=warn,sqlx::query=off,sea_orm=warn,sea_orm_migration=warn,hyper=warn,tower=warn,tower_http=warn,axum=warn,h2=warn,mio=warn,want=warn,tokio_util=warn"
        )
    })?;

    let use_colors = atty::is(atty::Stream::Stdout);

    if use_colors {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .event_format(CleanFormatter)
                    .with_ansi(true),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().with_ansi(false))
            .init();
    }

    Ok(())
}

pub fn log_startup_info(config: &crate::config::Config) {
    println!();
    println!("{}", "─".repeat(60).bright_cyan());
    println!("{}", "  🚀 Server API 启动中...".bright_cyan().bold());
    println!("{}", "─".repeat(60).bright_cyan());

    tracing::info!("配置加载成功");
    tracing::info!("数据库: {}", mask_database_url(&config.database.url));
    tracing::info!("服务器: {}:{}", config.server.host, config.server.port);
    tracing::info!("Redis: {}:{}", config.redis.host, config.redis.port);
    tracing::info!("JWT: 已配置");
}

pub fn log_server_ready(addr: &std::net::SocketAddr) {
    println!("{}", "─".repeat(60).bright_green());
    println!("{}", "  ✅ 服务器启动完成".bright_green().bold());
    println!("{}", "─".repeat(60).bright_green());
    println!(
        "  🌐 服务地址: {}",
        format!("http://{addr}").bright_white().underline()
    );
    println!(
        "  ❤️  健康检查: {}",
        format!("http://{addr}/health").bright_green().underline()
    );
    println!(
        "  📚 API 文档: {}",
        format!("http://{addr}/docs").bright_blue().underline()
    );
    println!("{}", "─".repeat(60).bright_green());
    println!();

    tracing::info!("服务器监听地址: {}", addr);
}

fn mask_database_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        let mut masked = parsed.clone();
        if parsed.password().is_some() {
            let _ = masked.set_password(Some("****"));
        }
        masked.to_string()
    } else {
        "***已屏蔽***".to_string()
    }
}

pub fn log_shutdown() {
    println!();
    println!("{}", "─".repeat(60).bright_yellow());
    println!("{}", "  👋 服务器关闭中...".bright_yellow().bold());
    println!("{}", "─".repeat(60).bright_yellow());
    tracing::info!("服务器关闭完成");
}

#[macro_export]
macro_rules! log_http_request {
    ($method:expr, $uri:expr, $status:expr, $duration:expr) => {
        tracing::info!(
            "{}",
            $crate::logging::HttpLogFormatter::format_request(
                $method, $uri, $status, $duration, None
            )
        );
    };
    ($method:expr, $uri:expr, $status:expr, $duration:expr, $remote:expr) => {
        tracing::info!(
            "{}",
            $crate::logging::HttpLogFormatter::format_request(
                $method,
                $uri,
                $status,
                $duration,
                Some($remote)
            )
        );
    };
}
