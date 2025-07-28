use crate::config::Config;
use anyhow::{Context, Result};
use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::Message;
use lettre::SmtpTransport;

/// 构建邮件消息
pub fn build_email_message(to_email: &str, from_email: &str, body: String) -> Result<Message> {
    Message::builder()
        .from(Mailbox::new(
            Some("MSCPO 验证系统".to_string()),
            from_email.parse().context("解析发件人邮箱地址失败")?,
        ))
        .to(to_email.parse().context("解析收件人邮箱地址失败")?)
        .subject("邮箱验证码")
        .header(ContentType::TEXT_HTML)
        .body(body)
        .context("构建邮件消息失败")
}

/// 构建SMTP传输对象
pub fn build_smtp_transport(config: &Config) -> Result<SmtpTransport> {
    let mut builder =
        SmtpTransport::relay(&config.email.smtp_server).context("Failed to create SMTP relay")?;
    builder = builder.port(config.email.smtp_port);
    Ok(builder
        .credentials(Credentials::new(
            config.email.smtp_username.clone(),
            config.email.smtp_password.clone(),
        ))
        .build())
}
