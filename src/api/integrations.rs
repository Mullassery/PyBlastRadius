use serde::{Deserialize, Serialize};

// Slack/PagerDuty clients.
//
// Outbound dispatch is real when the default `http-discovery` feature
// (which enables `reqwest`) is on. Without that feature, calls return a
// configuration error instead of silently succeeding.

#[cfg(feature = "http-discovery")]
async fn post_json(
    url: &str,
    payload: &serde_json::Value,
) -> crate::error::Result<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| crate::error::Error::HttpError(format!("client init failed: {e}")))?;
    let response = client.post(url).json(payload).send().await.map_err(|e| {
        let mut message = format!("request failed: {e}");
        let mut source = std::error::Error::source(&e);
        while let Some(cause) = source {
            message.push_str(&format!(": {cause}"));
            source = cause.source();
        }
        crate::error::Error::HttpError(message)
    })?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(crate::error::Error::HttpError(format!(
            "webhook returned HTTP {status}: {body}"
        )));
    }
    Ok(serde_json::from_str(&body).unwrap_or(serde_json::Value::Null))
}

#[cfg(not(feature = "http-discovery"))]
async fn post_json(
    _url: &str,
    _payload: &serde_json::Value,
) -> crate::error::Result<serde_json::Value> {
    Err(crate::error::Error::ConfigError(
        "webhook dispatch requires the `http-discovery` feature (enabled by default)".to_string(),
    ))
}

/// Slack Bot Integration for incident notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackIntegration {
    pub webhook_url: String,
    pub channel: Option<String>,
    pub bot_name: Option<String>,
}

impl SlackIntegration {
    pub fn new(webhook_url: String) -> Self {
        SlackIntegration {
            webhook_url,
            channel: Some("#incidents".to_string()),
            bot_name: Some("BlastRadiusBot".to_string()),
        }
    }

    pub fn with_channel(mut self, channel: String) -> Self {
        self.channel = Some(channel);
        self
    }

    /// Send blast radius alert to Slack
    pub async fn send_blast_radius_alert(
        &self,
        service_name: &str,
        blast_score: f32,
        affected_services: Vec<String>,
    ) -> crate::error::Result<()> {
        let affected_list = affected_services.join(", ");

        let payload = serde_json::json!({
            "channel": self.channel,
            "username": self.bot_name.clone().unwrap_or_default(),
            "text": format!("🚨 Blast Radius Analysis for {}", service_name),
            "attachments": [{
                "color": if blast_score > 0.8 { "danger" } else { "warning" },
                "fields": [
                    {
                        "title": "Service",
                        "value": service_name,
                        "short": true
                    },
                    {
                        "title": "Blast Radius Score",
                        "value": format!("{:.1}/100", blast_score * 100.0),
                        "short": true
                    },
                    {
                        "title": "Affected Services",
                        "value": affected_list,
                        "short": false
                    }
                ]
            }]
        });

        post_json(&self.webhook_url, &payload).await?;
        Ok(())
    }

    /// Send anomaly detection alert to Slack
    pub async fn send_anomaly_alert(
        &self,
        node_name: &str,
        anomaly_type: &str,
        severity: f32,
        predicted_ttf: u32,
    ) -> crate::error::Result<()> {
        let payload = serde_json::json!({
            "channel": self.channel,
            "username": self.bot_name.clone().unwrap_or_default(),
            "text": format!("⚠️ Anomaly Detected on {}", node_name),
            "attachments": [{
                "color": if severity > 0.8 { "danger" } else { "warning" },
                "fields": [
                    {
                        "title": "Service",
                        "value": node_name,
                        "short": true
                    },
                    {
                        "title": "Anomaly Type",
                        "value": anomaly_type,
                        "short": true
                    },
                    {
                        "title": "Severity",
                        "value": format!("{:.0}%", severity * 100.0),
                        "short": true
                    },
                    {
                        "title": "Predicted Time to Failure",
                        "value": format!("{} minutes", predicted_ttf),
                        "short": true
                    }
                ]
            }]
        });

        post_json(&self.webhook_url, &payload).await?;
        Ok(())
    }
}

