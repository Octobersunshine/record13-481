use axum::{
    extract::{State, Path, Query},
    Json,
    http::StatusCode,
};
use crate::db::Database;
use crate::models::{
    ApiResponse, DeviceMetric, MetricListResponse, MetricRequest,
};
use serde::Deserialize;
use std::sync::Arc;

pub type AppState = Arc<Database>;

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

pub async fn submit_metric(
    State(state): State<AppState>,
    Json(metric): Json<MetricRequest>,
) -> (StatusCode, Json<ApiResponse<DeviceMetric>>) {
    if let Err(errors) = metric.validate() {
        let err_msg = errors.iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect::<Vec<_>>()
            .join("; ");
        tracing::warn!("指标校验失败 [device_id={}]: {}", metric.device_id, err_msg);
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::validation_error("指标数据校验失败，存在异常脏数据", errors)),
        );
    }

    match state.insert_metric(&metric) {
        Ok(saved) => (
            StatusCode::OK,
            Json(ApiResponse::ok_with_message("指标上报成功", saved)),
        ),
        Err(e) => {
            tracing::error!("保存指标失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("保存指标失败")),
            )
        }
    }
}

pub async fn get_device_metrics(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> (StatusCode, Json<ApiResponse<MetricListResponse>>) {
    match state
        .get_metrics_by_device(&device_id, params.limit, params.offset)
    {
        Ok((metrics, total)) => (
            StatusCode::OK,
            Json(ApiResponse::ok(MetricListResponse { metrics, total })),
        ),
        Err(e) => {
            tracing::error!("查询设备指标失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("查询失败")),
            )
        }
    }
}

pub async fn get_all_metrics(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> (StatusCode, Json<ApiResponse<MetricListResponse>>) {
    match state.get_all_metrics(params.limit, params.offset) {
        Ok((metrics, total)) => (
            StatusCode::OK,
            Json(ApiResponse::ok(MetricListResponse { metrics, total })),
        ),
        Err(e) => {
            tracing::error!("查询所有指标失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("查询失败")),
            )
        }
    }
}

pub async fn get_latest_metric(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<DeviceMetric>>) {
    match state.get_latest_metric(&device_id) {
        Ok(Some(metric)) => (
            StatusCode::OK,
            Json(ApiResponse::ok(metric)),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("未找到该设备的指标数据")),
        ),
        Err(e) => {
            tracing::error!("查询最新指标失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("查询失败")),
            )
        }
    }
}

pub async fn health_check() -> (StatusCode, Json<ApiResponse<String>>) {
    (
        StatusCode::OK,
        Json(ApiResponse::ok("服务运行正常".to_string())),
    )
}
