use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceMetric {
    pub id: Uuid,
    pub device_id: String,
    pub temperature: Option<f64>,
    pub voltage: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricRequest {
    pub device_id: String,
    pub temperature: Option<f64>,
    pub voltage: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            message: "操作成功".to_string(),
            data: Some(data),
        }
    }

    pub fn ok_with_message(message: &str, data: T) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            data: Some(data),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            data: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricListResponse {
    pub metrics: Vec<DeviceMetric>,
    pub total: i64,
}
