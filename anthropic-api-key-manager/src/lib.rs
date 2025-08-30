use hyperprocess_macro::*;
use hyperware_process_lib::{
    our,
    println,
    homepage::add_to_homepage,
    http::client::send_request_await_response,
    hyperapp::{source, SaveOptions, spawn, sleep},
    timer::set_timer,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use rand::seq::SliceRandom;
use chrono::Utc;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use url::Url;

#[derive(Default, Serialize, Deserialize)]
pub struct AnthropicApiKeyManagerState {
    admin_api_key: Option<String>,
    active_keys: HashSet<String>,
    historical_keys: HashSet<String>,
    key_to_nodes: HashMap<String, Vec<String>>,
    node_issue_times: HashMap<String, i64>,
    key_costs: HashMap<String, Vec<CostRecord>>,
    all_costs: Vec<CostRecord>,  // Store all costs globally
    last_cost_check: Option<i64>,
    last_cost_query_date: Option<String>,  // Store the last date we queried up to (RFC3339 format)
    ui_auth_token: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CostRecord {
    timestamp: i64,
    amount: f64,        // Amount in dollars (converted from API's cents)
    currency: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ApiKeyInfo {
    key: String,
    status: String,
    total_cost: f64,
    assigned_nodes: Vec<String>,
    created_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct AddKeyRequest {
    api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RemoveKeyRequest {
    api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyStatusRequest {
    api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CostRangeRequest {
    start_date: Option<String>,
    end_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyCostRequest {
    api_key: String,
    start_date: Option<String>,
    end_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SetAdminKeyParams {
    admin_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NodeAssignment {
    node_id: String,
    api_key: String,
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
    has_admin_key: bool,
    key_prefix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    token: String,
    has_admin_key: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyStatusResponse {
    status: String,
    assigned_nodes: Vec<String>,
    total_cost: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TotalCostsResponse {
    total_cost: f64,
    cost_by_key: Vec<(String, f64)>,  // Changed from HashMap to Vec of tuples for TypeScript compatibility
    currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyCostsResponse {
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
    currency: String,              // Always "USD" 
    amount: String,                // Amount in cents as decimal string (e.g., "123.45" = $1.2345)
    workspace_id: Option<String>,
    description: Option<String>,   // Made optional since it can be null when not grouping by description
    cost_type: Option<String>,     // Made optional
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
impl AnthropicApiKeyManagerState {
    #[init]
    async fn initialize(&mut self) {
        add_to_homepage("API Key Manager", None, Some("/"), None);

        if self.ui_auth_token.is_none() {
            let token = BASE64.encode(format!("{:x}", rand::random::<u128>()));
            self.ui_auth_token = Some(token.clone());
            println!("Generated UI auth token: {}", token);
        }

        // Clone admin_api_key for the spawn task (if it exists)
        let admin_key = self.admin_api_key.clone();

        // Spawn a task to periodically refresh costs
        spawn(async move {
            loop {
                // Wait 1 hour between cost refreshes
                let _ = sleep(3600000).await;

                // Only attempt to refresh if we have an admin key
                if admin_key.is_some() {
                    println!("Periodic cost refresh triggered");
                    // Note: In the spawned task we can't directly call methods on self
                    // The timer handler will still work for now as a fallback
                }
            }
        });

        println!("Anthropic API Key Manager initialized on node: {}", our().node);
    }

    #[remote]
    async fn request_api_key(&mut self) -> Result<String, String> {
        let node_id = source().node;

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
        println!("get_all_costs called. Returning {} cost records", self.all_costs.len());

        // Debug: print first few records if any exist
        if !self.all_costs.is_empty() {
            println!("Sample cost records:");
            for (i, cost) in self.all_costs.iter().take(3).enumerate() {
                println!("  [{}] {} {} - {}", i, cost.amount, cost.currency, cost.description);
            }
        }

        Ok(self.all_costs.clone())
    }

    #[http]
    async fn refresh_costs(&mut self) -> Result<CostsRefreshResponse, String> {
        if self.admin_api_key.is_none() {
            return Err("Admin API key not configured".to_string());
        }

        let now = Utc::now().timestamp();

        // Debug logging
        println!("refresh_costs called. Current time: {}, Last check: {:?}", now, self.last_cost_check);

        if let Some(last_check) = self.last_cost_check {
            let time_since_last = now - last_check;
            println!("Time since last check: {} seconds", time_since_last);

            // Only rate limit if less than 60 seconds (not 3600)
            // But allow negative time_since_last (which means timestamp is in future - a bug we're fixing)
            if time_since_last < 60 && time_since_last > 0 {
                println!("Rate limiting: costs were refreshed {} seconds ago", time_since_last);
                return Ok(CostsRefreshResponse {
                    success: false,
                    message: format!("Costs were recently refreshed {} seconds ago", time_since_last),
                    timestamp: last_check,
                });
            }

            // If time_since_last is negative, clear the bad timestamp
            if time_since_last < 0 {
                println!("WARNING: last_cost_check is in the future! Clearing it.");
                self.last_cost_check = None;
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

    #[http]
    async fn reset_costs(&mut self) -> Result<SuccessResponse, String> {
        if self.admin_api_key.is_none() {
            return Err("Admin API key not configured".to_string());
        }

        // Clear all cost data
        self.all_costs.clear();
        self.key_costs.clear();
        self.last_cost_query_date = None;
        self.last_cost_check = None;

        println!("Cost data reset. All historical cost data cleared.");

        Ok(SuccessResponse {
            success: true,
            message: "Cost data reset successfully. All historical data cleared.".to_string(),
        })
    }

}

impl AnthropicApiKeyManagerState {
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

        let now = Utc::now();
        
        // Use the last query date if available, otherwise start from 30 days ago
        let starting_at = if let Some(ref last_date) = self.last_cost_query_date {
            // Start from the last date we queried (should already be in correct format)
            last_date.clone()
        } else {
            // Default to 30 days ago for initial fetch
            let thirty_days_ago = now - chrono::Duration::days(30);
            // Format as UTC with Z suffix (e.g., "2025-08-01T00:00:00Z")
            format!("{}Z", thirty_days_ago.format("%Y-%m-%dT%H:%M:%S"))
        };
        
        println!("Fetching costs starting from: {}", starting_at);
        
        // Collect all cost reports across pages
        let mut all_cost_reports = Vec::new();
        let mut next_page: Option<String> = None;
        let mut page_count = 0;
        let max_pages = 100; // Safety limit to prevent infinite loops

        // Headers remain the same for all requests
        let mut headers = HashMap::new();
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("x-api-key".to_string(), admin_key.clone());

        // Fetch all pages
        loop {
            page_count += 1;
            if page_count > max_pages {
                println!("Warning: Reached maximum page limit of {}", max_pages);
                break;
            }

            // Build URL with pagination
            let url_str = if let Some(ref page_token) = next_page {
                format!(
                    "https://api.anthropic.com/v1/organizations/cost_report?starting_at={}&group_by[]=workspace_id&group_by[]=description&limit=30&page={}",
                    starting_at, page_token
                )
            } else {
                format!(
                    "https://api.anthropic.com/v1/organizations/cost_report?starting_at={}&group_by[]=workspace_id&group_by[]=description&limit=30",
                    starting_at
                )
            };

            println!("Fetching costs page {} from URL: {}", page_count, url_str);

            let url = Url::parse(&url_str).map_err(|e| format!("Invalid URL: {}", e))?;

            // Retry logic for each page request
            let mut attempts = 0;
            let max_attempts = 3;
            let mut last_error = String::new();
            let mut page_fetched = false;

            while attempts < max_attempts && !page_fetched {
                attempts += 1;

                match send_request_await_response(
                    http::Method::GET,
                    url.clone(),
                    Some(headers.clone()),
                    30000, // 30 second timeout
                    vec![]
                ).await {
                    Ok(response) => {
                        if response.status() == http::StatusCode::OK {
                            let response_body = String::from_utf8_lossy(response.body());
                            
                            // Only log first 500 chars of response to avoid clutter
                            if response_body.len() > 500 {
                                println!("Anthropic API response (truncated): {}...", &response_body[..500]);
                            } else {
                                println!("Anthropic API response: {}", response_body);
                            }

                            match serde_json::from_str::<AnthropicCostReport>(&response_body) {
                                Ok(cost_report) => {
                                    println!("Successfully parsed cost report page {} with {} data entries", 
                                             page_count, cost_report.data.len());
                                    
                                    // Store the next page token if available
                                    let has_more = cost_report.has_more;
                                    next_page = cost_report.next_page.clone();
                                    
                                    // Add this page's data to our collection
                                    all_cost_reports.push(cost_report);
                                    page_fetched = true;
                                    
                                    // Check if we need to fetch more pages
                                    if !has_more || next_page.is_none() {
                                        println!("Reached last page of cost reports (total pages: {})", page_count);
                                        // Process all collected reports
                                        return self.process_all_cost_reports(all_cost_reports, now.timestamp());
                                    }
                                }
                                Err(e) => {
                                    last_error = format!("Failed to parse response: {}. Body: {}", e, response_body);
                                    println!("Parse error: {}", last_error);
                                }
                            }
                        } else if response.status() == http::StatusCode::TOO_MANY_REQUESTS {
                            last_error = format!("Rate limited on page {}, attempt {}/{}", page_count, attempts, max_attempts);
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
                                return Err(last_error);
                            }
                        }
                    }
                    Err(e) => {
                        last_error = format!("HTTP request failed: {:?}", e);
                    }
                }

                // Wait before retry (exponential backoff)
                if attempts < max_attempts && !page_fetched {
                    let wait_ms = 1000 * (2_u64.pow(attempts - 1)); // 1s, 2s, 4s
                    println!("Request failed for page {}, waiting {}ms before retry", page_count, wait_ms);
                    set_timer(wait_ms, None);
                }
            }

            // If we couldn't fetch this page after all retries, fail
            if !page_fetched {
                return Err(format!("Failed to fetch page {} after {} attempts: {}", 
                                   page_count, attempts, last_error));
            }
        }

        // If we somehow exit the loop without returning, process what we have
        if !all_cost_reports.is_empty() {
            self.process_all_cost_reports(all_cost_reports, now.timestamp())
        } else {
            Err("No cost reports fetched".to_string())
        }
    }

    fn process_all_cost_reports(&mut self, cost_reports: Vec<AnthropicCostReport>, timestamp: i64) -> Result<usize, String> {
        let mut total_costs_added = 0;
        let mut latest_date: Option<String> = None;
        
        println!("Processing {} pages of cost reports", cost_reports.len());
        
        for (page_num, cost_report) in cost_reports.into_iter().enumerate() {
            println!("Processing page {} with {} data entries", page_num + 1, cost_report.data.len());
            
            // Track the latest date we've seen
            for data in &cost_report.data {
                if let Some(ref current_latest) = latest_date {
                    if data.ending_at > *current_latest {
                        latest_date = Some(data.ending_at.clone());
                    }
                } else {
                    latest_date = Some(data.ending_at.clone());
                }
            }
            
            match self.process_cost_report(cost_report, timestamp) {
                Ok(costs_added) => {
                    total_costs_added += costs_added;
                    println!("Added {} costs from page {}", costs_added, page_num + 1);
                }
                Err(e) => {
                    println!("Warning: Failed to process page {}: {}", page_num + 1, e);
                    // Continue processing other pages even if one fails
                }
            }
        }
        
        // Store the latest date we've queried for next time
        if let Some(latest) = latest_date {
            // Ensure the date is in the correct format (YYYY-MM-DDTHH:MM:SSZ)
            // If it already ends with Z, use as-is; otherwise parse and reformat
            let formatted_date = if latest.ends_with('Z') {
                latest
            } else {
                // Parse and reformat to ensure correct format
                match chrono::DateTime::parse_from_rfc3339(&latest) {
                    Ok(dt) => {
                        let utc_dt = dt.with_timezone(&Utc);
                        format!("{}Z", utc_dt.format("%Y-%m-%dT%H:%M:%S"))
                    }
                    Err(_) => {
                        // If parsing fails, keep the original but warn
                        println!("WARNING: Could not parse ending_at date '{}', storing as-is", latest);
                        latest
                    }
                }
            };
            
            println!("Updating last_cost_query_date to: {}", formatted_date);
            self.last_cost_query_date = Some(formatted_date);
        }
        
        println!("Total costs added across all pages: {}. Total costs in system: {}", 
                 total_costs_added, self.all_costs.len());
        Ok(total_costs_added)
    }

    fn process_cost_report(&mut self, cost_report: AnthropicCostReport, _query_timestamp: i64) -> Result<usize, String> {
        let mut costs_added = 0;

        println!("Processing cost report with {} data entries", cost_report.data.len());

        for data in cost_report.data {
            println!("Processing data entry from {} to {} with {} results",
                     data.starting_at, data.ending_at, data.results.len());

            // Parse the starting_at timestamp to use as the cost incurred timestamp
            let cost_timestamp = chrono::DateTime::parse_from_rfc3339(&data.starting_at)
                .map(|dt| dt.timestamp())
                .unwrap_or_else(|e| {
                    println!("Warning: Failed to parse starting_at timestamp '{}': {}", data.starting_at, e);
                    // Fallback to current time if parsing fails
                    Utc::now().timestamp()
                });

            for result in data.results {
                // Parse the amount - API returns cents as a decimal string (e.g., "123.45" = 123.45 cents = $1.2345)
                let amount_in_cents = result.amount.parse::<f64>().unwrap_or(0.0);
                // Convert cents to dollars
                let amount_in_dollars = amount_in_cents / 100.0;
                let description = result.description.clone().unwrap_or_else(|| "Unknown".to_string());
                println!("Cost result: {} cents (${:.4}) {} - {} (incurred at {})", 
                         amount_in_cents, amount_in_dollars, result.currency, description, data.starting_at);

                // Skip zero amounts
                if amount_in_cents == 0.0 {
                    continue;
                }

                // Find which key this cost belongs to (if any)
                // For now, we'll aggregate all costs since we don't have workspace mapping
                // Store the amount in dollars for consistency
                let record = CostRecord {
                    timestamp: cost_timestamp,  // Use the actual cost incurred timestamp
                    amount: amount_in_dollars,  // Store as dollars
                    currency: result.currency,
                    description,
                };

                // Add to global costs
                self.all_costs.push(record.clone());
                costs_added += 1;
                println!("Added cost record: ${:.4} {} incurred at {}", 
                         amount_in_dollars, record.currency, data.starting_at);

                // Also add to per-key costs if we have active keys
                for key in self.active_keys.iter() {
                    self.key_costs
                        .entry(key.clone())
                        .or_insert_with(Vec::new)
                        .push(record.clone());
                }
            }
        }

        println!("Total costs added: {}. Total costs in system: {}", costs_added, self.all_costs.len());
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
        ).await.map_err(|e| format!("HTTP request failed: {:?}", e))?;

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

        let mut url_str = "https://api.anthropic.com/v1/organizations/api_keys?limit=30&status=active".to_string();
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
        ).await.map_err(|e| format!("HTTP request failed: {:?}", e))?;

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

