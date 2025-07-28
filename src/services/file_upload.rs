use anyhow::Result;
use image::{GenericImageView, ImageFormat};
use reqwest::Client as HttpClient;
use rusty_s3::{Bucket, Credentials, S3Action, UrlStyle};
use sea_orm::*;
use std::io::Cursor;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    config::S3Config, entities::files, errors::{ApiError, ApiResult}, services::database::DatabaseConnection
};

pub struct FileUploadService;

impl FileUploadService {
    /// 创建 S3 客户端配置
    pub fn create_s3_credentials(s3_config: &S3Config) -> Credentials {
        Credentials::new(&s3_config.access_key, &s3_config.secret_key)
    }

    /// 创建 S3 Bucket 实例
    pub fn create_s3_bucket(s3_config: &S3Config) -> Result<Bucket> {
        let endpoint = url::Url::parse(&s3_config.endpoint_url)?;
        let bucket = Bucket::new(
            endpoint,
            UrlStyle::VirtualHost,
            s3_config.bucket.clone(),
            "noting".to_string(),
        )?;
        Ok(bucket)
    }

    /// 获取文件扩展名
    /// 对于复合扩展名（如 .backup.tar.gz），会返回完整的扩展名部分
    pub fn get_file_extension(filename: &str) -> String {
        // 特殊处理已知的复合扩展名模式
        if filename.contains(".backup.tar.gz") {
            return ".backup.tar.gz".to_string();
        }
        if let Some(pos) = filename.find(".backup.tar") {
            return filename[pos..].to_string();
        }
        if filename.contains(".tar.gz") {
            return ".tar.gz".to_string();
        }
        if filename.contains(".tar.bz2") {
            return ".tar.bz2".to_string();
        }
        if filename.contains(".tar.xz") {
            return ".tar.xz".to_string();
        }
        // 默认返回最后的扩展名
        if let Some(last_dot_pos) = filename.rfind('.') {
            ToString::to_string(&filename[last_dot_pos..])
        } else {
            String::new()
        }
    }

    /// 验证图片格式和比例
    pub fn validate_image(content: &[u8]) -> ApiResult<(u32, u32)> {
        // 检查文件大小（5MB 限制）
        if content.len() > 5 * 1024 * 1024 {
            return Err(ApiError::BadRequest(
                "图片文件大小不能超过 5 MB".to_string(),
            ));
        }

        // 尝试打开图片
        let img = image::load_from_memory(content)
            .map_err(|_| ApiError::BadRequest("图片文件无效".to_string()))?;

        // 检查图片格式
        let format = image::guess_format(content)
            .map_err(|_| ApiError::BadRequest("无法识别图片格式".to_string()))?;

        match format {
            ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP => {}
            _ => {
                return Err(ApiError::BadRequest("图片文件格式无效".to_string()));
            }
        }

        let (width, height) = img.dimensions();
        let expected_ratio = 16.0 / 9.0;
        let actual_ratio = (width as f64) / (height as f64);

        if (actual_ratio - expected_ratio).abs() > 0.01 {
            return Err(ApiError::BadRequest("图片比例最好为 512*300".to_string()));
        }

        Ok((width, height))
    }

    /// 将图片转换为 WebP 格式
    pub fn convert_to_webp(content: &[u8]) -> ApiResult<Vec<u8>> {
        let img = image::load_from_memory(content)
            .map_err(|_| ApiError::BadRequest("图片文件无效".to_string()))?;

        let mut webp_data = Vec::new();
        let mut cursor = Cursor::new(&mut webp_data);

        img.write_to(&mut cursor, ImageFormat::WebP)
            .map_err(|_| ApiError::Internal("图片格式转换失败".to_string()))?;

        Ok(webp_data)
    }

