use anyhow::{Result, bail};
use sled::Db;
use crate::models::{
    DeviceMetric, MetricRequest, HourlyAggregation,
    TEMPERATURE_MIN, TEMPERATURE_MAX, VOLTAGE_MIN, VOLTAGE_MAX,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use uuid::Uuid;
use std::sync::Arc;
use std::collections::HashMap;

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

    pub fn get_hourly_aggregation(
        &self,
        device_id: &str,
        hours: i64,
    ) -> Result<Vec<HourlyAggregation>> {
        let hours = hours.max(1).min(720);
        let now = Utc::now();
        let start_time = now - Duration::hours(hours);

        let prefix = format!("device:{}:", device_id);
        let mut metrics = Vec::new();

        for item in self.db.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item?;
            let id_str = String::from_utf8_lossy(&value);
            let metric_key = format!("metric:{}", id_str);

            if let Some(metric_bytes) = self.db.get(metric_key.as_bytes())? {
                let metric: DeviceMetric = serde_json::from_slice(&metric_bytes)?;
                if metric.timestamp >= start_time {
                    metrics.push(metric);
                }
            }
        }

        let mut hour_groups: HashMap<i64, Vec<&DeviceMetric>> = HashMap::new();

        for metric in &metrics {
            let hour_key = metric.timestamp.timestamp() / 3600;
            hour_groups.entry(hour_key).or_default().push(metric);
        }

        let mut aggregations: Vec<HourlyAggregation> = Vec::new();

        for (hour_key, group_metrics) in hour_groups {
            let hour_start_timestamp = hour_key * 3600;
            let hour = Utc.timestamp_opt(hour_start_timestamp, 0).single()
                .unwrap_or_else(|| Utc.timestamp_opt(hour_start_timestamp, 0).unwrap());

            let mut temp_sum = 0.0;
            let mut temp_count = 0;
            let mut temp_min = None;
            let mut temp_max = None;

            let mut volt_sum = 0.0;
            let mut volt_count = 0;
            let mut volt_min = None;
            let mut volt_max = None;

            for metric in &group_metrics {
                if let Some(temp) = metric.temperature {
                    temp_sum += temp;
                    temp_count += 1;
                    temp_min = Some(temp_min.map_or(temp, |m: f64| m.min(temp)));
                    temp_max = Some(temp_max.map_or(temp, |m: f64| m.max(temp)));
                }
                if let Some(volt) = metric.voltage {
                    volt_sum += volt;
                    volt_count += 1;
                    volt_min = Some(volt_min.map_or(volt, |m: f64| m.min(volt)));
                    volt_max = Some(volt_max.map_or(volt, |m: f64| m.max(volt)));
                }
            }

            let avg_temperature = if temp_count > 0 {
                Some(temp_sum / temp_count as f64)
            } else {
                None
            };

            let avg_voltage = if volt_count > 0 {
                Some(volt_sum / volt_count as f64)
            } else {
                None
            };

            aggregations.push(HourlyAggregation {
                hour,
                avg_temperature,
                avg_voltage,
                min_temperature: temp_min,
                max_temperature: temp_max,
                min_voltage: volt_min,
                max_voltage: volt_max,
                sample_count: group_metrics.len() as i64,
            });
        }

        aggregations.sort_by(|a, b| a.hour.cmp(&b.hour));

        Ok(aggregations)
    }
}
