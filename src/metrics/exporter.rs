use super::collector::MetricsSnapshot;
use serde_json;

pub struct MetricsExporter;

impl MetricsExporter {
    pub fn export_json(snapshot: &MetricsSnapshot) -> String {
        serde_json::to_string_pretty(snapshot).unwrap_or_default()
    }

    pub fn export_prometheus(snapshot: &MetricsSnapshot) -> String {
        format!(
            "# HELP ultrabalancer_requests_total Total number of requests\n\
             # TYPE ultrabalancer_requests_total counter\n\
             ultrabalancer_requests_total {}\n\
             \n\
             # HELP ultrabalancer_requests_success Successful requests\n\
             # TYPE ultrabalancer_requests_success counter\n\
             ultrabalancer_requests_success {}\n\
             \n\
             # HELP ultrabalancer_requests_failed Failed requests\n\
             # TYPE ultrabalancer_requests_failed counter\n\
             ultrabalancer_requests_failed {}\n\
             \n\
             # HELP ultrabalancer_response_time_ms Response time in milliseconds\n\
             # TYPE ultrabalancer_response_time_ms summary\n\
             ultrabalancer_response_time_ms{{quantile=\"0.5\"}} {}\n\
             ultrabalancer_response_time_ms{{quantile=\"0.95\"}} {}\n\
             ultrabalancer_response_time_ms{{quantile=\"0.99\"}} {}\n\
             ultrabalancer_response_time_ms_sum {}\n\
             ultrabalancer_response_time_ms_count {}\n\
             \n\
             # HELP ultrabalancer_uptime_seconds Uptime in seconds\n\
             # TYPE ultrabalancer_uptime_seconds counter\n\
             ultrabalancer_uptime_seconds {}\n\
             \n\
             # HELP ultrabalancer_requests_per_second Requests per second\n\
             # TYPE ultrabalancer_requests_per_second gauge\n\
             ultrabalancer_requests_per_second {}\n",
            snapshot.total_requests,
            snapshot.successful_requests,
            snapshot.failed_requests,
            snapshot.p50_response_time_ms,
            snapshot.p95_response_time_ms,
            snapshot.p99_response_time_ms,
            snapshot.avg_response_time_ms * snapshot.total_requests as f64,
            snapshot.total_requests,
            snapshot.uptime_seconds,
            snapshot.requests_per_second,
        )
    }
}