/// PagerDuty Integration for incident management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagerDutyIntegration {
    pub integration_key: String,
    pub api_url: Option<String>,
}

impl PagerDutyIntegration {
    pub fn new(integration_key: String) -> Self {
        PagerDutyIntegration {
            integration_key,
            api_url: Some("https://events.pagerduty.com/v2/enqueue".to_string()),
        }
    }

    /// Create PagerDuty incident from blast radius analysis
    pub async fn create_incident(
        &self,
        service_name: &str,
        blast_score: f32,
        description: &str,
    ) -> crate::error::Result<String> {
        let payload = serde_json::json!({
            "routing_key": self.integration_key,
            "event_action": "trigger",
            "dedup_key": format!("blast-{}-{}", service_name, chrono::Utc::now().timestamp()),
            "payload": {
                "summary": format!("High blast radius detected on {}", service_name),
                "severity": if blast_score > 0.8 { "critical" } else { "warning" },
                "source": "PyBlastRadius",
                "custom_details": {
                    "service": service_name,
                    "blast_score": blast_score,
                    "description": description
                }
            }
        });

        let url = self
            .api_url
            .as_deref()
            .unwrap_or("https://events.pagerduty.com/v2/enqueue");
        let response = post_json(url, &payload).await?;
        Ok(response
            .get("dedup_key")
            .or_else(|| response.get("id"))
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string())
    }

    /// Resolve PagerDuty incident
    pub async fn resolve_incident(&self, incident_id: &str) -> crate::error::Result<()> {
        let payload = serde_json::json!({
            "routing_key": self.integration_key,
            "event_action": "resolve",
            "dedup_key": incident_id
        });

        let url = self
            .api_url
            .as_deref()
            .unwrap_or("https://events.pagerduty.com/v2/enqueue");
        post_json(url, &payload).await?;
        Ok(())
    }
}

/// Slack command response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackCommandResponse {
    pub response_type: String, // "in_channel" or "ephemeral"
    pub text: String,
    pub blocks: Option<Vec<serde_json::Value>>,
}

impl SlackCommandResponse {
    pub fn new(text: String) -> Self {
        SlackCommandResponse {
            response_type: "in_channel".to_string(),
            text,
            blocks: None,
        }
    }

    pub fn ephemeral(text: String) -> Self {
        SlackCommandResponse {
            response_type: "ephemeral".to_string(),
            text,
            blocks: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_integration() {
        let slack = SlackIntegration::new("https://hooks.slack.com/services/...".to_string())
            .with_channel("#ops".to_string());

        assert_eq!(slack.channel, Some("#ops".to_string()));
    }

    #[test]
    fn test_pagerduty_integration() {
        let pd = PagerDutyIntegration::new("key123".to_string());

        assert_eq!(pd.integration_key, "key123");
        assert!(pd.api_url.is_some());
    }

    #[tokio::test]
    async fn test_slack_alert_posts_to_webhook() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_body("ok")
            .create_async()
            .await;

        let slack = SlackIntegration::new(server.url());
        slack
            .send_blast_radius_alert("payment-service", 0.95, vec!["db".to_string()])
            .await
            .unwrap();

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_pagerduty_create_incident_returns_dedup_key() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(202)
            .with_body(r#"{"dedup_key":"abc123"}"#)
            .create_async()
            .await;

        let mut pd = PagerDutyIntegration::new("key123".to_string());
        pd.api_url = Some(server.url());
        let id = pd
            .create_incident(
                "payment-service",
                0.95,
                "Payment service blast radius critical",
            )
            .await
            .unwrap();

        assert_eq!(id, "abc123");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_webhook_http_error_is_surfaced() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("POST", "/")
            .with_status(500)
            .with_body("boom")
            .create_async()
            .await;

        let slack = SlackIntegration::new(server.url());
        let error = slack
            .send_anomaly_alert("postgres", "latency_spike", 0.85, 15)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("HTTP 500"), "{error}");
    }
}
