use anyhow::Result;
use askama::Template;
use chrono::{Datelike, Utc};

use crate::services::utils::{get_sentence_from_queue, refill_sentence_queue};

#[derive(Template)]
#[template(path = "email_code_verify.html")]
pub struct EmailTemplate {
    /// 验证码
    pub code: String,
    /// 今年的年份
    pub fullyear: String,
    /// 句子
    pub sentence: String,
    /// 句子的作者
    pub sentence_from: String,
    /// 句子的来源
    pub from_who: Option<String>,
}

pub async fn build_email_template(code: &str) -> Result<EmailTemplate> {
    let response = get_sentence_from_queue().await;
    tokio::spawn(async move {
        refill_sentence_queue().await;
    });

    let template = EmailTemplate {
        code: code.to_string(),
        fullyear: Utc::now().year().to_string(),
        sentence: response["hitokoto"]
            .as_str()
            .unwrap_or("历史的每一天都值得被铭记")
            .to_string(),
        sentence_from: response["from"].as_str().unwrap_or("未知").to_string(),
        from_who: response["from_who"].as_str().map(|s| s.to_string()),
    };
    Ok(template)
}
