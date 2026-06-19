use anyhow::{Result, bail};
use sled::Db;
use crate::models::{DeviceMetric, MetricRequest, TEMPERATURE_MIN, TEMPERATURE_MAX, VOLTAGE_MIN, VOLTAGE_MAX};
use chrono::Utc;
use uuid::Uuid;
use std::sync::Arc;

#[derive(Clone)]
pub struct Database {
    db: Arc<Db>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db: Arc::new(db) })
    }

    pub fn insert_metric(&self, request: &MetricRequest) -> Result<DeviceMetric> {
        if let Some(temp) = request.temperature {
            if temp.is_nan() || temp.is_infinite() {
                bail!("DB安全校验拦截: 温度值无效（NaN或无限大）: {}", temp);
            }
            if temp < TEMPERATURE_MIN || temp > TEMPERATURE_MAX {
                bail!(
                    "DB安全校验拦截: 温度值超出合理范围 [{}, {}]，当前值: {}",
                    TEMPERATURE_MIN, TEMPERATURE_MAX, temp
                );
            }
        }
        if let Some(volt) = request.voltage {
            if volt.is_nan() || volt.is_infinite() {
                bail!("DB安全校验拦截: 电压值无效（NaN或无限大）: {}", volt);
            }
            if volt < VOLTAGE_MIN || volt > VOLTAGE_MAX {
                bail!(
                    "DB安全校验拦截: 电压值超出合理范围 [{}, {}]，当前值: {}",
                    VOLTAGE_MIN, VOLTAGE_MAX, volt
                );
            }
        }

        let metric = DeviceMetric {
            id: Uuid::new_v4(),
            device_id: request.device_id.clone(),
            temperature: request.temperature,
            voltage: request.voltage,
            timestamp: Utc::now(),
        };

        let metric_json = serde_json::to_vec(&metric)?;

        let metric_key = format!("metric:{}", metric.id);
        let device_key = format!(
            "device:{}:{}:{}",
            metric.device_id,
            metric.timestamp.timestamp_millis(),
            metric.id
        );
        let all_key = format!(
            "all:{}:{}",
            metric.timestamp.timestamp_millis(),
            metric.id
        );

        self.db.insert(metric_key.as_bytes(), metric_json.as_slice())?;
        self.db.insert(device_key.as_bytes(), metric.id.as_bytes())?;
        self.db.insert(all_key.as_bytes(), metric.id.as_bytes())?;

        self.db.flush()?;

        Ok(metric)
    }

    pub fn get_metrics_by_device(
        &self,
        device_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<DeviceMetric>, i64)> {
        let prefix = format!("device:{}:", device_id);
        let mut all_ids = Vec::new();

        for item in self.db.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item?;
            all_ids.push(value.to_vec());
        }

        all_ids.reverse();

        let total = all_ids.len() as i64;

        let ids: Vec<_> = all_ids
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        let mut metrics = Vec::new();
        for id_bytes in ids {
            let id_str = String::from_utf8_lossy(&id_bytes);
            let metric_key = format!("metric:{}", id_str);

            if let Some(metric_bytes) = self.db.get(metric_key.as_bytes())? {
                let metric: DeviceMetric = serde_json::from_slice(&metric_bytes)?;
                metrics.push(metric);
            }
        }

        Ok((metrics, total))
    }

    pub fn get_all_metrics(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<DeviceMetric>, i64)> {
        let prefix = "all:";
        let mut all_ids = Vec::new();

        for item in self.db.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item?;
            all_ids.push(value.to_vec());
        }

        all_ids.reverse();

        let total = all_ids.len() as i64;

        let ids: Vec<_> = all_ids
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        let mut metrics = Vec::new();
        for id_bytes in ids {
            let id_str = String::from_utf8_lossy(&id_bytes);
            let metric_key = format!("metric:{}", id_str);

            if let Some(metric_bytes) = self.db.get(metric_key.as_bytes())? {
                let metric: DeviceMetric = serde_json::from_slice(&metric_bytes)?;
                metrics.push(metric);
            }
        }

        Ok((metrics, total))
    }

    pub fn get_latest_metric(&self, device_id: &str) -> Result<Option<DeviceMetric>> {
        let prefix = format!("device:{}:", device_id);
        let mut last_id: Option<Vec<u8>> = None;

        for item in self.db.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item?;
            last_id = Some(value.to_vec());
        }

        match last_id {
            Some(id_bytes) => {
                let id_str = String::from_utf8_lossy(&id_bytes);
                let metric_key = format!("metric:{}", id_str);

                if let Some(metric_bytes) = self.db.get(metric_key.as_bytes())? {
                    let metric: DeviceMetric = serde_json::from_slice(&metric_bytes)?;
                    Ok(Some(metric))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
}
