use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::{Validate, ValidationError};
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthToken {
    /// JWT 访问令牌
    #[schema(
        example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyLCJleHAiOjE1MTYyNDI2MjJ9.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
    )]
    pub access_token: String,
    /// 过期时间
    #[schema(example = 2592000)]
    pub expires_in: u64,
}

/// 用户登录请求数据结构体
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserLoginData {
    /// 用户名或邮箱（识别是否包含 @ 判断是否有邮箱）
    #[schema(example = "user123")]
    #[schema(example = "user@example.com")]
    pub username_or_email: String,
    /// 密码
    #[schema(example = "Password123")]
    pub password: String,
}

fn validate_password_complexity(password: &str) -> Result<(), ValidationError> {
    let has_letter = password.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());

    if has_letter && has_digit {
        Ok(()) // 验证通过
    } else {
        Err(ValidationError::new("密码必须同时包含字母和数字"))
    }
}

#[derive(Debug, Clone, Serialize, Validate, Deserialize, ToSchema)]
pub struct UserRegisterData {
    /// 邮箱
    #[validate(email(message = "邮箱格式不正确"))]
    #[schema(example = "user@example.com")]
    pub email: String,

    /// 密码(长度在 8 到 32 个字符之间，必须包含字母和数字)
    #[validate(length(min = 8, max = 32, message = "密码长度必须在 8 到 32 个字符之间"))]
    #[validate(custom(function = "validate_password_complexity"))]
    #[schema(example = "Password123")]
    pub password: String,

    /// 用户名(长度在 3 到 20 个字符之间，只能包含字母、数字和下划线)
    #[validate(length(min = 3, max = 20, message = "用户名长度必须在 3 到 20 个字符之间"))]
    #[validate(regex(path = "*USERNAME_REGEX", message = "用户名只能包含字母、数字和下划线"))]
    #[schema(example = "user123")]
    pub username: String,
}

// register by email
#[derive(Debug, Clone, Serialize, Validate, Deserialize, ToSchema)]
pub struct UserRegisterByEmailData {
    /// 邮箱
    #[validate(email(message = "邮箱格式不正确"))]
    #[schema(example = "user@example.com")]
    pub email: String,
}

pub static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_]+$").unwrap());
