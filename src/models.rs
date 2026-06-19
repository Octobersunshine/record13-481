use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub const TEMPERATURE_MIN: f64 = -273.15;
pub const TEMPERATURE_MAX: f64 = 3000.0;

pub const VOLTAGE_MIN: f64 = -5000.0;
pub const VOLTAGE_MAX: f64 = 5000.0;

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
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl MetricRequest {
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        if self.device_id.trim().is_empty() {
            errors.push(ValidationError {
                field: "device_id".to_string(),
                message: "设备ID不能为空".to_string(),
            });
        }

        if self.temperature.is_none() && self.voltage.is_none() {
            errors.push(ValidationError {
                field: "temperature,voltage".to_string(),
                message: "温度和电压不能同时为空".to_string(),
            });
        }

        if let Some(temp) = self.temperature {
            if temp.is_nan() || temp.is_infinite() {
                errors.push(ValidationError {
                    field: "temperature".to_string(),
                    message: format!("温度值无效（NaN或无限大）: {}", temp),
                });
            } else if temp < TEMPERATURE_MIN || temp > TEMPERATURE_MAX {
                errors.push(ValidationError {
                    field: "temperature".to_string(),
                    message: format!(
                        "温度值超出合理范围 [{}, {}]，当前值: {}",
                        TEMPERATURE_MIN, TEMPERATURE_MAX, temp
                    ),
                });
            }
        }

        if let Some(volt) = self.voltage {
            if volt.is_nan() || volt.is_infinite() {
                errors.push(ValidationError {
                    field: "voltage".to_string(),
                    message: format!("电压值无效（NaN或无限大）: {}", volt),
                });
            } else if volt < VOLTAGE_MIN || volt > VOLTAGE_MAX {
                errors.push(ValidationError {
                    field: "voltage".to_string(),
                    message: format!(
                        "电压值超出合理范围 [{}, {}]，当前值: {}",
                        VOLTAGE_MIN, VOLTAGE_MAX, volt
                    ),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ValidationError>>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            message: "操作成功".to_string(),
            data: Some(data),
            errors: None,
        }
    }

    pub fn ok_with_message(message: &str, data: T) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            data: Some(data),
            errors: None,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            data: None,
            errors: None,
        }
    }

    pub fn validation_error(message: &str, errors: Vec<ValidationError>) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            data: None,
            errors: Some(errors),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricListResponse {
    pub metrics: Vec<DeviceMetric>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HourlyAggregation {
    pub hour: DateTime<Utc>,
    pub avg_temperature: Option<f64>,
    pub avg_voltage: Option<f64>,
    pub min_temperature: Option<f64>,
    pub max_temperature: Option<f64>,
    pub min_voltage: Option<f64>,
    pub max_voltage: Option<f64>,
    pub sample_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HourlyAggregationResponse {
    pub aggregations: Vec<HourlyAggregation>,
    pub device_id: String,
    pub total_hours: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AggregationQuery {
    pub hours: Option<i64>,
}

impl Default for AggregationQuery {
    fn default() -> Self {
        Self { hours: Some(24) }
    }
}
