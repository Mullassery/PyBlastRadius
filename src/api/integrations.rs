use serde::{Deserialize, Serialize};

// Slack/PagerDuty client stubs.
//
// **Not wired to a real HTTP client.** These build the correct outbound
// payload shape but never send it — the `reqwest` POST calls are commented
// out below and each method just returns `Ok(())` / a hardcoded fake id.
// They exist to document the intended integration shape, not as working
// notification/incident clients. Do not rely on these for real alerting
// until they're wired to an HTTP client; this is tracked as a known gap
// rather than something this crate claims to do today.

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

        // In production: Use reqwest to POST to webhook_url
        // reqwest::Client::new()
        //     .post(&self.webhook_url)
        //     .json(&payload)
        //     .send()
        //     .await?;

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

        // In production: POST to webhook_url
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

        // In production: POST to PagerDuty API
        // let response = reqwest::Client::new()
        //     .post(self.api_url.as_ref().unwrap())
        //     .json(&payload)
        //     .send()
        //     .await?;
        //
        // let result: serde_json::Value = response.json().await?;
        // Ok(result["id"].as_str().unwrap_or("unknown").to_string())

        Ok("incident-123".to_string())
    }

    /// Resolve PagerDuty incident
    pub async fn resolve_incident(&self, incident_id: &str) -> crate::error::Result<()> {
        let payload = serde_json::json!({
            "routing_key": self.integration_key,
            "event_action": "resolve",
            "dedup_key": incident_id
        });

        // In production: POST to PagerDuty API
        Ok(())
    }
}

/// Slack command response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackCommandResponse {
    pub response_type: String,  // "in_channel" or "ephemeral"
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
    async fn test_slack_anomaly_alert() {
        let slack = SlackIntegration::new("https://hooks.slack.com/services/...".to_string());

        let result = slack
            .send_anomaly_alert("postgres", "latency_spike", 0.85, 15)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pagerduty_create_incident() {
        let pd = PagerDutyIntegration::new("key123".to_string());

        let result = pd
            .create_incident("payment-service", 0.95, "Payment service blast radius critical")
            .await;

        assert!(result.is_ok());
    }
}
