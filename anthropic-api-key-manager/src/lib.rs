use hyperprocess_macro::*;
use hyperware_process_lib::{
    our,
    homepage::add_to_homepage,
    http::client::send_request_await_response,
    timer::set_timer,
};
use hyperware_app_common::SaveOptions;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use rand::seq::SliceRandom;
use chrono::Utc;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use url::Url;

#[derive(Default, Serialize, Deserialize)]
pub struct ApiKeyManagerState {
    admin_api_key: Option<String>,
    active_keys: HashSet<String>,
    historical_keys: HashSet<String>,
    key_to_nodes: HashMap<String, Vec<String>>,
    node_issue_times: HashMap<String, i64>,
    key_costs: HashMap<String, Vec<CostRecord>>,
    all_costs: Vec<CostRecord>,  // Store all costs globally
    last_cost_check: Option<i64>,
    ui_auth_token: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CostRecord {
    timestamp: i64,
    amount: f64,
    currency: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ApiKeyInfo {
    key: String,
    status: String,
    #[serde(rename = "totalCost")]
    total_cost: f64,
    #[serde(rename = "assignedNodes")]
    assigned_nodes: Vec<String>,
    #[serde(rename = "createdAt")]
    created_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct AddKeyRequest {
    #[serde(rename = "apiKey")]
    api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RemoveKeyRequest {
    #[serde(rename = "apiKey")]
    api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyStatusRequest {
    #[serde(rename = "apiKey")]
    api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CostRangeRequest {
    #[serde(rename = "startDate")]
    start_date: Option<String>,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyCostRequest {
    #[serde(rename = "apiKey")]
    api_key: String,
    #[serde(rename = "startDate")]
    start_date: Option<String>,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SetAdminKeyParams {
    #[serde(rename = "adminKey")]
    admin_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NodeAssignment {
    #[serde(rename = "nodeId")]
    node_id: String,
    #[serde(rename = "apiKey")]
    api_key: String,
    #[serde(rename = "issuedAt")]
    issued_at: i64,
}

// Response types
#[derive(Debug, Serialize, Deserialize)]
struct SuccessResponse {
    success: bool,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AdminKeyStatusResponse {
    #[serde(rename = "hasAdminKey")]
    has_admin_key: bool,
    #[serde(rename = "keyPrefix")]
    key_prefix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    token: String,
    #[serde(rename = "hasAdminKey")]
    has_admin_key: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyStatusResponse {
    status: String,
    #[serde(rename = "assignedNodes")]
    assigned_nodes: Vec<String>,
    #[serde(rename = "totalCost")]
    total_cost: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TotalCostsResponse {
    #[serde(rename = "totalCost")]
    total_cost: f64,
    #[serde(rename = "costByKey")]
    cost_by_key: Vec<(String, f64)>,  // Changed from HashMap to Vec of tuples for TypeScript compatibility
    currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyCostsResponse {
    #[serde(rename = "apiKey")]
    api_key: String,
    costs: Vec<CostRecord>,
    total: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CostsRefreshResponse {
    success: bool,
    message: String,
    timestamp: i64,
}

// Anthropic API structures
#[derive(Serialize, Deserialize, Debug)]
struct AnthropicCostReport {
    data: Vec<CostReportData>,
    has_more: bool,
    next_page: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CostReportData {
    starting_at: String,
    ending_at: String,
    results: Vec<CostReportResult>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CostReportResult {
    currency: String,
    amount: String,
    workspace_id: Option<String>,
    description: String,
    cost_type: String,
    context_window: Option<String>,
    model: Option<String>,
    service_tier: Option<String>,
    token_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicWorkspace {
    id: String,
    name: String,
    created_at: String,
    archived_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicApiKey {
    id: String,
    name: String,
    status: String,
    created_at: String,
    workspace_id: String,
}

#[hyperprocess(
    name = "Anthropic API Key Manager",
    ui = Some(HttpBindingConfig::default()),
    endpoints = vec![
        Binding::Http {
            path: "/api",
            config: HttpBindingConfig::new(false, false, false, None)
        },
        Binding::Ws {
            path: "/ws",
            config: WsBindingConfig::default()
        }
    ],
    save_config = SaveOptions::OnDiff,
    wit_world = "anthropic-api-key-manager-v0"
)]
impl ApiKeyManagerState {
    #[init]
    async fn initialize(&mut self) {
        add_to_homepage("API Key Manager", Some("🔑"), Some("/"), None);

        if self.ui_auth_token.is_none() {
            let token = BASE64.encode(format!("{:x}", rand::random::<u128>()));
            self.ui_auth_token = Some(token.clone());
            println!("Generated UI auth token: {}", token);
        }

        // Start hourly cost polling timer (3600000 ms = 1 hour)
        let _ = set_timer(3600000, None);

        println!("Anthropic API Key Manager initialized on node: {}", our().node);
    }

    #[remote]
    async fn request_api_key(&mut self, node_id: String) -> Result<String, String> {
        if let Some(existing_key) = self.find_key_for_node(&node_id) {
            return Ok(existing_key);
        }

        if self.active_keys.is_empty() {
            return Err("No active API keys available".to_string());
        }

        let keys: Vec<String> = self.active_keys.iter().cloned().collect();
        let selected_key = keys
            .choose(&mut rand::thread_rng())
            .ok_or("Failed to select random key")?
            .clone();

        self.key_to_nodes
            .entry(selected_key.clone())
            .or_insert_with(Vec::new)
            .push(node_id.clone());

        self.node_issue_times.insert(node_id, Utc::now().timestamp());

        Ok(selected_key)
    }

    #[http]
    async fn add_api_key(&mut self, request: AddKeyRequest) -> Result<SuccessResponse, String> {
        if self.active_keys.contains(&request.api_key) {
            return Err("API key already exists".to_string());
        }

        self.active_keys.insert(request.api_key.clone());

        Ok(SuccessResponse {
            success: true,
            message: "API key added successfully".to_string(),
        })
    }

    #[http]
    async fn remove_api_key(&mut self, request: RemoveKeyRequest) -> Result<SuccessResponse, String> {
        if !self.active_keys.contains(&request.api_key) {
            return Err("API key not found".to_string());
        }

        self.active_keys.remove(&request.api_key);
        self.historical_keys.insert(request.api_key.clone());

        Ok(SuccessResponse {
            success: true,
            message: "API key removed successfully".to_string(),
        })
    }

    #[http]
    async fn list_keys(&self) -> Result<Vec<ApiKeyInfo>, String> {
        let keys: Vec<ApiKeyInfo> = self.active_keys
            .iter()
            .map(|key| {
                let nodes = self.key_to_nodes.get(key)
                    .map(|n| n.clone())
                    .unwrap_or_default();

                let total_cost = self.key_costs.get(key)
                    .map(|costs| costs.iter().map(|c| c.amount).sum())
                    .unwrap_or(0.0);

                ApiKeyInfo {
                    key: key.clone(),
                    status: "active".to_string(),
                    total_cost,
                    assigned_nodes: nodes,
                    created_at: 0,
                }
            })
            .collect();

        Ok(keys)
    }

    #[http]
    async fn get_key_status(&self, request: KeyStatusRequest) -> Result<KeyStatusResponse, String> {

        let is_active = self.active_keys.contains(&request.api_key);
        let is_historical = self.historical_keys.contains(&request.api_key);

        let status = if is_active {
            "active"
        } else if is_historical {
            "inactive"
        } else {
            "unknown"
        };

        let nodes = self.key_to_nodes.get(&request.api_key)
            .map(|n| n.clone())
            .unwrap_or_default();

        let total_cost = self.key_costs.get(&request.api_key)
            .map(|costs| costs.iter().map(|c| c.amount).sum::<f64>())
            .unwrap_or(0.0);

        Ok(KeyStatusResponse {
            status: status.to_string(),
            assigned_nodes: nodes,
            total_cost,
        })
    }

    #[http]
    async fn get_total_costs(&self, request: CostRangeRequest) -> Result<TotalCostsResponse, String> {

        let mut total_cost = 0.0;
        let mut cost_by_key: Vec<(String, f64)> = Vec::new();

        for (key, costs) in &self.key_costs {
            let key_total: f64 = costs.iter()
                .filter(|c| self.filter_by_date(c.timestamp, &request.start_date, &request.end_date))
                .map(|c| c.amount)
                .sum();

            if key_total > 0.0 {
                cost_by_key.push((key.clone(), key_total));
                total_cost += key_total;
            }
        }

        Ok(TotalCostsResponse {
            total_cost,
            cost_by_key,
            currency: "USD".to_string(),
        })
    }

    #[http]
    async fn get_key_costs(&self, request: KeyCostRequest) -> Result<KeyCostsResponse, String> {

        let costs = self.key_costs.get(&request.api_key)
            .map(|costs| {
                costs.iter()
                    .filter(|c| self.filter_by_date(c.timestamp, &request.start_date, &request.end_date))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let total: f64 = costs.iter().map(|c| c.amount).sum();

        Ok(KeyCostsResponse {
            api_key: request.api_key,
            costs,
            total,
        })
    }

    #[http]
    async fn get_node_history(&self) -> Result<Vec<NodeAssignment>, String> {
        let mut assignments: Vec<NodeAssignment> = Vec::new();

        for (key, nodes) in &self.key_to_nodes {
            for node in nodes {
                let issued_at = self.node_issue_times.get(node).copied().unwrap_or(0);
                assignments.push(NodeAssignment {
                    node_id: node.clone(),
                    api_key: key.clone(),
                    issued_at,
                });
            }
        }

        assignments.sort_by_key(|a| a.issued_at);

        Ok(assignments)
    }

    #[http]
    async fn set_admin_key(&mut self, request: SetAdminKeyParams) -> Result<SuccessResponse, String> {
        self.admin_api_key = Some(request.admin_key.clone());

        // Log for debugging
        println!("Admin key set: {}", if request.admin_key.starts_with("sk-") { "sk-***" } else { "invalid format" });

        Ok(SuccessResponse {
            success: true,
            message: "Admin key set successfully".to_string(),
        })
    }

    #[http]
    async fn check_admin_key(&self) -> Result<AdminKeyStatusResponse, String> {
        Ok(AdminKeyStatusResponse {
            has_admin_key: self.admin_api_key.is_some(),
            key_prefix: self.admin_api_key.as_ref().map(|k| {
                if k.starts_with("sk-") {
                    "sk-***".to_string()
                } else {
                    "invalid".to_string()
                }
            }),
        })
    }

    #[http]
    async fn initialize_auth(&mut self) -> Result<AuthResponse, String> {
        if self.ui_auth_token.is_none() {
            let token = BASE64.encode(format!("{:x}", rand::random::<u128>()));
            self.ui_auth_token = Some(token.clone());
        }

        // Include admin key status in response
        Ok(AuthResponse {
            token: self.ui_auth_token.as_ref().unwrap().clone(),
            has_admin_key: self.admin_api_key.is_some(),
        })
    }

    #[http]
    async fn get_all_costs(&self) -> Result<Vec<CostRecord>, String> {
        Ok(self.all_costs.clone())
    }
    
    #[http]
    async fn refresh_costs(&mut self) -> Result<CostsRefreshResponse, String> {
        if self.admin_api_key.is_none() {
            return Err("Admin API key not configured".to_string());
        }

        let now = Utc::now().timestamp();

        if let Some(last_check) = self.last_cost_check {
            if now - last_check < 3600 {
                return Ok(CostsRefreshResponse {
                    success: false,
                    message: format!("Costs were recently refreshed at {}", last_check),
                    timestamp: last_check,
                });
            }
        }

        // Fetch real costs from Anthropic API
        match self.fetch_costs_from_anthropic().await {
            Ok(costs_added) => {
                self.last_cost_check = Some(now);
                Ok(CostsRefreshResponse {
                    success: true,
                    message: format!("Costs refreshed successfully. Added {} cost records", costs_added),
                    timestamp: now,
                })
            }
            Err(e) => {
                println!("Failed to fetch costs from Anthropic: {}", e);
                Err(format!("Failed to fetch costs: {}", e))
            }
        }
    }

}

impl ApiKeyManagerState {
    fn find_key_for_node(&self, node_id: &str) -> Option<String> {
        for (key, nodes) in &self.key_to_nodes {
            if nodes.contains(&node_id.to_string()) {
                return Some(key.clone());
            }
        }
        None
    }

    fn filter_by_date(&self, timestamp: i64, start_date: &Option<String>, end_date: &Option<String>) -> bool {
        if let Some(start) = start_date {
            if let Ok(start_ts) = chrono::DateTime::parse_from_rfc3339(start) {
                if timestamp < start_ts.timestamp() {
                    return false;
                }
            }
        }

        if let Some(end) = end_date {
            if let Ok(end_ts) = chrono::DateTime::parse_from_rfc3339(end) {
                if timestamp > end_ts.timestamp() {
                    return false;
                }
            }
        }

        true
    }

    async fn fetch_costs_from_anthropic(&mut self) -> Result<usize, String> {
        let admin_key = self.admin_api_key.as_ref()
            .ok_or("Admin API key not configured")?;

        // Use fixed starting date
        let now = Utc::now();
        let starting_at = "2025-08-01T00:00:00Z";

        let url_str = format!(
            "https://api.anthropic.com/v1/organizations/cost_report?starting_at={}&group_by[]=workspace_id&group_by[]=description&limit=31",
            starting_at
        );

        let url = Url::parse(&url_str).map_err(|e| format!("Invalid URL: {}", e))?;

        let mut headers = HashMap::new();
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("x-api-key".to_string(), admin_key.clone());

        // Retry logic: try up to 3 times with exponential backoff
        let mut attempts = 0;
        let max_attempts = 3;
        let mut last_error = String::new();

        while attempts < max_attempts {
            attempts += 1;

            match send_request_await_response(
                http::Method::GET,
                url.clone(),
                Some(headers.clone()),
                30000, // 30 second timeout
                vec![]
            ) {
                Ok(response) => {
                    if response.status() == http::StatusCode::OK {
                        match serde_json::from_slice::<AnthropicCostReport>(response.body()) {
                            Ok(cost_report) => {
                                // Successfully got the cost report, process it
                                return self.process_cost_report(cost_report, now.timestamp());
                            }
                            Err(e) => {
                                last_error = format!("Failed to parse response: {}", e);
                            }
                        }
                    } else if response.status() == http::StatusCode::TOO_MANY_REQUESTS {
                        last_error = format!("Rate limited, attempt {}/{}", attempts, max_attempts);
                        // Wait longer for rate limiting
                        if attempts < max_attempts {
                            let wait_ms = 5000 * attempts as u64; // 5s, 10s, 15s
                            println!("Rate limited, waiting {}ms before retry", wait_ms);
                            set_timer(wait_ms, None);
                        }
                    } else {
                        last_error = format!("API returned status {}: {}",
                            response.status(),
                            String::from_utf8_lossy(response.body())
                        );
                        // For non-retriable errors, break immediately
                        if response.status() == http::StatusCode::UNAUTHORIZED ||
                           response.status() == http::StatusCode::FORBIDDEN {
                            break;
                        }
                    }
                }
                Err(e) => {
                    last_error = format!("HTTP request failed: {:?}", e);
                }
            }

            // Wait before retry (exponential backoff)
            if attempts < max_attempts {
                let wait_ms = 1000 * (2_u64.pow(attempts - 1)); // 1s, 2s, 4s
                println!("Request failed, waiting {}ms before retry", wait_ms);
                set_timer(wait_ms, None);
            }
        }

        Err(format!("Failed after {} attempts: {}", attempts, last_error))
    }

    fn process_cost_report(&mut self, cost_report: AnthropicCostReport, timestamp: i64) -> Result<usize, String> {
        let mut costs_added = 0;

        for data in cost_report.data {
            for result in data.results {
                // Try to parse the amount
                let amount = result.amount.parse::<f64>().unwrap_or(0.0);
                
                // Skip zero amounts
                if amount == 0.0 {
                    continue;
                }

                // Find which key this cost belongs to (if any)
                // For now, we'll aggregate all costs since we don't have workspace mapping
                let record = CostRecord {
                    timestamp,
                    amount,
                    currency: result.currency,
                    description: result.description,
                };
                
                // Add to global costs
                self.all_costs.push(record.clone());
                costs_added += 1;

                // Also add to per-key costs if we have active keys
                for key in self.active_keys.iter() {
                    self.key_costs
                        .entry(key.clone())
                        .or_insert_with(Vec::new)
                        .push(record.clone());
                }
            }
        }

        Ok(costs_added)
    }

    async fn create_workspace(&self, name: String) -> Result<AnthropicWorkspace, String> {
        let admin_key = self.admin_api_key.as_ref()
            .ok_or("Admin API key not configured")?;

        let url = Url::parse("https://api.anthropic.com/v1/organizations/workspaces")
            .map_err(|e| format!("Invalid URL: {}", e))?;

        let mut headers = HashMap::new();
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("x-api-key".to_string(), admin_key.clone());

        let body = serde_json::json!({
            "name": name
        });

        let response = send_request_await_response(
            http::Method::POST,
            url,
            Some(headers),
            30000,
            body.to_string().into_bytes()
        ).map_err(|e| format!("HTTP request failed: {:?}", e))?;

        if response.status() != http::StatusCode::OK && response.status() != http::StatusCode::CREATED {
            return Err(format!("API returned status {}: {}",
                response.status(),
                String::from_utf8_lossy(response.body())
            ));
        }

        serde_json::from_slice(response.body())
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    async fn list_api_keys(&self, workspace_id: Option<String>) -> Result<Vec<AnthropicApiKey>, String> {
        let admin_key = self.admin_api_key.as_ref()
            .ok_or("Admin API key not configured")?;

        let mut url_str = "https://api.anthropic.com/v1/organizations/api_keys?limit=100&status=active".to_string();
        if let Some(ws_id) = workspace_id {
            url_str.push_str(&format!("&workspace_id={}", ws_id));
        }

        let url = Url::parse(&url_str).map_err(|e| format!("Invalid URL: {}", e))?;

        let mut headers = HashMap::new();
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        headers.insert("x-api-key".to_string(), admin_key.clone());

        let response = send_request_await_response(
            http::Method::GET,
            url,
            Some(headers),
            30000,
            vec![]
        ).map_err(|e| format!("HTTP request failed: {:?}", e))?;

        if response.status() != http::StatusCode::OK {
            return Err(format!("API returned status {}: {}",
                response.status(),
                String::from_utf8_lossy(response.body())
            ));
        }

        serde_json::from_slice(response.body())
            .map_err(|e| format!("Failed to parse response: {}", e))
    }
}

// Timer handler for periodic cost polling
async fn handle_timer(state: &mut ApiKeyManagerState) -> Result<(), String> {
    if state.admin_api_key.is_some() {
        // Refresh costs periodically
        match state.fetch_costs_from_anthropic().await {
            Ok(costs_added) => {
                println!("Periodic cost refresh: Added {} cost records", costs_added);
            }
            Err(e) => {
                println!("Failed to refresh costs periodically: {}", e);
            }
        }
    }

    // Schedule next timer
    let _ = set_timer(3600000, None); // 1 hour
    Ok(())
}