    /// 上传文件到 S3
    pub async fn upload_file_to_s3(
        db: &DatabaseConnection,
        s3_config: &S3Config,
        file_content: Vec<u8>,
        file_name: &str,
    ) -> ApiResult<(String, files::Model)> {
        let file_hash = files::Model::generate_file_hash(&file_content);
        let extension = Self::get_file_extension(file_name);
        let s3_object_name = format!("uploads/{}{}", Uuid::new_v4(), extension);

        // 检查文件是否已存在
        if let Some(existing_file) = files::Entity::find()
            .filter(files::Column::HashValue.eq(&file_hash))
            .one(db.as_ref())
            .await
            .map_err(|e| ApiError::Database(e.to_string()))?
        {
            return Ok((existing_file.file_path.clone(), existing_file));
        }

        // 创建 S3 配置
        let credentials = Self::create_s3_credentials(s3_config);
        let bucket = Self::create_s3_bucket(s3_config)
            .map_err(|e| ApiError::Internal(format!("S3 bucket 配置失败: {e}")))?;

        // 生成上传的预签名 URL
        let action = bucket.put_object(Some(&credentials), &s3_object_name);

        // 使用 HTTP 客户端上传文件
        let http_client = HttpClient::new();
        let response = http_client
            .put(action.sign(Duration::from_secs(3600)))
            .body(file_content.clone())
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("文件上传失败: {e}")))?;

        if !response.status().is_success() {
            return Err(ApiError::Internal(format!(
                "文件上传失败，状态码: {}",
                response.status()
            )));
        }

        // 保存文件信息到数据库
        let file_path = format!(
            "{}/{}/{}",
            s3_config.endpoint_url, s3_config.bucket, s3_object_name
        );
        let file_object = files::ActiveModel {
            hash_value: Set(file_hash),
            file_path: Set(file_path.clone()),
        };

        let created_file = files::Entity::insert(file_object)
            .exec_with_returning(db.as_ref())
            .await
            .map_err(|e| ApiError::Database(e.to_string()))?;

        Ok((file_path, created_file))
    }

    /// 验证并上传封面文件
    pub async fn validate_and_upload_cover(
        db: &DatabaseConnection,
        s3_config: &S3Config,
        content: Vec<u8>,
        _filename: &str,
    ) -> ApiResult<files::Model> {
        // 验证图片
        Self::validate_image(&content)?;

        // 转换为 WebP
        let webp_content = Self::convert_to_webp(&content)?;

        // 上传到 S3
        let (_url, file_model) =
            Self::upload_file_to_s3(db, s3_config, webp_content, "cover.webp").await?;

        Ok(file_model)
    }

    /// 验证并上传画册图片文件
    pub async fn validate_and_upload_gallery(
        db: &DatabaseConnection,
        s3_config: &S3Config,
        content: Vec<u8>,
        _filename: &str,
    ) -> ApiResult<files::Model> {
        // 检查文件大小（5MB 限制）
        if content.len() > 5 * 1024 * 1024 {
            return Err(ApiError::BadRequest(
                "图片文件大小不能超过 5 MB".to_string(),
            ));
        }

        // 尝试打开图片
        let _img = image::load_from_memory(&content)
            .map_err(|_| ApiError::BadRequest("图片文件无效".to_string()))?;

        // 检查图片格式
        let format = image::guess_format(&content)
            .map_err(|_| ApiError::BadRequest("无法识别图片格式".to_string()))?;

        match format {
            ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP => {}
            _ => {
                return Err(ApiError::BadRequest("图片文件格式无效".to_string()));
            }
        }

        // 转换为 WebP
        let webp_content = Self::convert_to_webp(&content)?;

        // 上传到 S3
        let (_url, file_model) =
            Self::upload_file_to_s3(db, s3_config, webp_content, "gallery.webp").await?;

        Ok(file_model)
    }

    /// 删除 S3 中的文件
    pub async fn delete_file(s3_config: &S3Config, hash_id: &str) -> ApiResult<()> {
        let credentials = Self::create_s3_credentials(s3_config);
        let bucket = Self::create_s3_bucket(s3_config)
            .map_err(|e| ApiError::Internal(format!("S3 配置错误: {e}")))?;

        let delete_action = bucket.delete_object(Some(&credentials), hash_id);
        let url = delete_action.sign(Duration::from_secs(60));

        let client = HttpClient::new();
        let response = client
            .delete(url.as_str())
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("删除文件失败: {e}")))?;

        if !response.status().is_success() {
            return Err(ApiError::Internal("删除 S3 文件失败".to_string()));
        }

        Ok(())
    }
}
