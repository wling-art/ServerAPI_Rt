use rand::Rng;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

lazy_static::lazy_static! {
    static ref DAILY_SENTENCE_CACHE: Arc<RwLock<Option<(serde_json::Value, i64)>>> =
        Arc::new(RwLock::new(None));
    static ref SENTENCE_QUEUE: Arc<Mutex<Vec<Value>>> =
        Arc::new(Mutex::new(Vec::new()));
}

const QUEUE_SIZE: usize = 10; // 队列大小

pub async fn maintain_sentence_queue() {
    tokio::spawn(async move {
        loop {
            // 补充队列
            refill_sentence_queue().await;

            // 检查间隔：每5秒检查一次队列状态
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });
}

/// 从队列中获取一个句子，如果没有则等待
pub async fn get_sentence_from_queue() -> Value {
    let mut queue = SENTENCE_QUEUE.lock().await;

    // 如果队列为空，等待补充
    while queue.is_empty() {
        drop(queue); // 释放锁
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        queue = SENTENCE_QUEUE.lock().await;
    }

    queue.remove(0) // 取出第一个句子
}

/// 向队列中添加句子
pub async fn add_sentence_to_queue(sentence: Value) {
    let mut queue = SENTENCE_QUEUE.lock().await;
    if queue.len() < QUEUE_SIZE {
        queue.push(sentence);
    }
}

/// 补充队列到指定大小
pub async fn refill_sentence_queue() {
    let current_size = {
        let queue = SENTENCE_QUEUE.lock().await;
        queue.len()
    };

    let needed = QUEUE_SIZE.saturating_sub(current_size);

    for _ in 0..needed {
        match asentence().await {
            Ok(sentence) => {
                add_sentence_to_queue(sentence).await;
            }
            Err(e) => {
                tracing::warn!("获取句子失败: {}, 使用默认数据", e);
                let default_sentence = serde_json::json!({
                    "hitokoto": "历史的每一天都值得被铭记",
                    "from": "未知",
                    "from_who": null
                });
                add_sentence_to_queue(default_sentence).await;
            }
        }
    }
}

/// 获取一言
pub async fn asentence() -> Result<Value, reqwest::Error> {
    let client = Client::new();
    let resp = client
        .get("https://international.v1.hitokoto.cn")
        .send()
        .await?;
    let data = resp.json::<Value>().await?;
    Ok(data)
}

/// 生成验证码
pub fn generate_verification_code() -> String {
    let mut rng = rand::rng();
    (0..6)
        .map(|_| rng.random_range(0..10).to_string())
        .collect()
}
