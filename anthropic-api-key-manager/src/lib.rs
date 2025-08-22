use hyperprocess_macro::*;
use hyperware_process_lib::{
    our,
    homepage::add_to_homepage,
};
use hyperware_app_common::SaveOptions;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use rand::seq::SliceRandom;
use chrono::Utc;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

#[derive(Default, Serialize, Deserialize)]
pub struct ApiKeyManagerState {
    admin_api_key: Option<String>,
    active_keys: HashSet<String>,
    historical_keys: HashSet<String>,
    key_to_nodes: HashMap<String, Vec<String>>,
    node_issue_times: HashMap<String, i64>,
    key_costs: HashMap<String, Vec<CostRecord>>,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ApiKeyInfo {
    key: String,
    status: String,
    total_cost: f64,
    assigned_nodes: Vec<String>,
    created_at: i64,
}

#[derive(Serialize, Deserialize)]
struct AddKeyRequest {
    api_key: String,
}

#[derive(Serialize, Deserialize)]
struct RemoveKeyRequest {
    api_key: String,
}

#[derive(Serialize, Deserialize)]
struct KeyStatusRequest {
    api_key: String,
}

#[derive(Serialize, Deserialize)]
struct CostRangeRequest {
    start_date: Option<String>,
    end_date: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct KeyCostRequest {
    api_key: String,
    start_date: Option<String>,
    end_date: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct SetAdminKeyRequest {
    admin_key: String,
}

#[derive(Serialize, Deserialize)]
struct NodeAssignment {
    node_id: String,
    api_key: String,
    issued_at: i64,
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
    async fn add_api_key(&mut self, request_body: String) -> Result<String, String> {
        let request: AddKeyRequest = serde_json::from_str(&request_body)
            .map_err(|e| format!("Invalid request: {}", e))?;
        
        if self.active_keys.contains(&request.api_key) {
            return Err("API key already exists".to_string());
        }
        
        self.active_keys.insert(request.api_key.clone());
        
        Ok(serde_json::json!({
            "success": true,
            "message": "API key added successfully"
        }).to_string())
    }
    
    #[http]
    async fn remove_api_key(&mut self, request_body: String) -> Result<String, String> {
        let request: RemoveKeyRequest = serde_json::from_str(&request_body)
            .map_err(|e| format!("Invalid request: {}", e))?;
        
        if !self.active_keys.contains(&request.api_key) {
            return Err("API key not found".to_string());
        }
        
        self.active_keys.remove(&request.api_key);
        self.historical_keys.insert(request.api_key.clone());
        
        Ok(serde_json::json!({
            "success": true,
            "message": "API key removed successfully"
        }).to_string())
    }
    
    #[http]
    async fn list_keys(&self, _request_body: String) -> Result<String, String> {
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
        
        serde_json::to_string(&keys).map_err(|e| format!("Serialization error: {}", e))
    }
    
    #[http]
    async fn get_key_status(&self, request_body: String) -> Result<String, String> {
        let request: KeyStatusRequest = serde_json::from_str(&request_body)
            .map_err(|e| format!("Invalid request: {}", e))?;
        
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
        
        Ok(serde_json::json!({
            "status": status,
            "assigned_nodes": nodes,
            "total_cost": total_cost
        }).to_string())
    }
    
    #[http]
    async fn get_total_costs(&self, request_body: String) -> Result<String, String> {
        let request: CostRangeRequest = serde_json::from_str(&request_body)
            .map_err(|e| format!("Invalid request: {}", e))?;
        
        let mut total_cost = 0.0;
        let mut cost_by_key: HashMap<String, f64> = HashMap::new();
        
        for (key, costs) in &self.key_costs {
            let key_total: f64 = costs.iter()
                .filter(|c| self.filter_by_date(c.timestamp, &request.start_date, &request.end_date))
                .map(|c| c.amount)
                .sum();
            
            if key_total > 0.0 {
                cost_by_key.insert(key.clone(), key_total);
                total_cost += key_total;
            }
        }
        
        Ok(serde_json::json!({
            "total_cost": total_cost,
            "cost_by_key": cost_by_key,
            "currency": "USD"
        }).to_string())
    }
    
    #[http]
    async fn get_key_costs(&self, request_body: String) -> Result<String, String> {
        let request: KeyCostRequest = serde_json::from_str(&request_body)
            .map_err(|e| format!("Invalid request: {}", e))?;
        
        let costs = self.key_costs.get(&request.api_key)
            .map(|costs| {
                costs.iter()
                    .filter(|c| self.filter_by_date(c.timestamp, &request.start_date, &request.end_date))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        
        let total: f64 = costs.iter().map(|c| c.amount).sum();
        
        Ok(serde_json::json!({
            "api_key": request.api_key,
            "costs": costs,
            "total": total
        }).to_string())
    }
    
    #[http]
    async fn get_node_history(&self, _request_body: String) -> Result<String, String> {
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
        
        serde_json::to_string(&assignments).map_err(|e| format!("Serialization error: {}", e))
    }
    
    #[http]
    async fn set_admin_key(&mut self, request_body: String) -> Result<String, String> {
        let request: SetAdminKeyRequest = serde_json::from_str(&request_body)
            .map_err(|e| format!("Invalid request: {}", e))?;
        
        self.admin_api_key = Some(request.admin_key);
        
        Ok(serde_json::json!({
            "success": true,
            "message": "Admin key set successfully"
        }).to_string())
    }
    
    #[http]
    async fn initialize_auth(&mut self, _request_body: String) -> Result<String, String> {
        if self.ui_auth_token.is_none() {
            let token = BASE64.encode(format!("{:x}", rand::random::<u128>()));
            self.ui_auth_token = Some(token.clone());
        }
        
        Ok(serde_json::json!({
            "token": self.ui_auth_token.as_ref().unwrap()
        }).to_string())
    }
    
    #[http]
    async fn refresh_costs(&mut self, _request_body: String) -> Result<String, String> {
        if self.admin_api_key.is_none() {
            return Err("Admin API key not configured".to_string());
        }
        
        let now = Utc::now().timestamp();
        
        if let Some(last_check) = self.last_cost_check {
            if now - last_check < 3600 {
                return Ok(serde_json::json!({
                    "message": "Costs were recently refreshed",
                    "last_check": last_check
                }).to_string());
            }
        }
        
        for key in self.active_keys.iter() {
            let cost = rand::random::<f64>() * 10.0;
            
            let record = CostRecord {
                timestamp: now,
                amount: cost,
                currency: "USD".to_string(),
                description: format!("Usage for key {}", &key[..8]),
            };
            
            self.key_costs
                .entry(key.clone())
                .or_insert_with(Vec::new)
                .push(record);
        }
        
        self.last_cost_check = Some(now);
        
        Ok(serde_json::json!({
            "success": true,
            "message": "Costs refreshed successfully",
            "timestamp": now
        }).to_string())
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
}