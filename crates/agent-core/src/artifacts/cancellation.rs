use super::*;

impl PanelClient {
    pub async fn diagnostic_cancellations(
        &self,
    ) -> Result<Vec<sinan_protocol::DiagnosticCancelRequest>> {
        let url = self.panel.join("/api/agent/v1/diagnostics/cancellations")?;
        let requests: Vec<sinan_protocol::DiagnosticCancelRequest> =
            serde_json::from_slice(&self.download(url.as_str(), 1024 * 1024).await?)?;
        ensure!(
            requests.len() <= 64,
            "too many pending diagnostic cancellations"
        );
        Ok(requests)
    }

    pub async fn diagnostic_cancel_result(
        &self,
        result: &sinan_protocol::DiagnosticCancelResult,
    ) -> Result<()> {
        let response = self
            .client
            .post(self.panel.join(&format!(
                "/api/agent/v1/diagnostics/{}/cancel-confirmation",
                result.id
            ))?)
            .bearer_auth(&self.session_token)
            .json(result)
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "cancellation result returned HTTP {}",
            response.status()
        );
        Ok(())
    }
}
