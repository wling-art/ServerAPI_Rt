use colored::*;
use std::fmt;
use tracing::Level;
use tracing_subscriber::{
    fmt::{ FmtContext, FormatEvent, FormatFields },
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// 自定义格式化器，类似于 Gin 框架的日志格式
pub struct GinLikeFormatter;

impl<S, N> FormatEvent<S, N>
    for GinLikeFormatter
    where
        S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
        N: for<'a> FormatFields<'a> + 'static
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>
    ) -> fmt::Result {
        // 本地时区
        let now = chrono::Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

        // 获取日志级别
        let level = *event.metadata().level();
        let level_str = format_level(&level);

        // 获取模块路径
        let target = event.metadata().target();
        let module = if target.starts_with("server_api_rt") {
            target.strip_prefix("server_api_rt::").unwrap_or(target)
        } else {
            target
        };

        // 格式化输出类似于 Gin: [GIN] 2023/12/25 - 15:30:45 | 200 | 123.456ms | 192.168.1.1 | GET /api/users
        write!(
            writer,
            "{} {} {} {} ",
            "[SERVER]".bright_cyan().bold(),
            timestamp.bright_black(),
            "|".bright_black(),
            level_str
        )?;

        if !module.is_empty() && module != "server_api_rt" {
            write!(writer, "{} {} ", "|".bright_black(), module.bright_black())?;
        }

        // 格式化事件消息
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

/// HTTP 请求日志格式化器
pub struct HttpLogFormatter;

impl HttpLogFormatter {
    pub fn format_request(
        method: &str,
        uri: &str,
        status: u16,
        duration: std::time::Duration,
        remote_addr: Option<&str>
    ) -> String {
        let method_colored = match method {
            "GET" => method.bright_green().bold(),
            "POST" => method.bright_blue().bold(),
            "PUT" => method.bright_yellow().bold(),
            "DELETE" => method.bright_red().bold(),
            "PATCH" => method.bright_magenta().bold(),
            _ => method.bright_white().bold(),
        };

        let status_colored = match status {
            200..=299 => status.to_string().bright_green().bold(),
            300..=399 => status.to_string().bright_yellow().bold(),
            400..=499 => status.to_string().bright_red().bold(),
            500..=599 => status.to_string().on_bright_red().bright_white().bold(),
            _ => status.to_string().bright_white().bold(),
        };

        let duration_ms = duration.as_secs_f64() * 1000.0;
        let duration_colored = if duration_ms < 100.0 {
            format!("{:.2}ms", duration_ms).bright_green()
        } else if duration_ms < 500.0 {
            format!("{:.2}ms", duration_ms).bright_yellow()
        } else {
            format!("{:.2}ms", duration_ms).bright_red()
        };

        let remote_display = match remote_addr {
            Some(addr) => format!("from {}", addr.bright_blue()),
            None => "".to_string(),
        };

        format!(
            "{} {} {} {} {} {} {}",
            "[HTTP]".bright_cyan().bold(),
            "|".bright_black(),
            status_colored,
            "|".bright_black(),
            duration_colored,
            "|".bright_black(),
            format!("{} {} {}", method_colored, uri.bright_white(), remote_display)
        )
    }
}

fn format_level(level: &Level) -> ColoredString {
    match *level {
        Level::ERROR => "ERROR".bright_red().bold(),
        Level::WARN => "WARN ".bright_yellow().bold(),
        Level::INFO => "INFO ".bright_green().bold(),
        Level::DEBUG => "DEBUG".bright_blue().bold(),
        Level::TRACE => "TRACE".bright_magenta().bold(),
    }
}

/// 初始化日志系统
pub fn init_logging() -> anyhow::Result<()> {
    // 配置日志过滤器，隐藏不必要的详细日志
    let env_filter = EnvFilter::try_from_default_env().or_else(|_| {
        EnvFilter::try_new(
            "info,sqlx=warn,sqlx::query=off,sea_orm=warn,sea_orm_migration=warn,hyper=warn,tower=warn,tower_http=warn,axum=warn,h2=warn,mio=warn,want=warn,tokio_util=warn"
        )
    })?;

    // 检查是否为 TTY（终端），如果是则启用彩色输出
    let use_colors = atty::is(atty::Stream::Stdout);

    if use_colors {
        // 着色
        tracing_subscriber
            ::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().event_format(GinLikeFormatter).with_ansi(true))
            .init();
    } else {
        tracing_subscriber
            ::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().with_ansi(false))
            .init();
    }

    Ok(())
}

/// 应用启动日志
pub fn log_startup_info(config: &crate::config::Config) {
    println!();
    println!("{}", "🚀 Server API Starting...".bright_cyan().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());

    tracing::info!("🔧 Configuration loaded successfully");
    tracing::info!("🗄️  Database: {}", mask_database_url(&config.database.url));
    tracing::info!("🌐 Server: {}:{}", config.server.host, config.server.port);
    tracing::info!("� Redis: {}:{}", config.redis.host, config.redis.port);
    tracing::info!("�🔐 JWT: Configured");

    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
}

/// 服务器启动完成日志
pub fn log_server_ready(addr: &std::net::SocketAddr) {
    println!();
    println!("{}", "✅ Server is ready!".bright_green().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
    println!("🏠 Server:      {}", format!("http://{}", addr).bright_white().underline());
    println!("❤️  Health:     {}", format!("http://{}/health", addr).bright_white().underline());
    println!(
        "📖 API Docs:    {}",
        format!("http://{}/docs", addr).bright_white().underline()
    );
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
    println!();

    tracing::info!("🎉 Server listening on {}", addr);
}

/// 屏蔽数据库 URL 中的敏感信息
fn mask_database_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        let mut masked = parsed.clone();
        if parsed.password().is_some() {
            let _ = masked.set_password(Some("****"));
        }
        masked.to_string()
    } else {
        "***masked***".to_string()
    }
}

/// 应用关闭日志
pub fn log_shutdown() {
    println!();
    println!("{}", "👋 Server shutting down...".bright_yellow().bold());
    tracing::info!("Server shutdown completed");
}

// 宏定义，用于简化 HTTP 日志记录
#[macro_export]
macro_rules! log_http_request {
    ($method:expr, $uri:expr, $status:expr, $duration:expr) => {
        tracing::info!(
            "{}",
            $crate::logging::HttpLogFormatter::format_request(
                $method,
                $uri,
                $status,
                $duration,
                None
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
