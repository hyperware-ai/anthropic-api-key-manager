# Anthropic API Key Manager - Implementation Plan

## Overview
This document outlines the implementation plan for an Anthropic API key manager hyperapp that manages limited-spend API keys, distributes them to nodes via P2P messaging, tracks usage costs, and provides a web UI for administration.

## Application Architecture

### Core Components
1. **Backend (Rust)**: Manages API keys, handles P2P requests, queries Anthropic API for costs
2. **Frontend (React/TypeScript)**: Admin UI for key management and cost visualization
3. **P2P Interface**: Handles key distribution to requesting nodes
4. **Cost Tracking**: Periodic polling of Anthropic API for usage metrics

## Implementation Steps

### Phase 1: Backend Core Structure

#### 1.1 Rename Application
- Update all references from `skeleton-app` to `anthropic-api-key-manager`
- Update `metadata.json` with appropriate app name and description
- Update package names in `Cargo.toml` files

#### 1.2 Define State Structure
Create the main application state in `anthropic-api-key-manager/src/lib.rs`:

```rust
#[derive(Default, Serialize, Deserialize)]
pub struct ApiKeyManagerState {
    // Anthropic admin API key (for managing other keys)
    admin_api_key: Option<String>,
    
    // Active API keys that can be distributed
    active_keys: HashSet<String>,
    
    // Historical keys that are no longer active
    historical_keys: HashSet<String>,
    
    // Maps API key to list of nodes that have been issued this key
    key_to_nodes: HashMap<String, Vec<String>>,
    
    // Maps node ID to timestamp when key was issued
    node_issue_times: HashMap<String, i64>,
    
    // Cost tracking: key -> timestamp -> cost
    key_costs: HashMap<String, Vec<CostRecord>>,
    
    // Last time costs were checked
    last_cost_check: Option<i64>,
    
    // Authentication token for UI access
    ui_auth_token: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct CostRecord {
    timestamp: i64,
    amount: f64,
    currency: String,
    description: String,
}
```

#### 1.3 Configure Hyperprocess Macro
```rust
#[hyperprocess(
    name = "anthropic-api-key-manager",
    ui = Some(HttpBindingConfig::default()),
    endpoints = vec![
        Binding::Http {
            path: "/api",
            config: HttpBindingConfig::default().authenticated(true),
        },
        Binding::Ws {
            path: "/ws",
            config: WsBindingConfig::default().authenticated(true),
        }
    ],
    save_config = SaveOptions::OnDiff,
    wit_world = "anthropic-api-key-manager-v0",
)]
```

### Phase 2: P2P Key Distribution

#### 2.1 Remote Endpoint for Key Requests
```rust
#[remote]
async fn request_api_key(&mut self, node_id: String) -> Result<String, String>
```
- Check if node already has a key
- If not, randomly select an active key
- Record the assignment
- Return the key

#### 2.2 Local Message Handler
Implement handling for incoming P2P messages requesting API keys.

### Phase 3: Anthropic API Integration

#### 3.1 HTTP Client Setup
Use `hyperware_process_lib::http::client::send_request_await_response` for making HTTP requests to Anthropic API.

#### 3.2 API Methods
Implement methods for:
- Creating workspaces
- Listing API keys
- Updating API key status
- Fetching cost reports

#### 3.3 Cost Polling
Create a periodic task (every hour) that:
- Queries cost for all active keys
- Stores results in state
- Calculates aggregate metrics

### Phase 4: HTTP API Endpoints

#### 4.1 Key Management Endpoints
```rust
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

#[http(method = "POST", path = "/api/keys/add")]
async fn add_api_key(&mut self, request: AddKeyRequest) -> Result<String, String>

#[http(method = "POST", path = "/api/keys/remove")]
async fn remove_api_key(&mut self, request: RemoveKeyRequest) -> Result<String, String>

#[http(method = "GET", path = "/api/keys")]
async fn list_keys(&self) -> Result<String, String>

#[http(method = "POST", path = "/api/keys/status")]
async fn get_key_status(&self, request: KeyStatusRequest) -> Result<String, String>
```

#### 4.2 Cost Analytics Endpoints
```rust
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

#[http(method = "POST", path = "/api/costs/total")]
async fn get_total_costs(&self, request: CostRangeRequest) -> Result<String, String>

#[http(method = "POST", path = "/api/costs/by-key")]
async fn get_key_costs(&self, request: KeyCostRequest) -> Result<String, String>

#[http(method = "GET", path = "/api/nodes/history")]
async fn get_node_history(&self) -> Result<String, String>
```

#### 4.3 Admin Endpoints
```rust
#[derive(Serialize, Deserialize)]
struct SetAdminKeyRequest {
    admin_key: String,
}

#[http(method = "POST", path = "/api/admin/set-key")]
async fn set_admin_key(&mut self, request: SetAdminKeyRequest) -> Result<String, String>

#[http(method = "GET", path = "/api/admin/init-auth")]
async fn initialize_auth(&mut self) -> Result<String, String>
```

### Phase 5: Frontend Implementation

#### 5.1 Update Package Structure
- Rename UI components from skeleton references
- Update `package.json` with appropriate name

#### 5.2 Type Definitions
Create types in `ui/src/types/api-key-manager.ts`:
```typescript
interface ApiKey {
  key: string;
  status: 'active' | 'inactive';
  totalCost: number;
  assignedNodes: string[];
  createdAt: string;
}

interface CostData {
  timestamp: number;
  amount: number;
  currency: string;
  description: string;
}

interface NodeAssignment {
  nodeId: string;
  apiKey: string;
  issuedAt: number;
}
```

#### 5.3 Zustand Store
Create state management in `ui/src/store/api-key-manager.ts`:
- API keys list
- Cost data
- Node assignments
- UI state (loading, errors, filters)

#### 5.4 API Service
Create API utilities in `ui/src/utils/api.ts`:
```typescript
// Key management functions
export async function addApiKey(apiKey: string) {
  const response = await fetch('/api/keys/add', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ AddApiKey: { api_key: apiKey } })
  });
  return response.json();
}

export async function listKeys() {
  const response = await fetch('/api/keys', {
    method: 'GET',
    headers: { 'Content-Type': 'application/json' }
  });
  return response.json();
}

// Cost fetching functions
export async function getTotalCosts(startDate?: string, endDate?: string) {
  const response = await fetch('/api/costs/total', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ 
      GetTotalCosts: { 
        start_date: startDate || null,
        end_date: endDate || null
      }
    })
  });
  return response.json();
}
```

#### 5.5 UI Components
Create React components:
- **Dashboard**: Overview of all keys and total costs
- **KeyList**: Table of API keys with status and actions
- **CostChart**: Visualization of costs over time using a charting library
- **NodeHistory**: Timeline showing when nodes joined
- **KeyDetails**: Detailed view for individual key with its cost chart
- **AdminPanel**: Settings and admin key configuration

#### 5.6 Cost Visualization with Recharts

**Library Choice: Recharts**
- Lightweight, React-specific library with excellent TypeScript support
- Component-based API that fits naturally with React patterns
- Built-in support for time series, annotations, and responsive design
- Over 1 million weekly downloads, well-maintained

**Installation:**
```bash
npm install recharts
npm install --save-dev @types/recharts
```

**Implementation Guide:**

1. **Data Preparation**
```typescript
// Transform backend data for Recharts
interface ChartDataPoint {
  timestamp: number;  // Unix timestamp
  date: string;       // Formatted date for display
  totalCost: number;
  currency: string;
  // For tracking which nodes joined at this time
  newNodes?: string[];
}

function prepareChartData(costs: CostRecord[], nodeHistory: NodeAssignment[]): ChartDataPoint[] {
  // Group costs by timestamp
  const costsByTime = new Map<number, number>();
  costs.forEach(record => {
    const existing = costsByTime.get(record.timestamp) || 0;
    costsByTime.set(record.timestamp, existing + record.amount);
  });
  
  // Create data points with cumulative costs
  let cumulativeCost = 0;
  const dataPoints: ChartDataPoint[] = [];
  
  Array.from(costsByTime.entries())
    .sort(([a], [b]) => a - b)
    .forEach(([timestamp, cost]) => {
      cumulativeCost += cost;
      
      // Find nodes that joined at this timestamp
      const newNodes = nodeHistory
        .filter(n => Math.abs(n.issuedAt - timestamp) < 3600000) // Within 1 hour
        .map(n => n.nodeId);
      
      dataPoints.push({
        timestamp,
        date: new Date(timestamp).toLocaleDateString(),
        totalCost: cumulativeCost,
        currency: 'USD',
        newNodes: newNodes.length > 0 ? newNodes : undefined
      });
    });
  
  return dataPoints;
}
```

2. **Total Cost Chart Component**
```typescript
import React from 'react';
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip,
  ResponsiveContainer, ReferenceLine, Label, Dot
} from 'recharts';

interface TotalCostChartProps {
  data: ChartDataPoint[];
  nodeJoinEvents: Array<{ timestamp: number; nodeId: string }>;
}

export const TotalCostChart: React.FC<TotalCostChartProps> = ({ data, nodeJoinEvents }) => {
  // Custom dot to highlight when nodes joined
  const CustomDot = (props: any) => {
    const { cx, cy, payload } = props;
    if (payload.newNodes && payload.newNodes.length > 0) {
      return (
        <g transform={`translate(${cx},${cy})`}>
          <circle r="6" fill="#8884d8" stroke="#fff" strokeWidth="2" />
          <text x="0" y="-10" textAnchor="middle" fontSize="10">
            +{payload.newNodes.length}
          </text>
        </g>
      );
    }
    return <circle cx={cx} cy={cy} r="3" fill="#8884d8" />;
  };

  // Custom tooltip showing details
  const CustomTooltip = ({ active, payload, label }: any) => {
    if (active && payload && payload[0]) {
      const data = payload[0].payload;
      return (
        <div style={{ backgroundColor: 'white', padding: '10px', border: '1px solid #ccc' }}>
          <p>{`Date: ${data.date}`}</p>
          <p>{`Cost: $${data.totalCost.toFixed(2)}`}</p>
          {data.newNodes && (
            <div>
              <p>New nodes:</p>
              {data.newNodes.map((node: string) => (
                <p key={node} style={{ fontSize: '12px', marginLeft: '10px' }}>• {node}</p>
              ))}
            </div>
          )}
        </div>
      );
    }
    return null;
  };

  return (
    <ResponsiveContainer width="100%" height={400}>
      <LineChart data={data} margin={{ top: 20, right: 30, left: 20, bottom: 20 }}>
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis 
          dataKey="timestamp"
          type="number"
          domain={['dataMin', 'dataMax']}
          tickFormatter={(timestamp) => new Date(timestamp).toLocaleDateString()}
        />
        <YAxis 
          label={{ value: 'Total Cost ($)', angle: -90, position: 'insideLeft' }}
        />
        <Tooltip content={<CustomTooltip />} />
        
        <Line 
          type="monotone" 
          dataKey="totalCost" 
          stroke="#8884d8" 
          strokeWidth={2}
          dot={<CustomDot />}
        />
        
        {/* Add vertical lines for significant events */}
        {nodeJoinEvents.map((event, index) => (
          <ReferenceLine 
            key={index}
            x={event.timestamp} 
            stroke="green" 
            strokeDasharray="5 5"
            label={<Label value={`${event.nodeId} joined`} position="top" />}
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  );
};
```

3. **Individual API Key Cost Chart**
```typescript
interface KeyCostChartProps {
  apiKey: string;
  data: ChartDataPoint[];
  nodeAssignments: Array<{ timestamp: number; nodeId: string }>;
}

export const KeyCostChart: React.FC<KeyCostChartProps> = ({ apiKey, data, nodeAssignments }) => {
  return (
    <ResponsiveContainer width="100%" height={300}>
      <LineChart data={data} margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis 
          dataKey="timestamp"
          type="number"
          domain={['dataMin', 'dataMax']}
          tickFormatter={(timestamp) => new Date(timestamp).toLocaleDateString()}
        />
        <YAxis />
        <Tooltip 
          labelFormatter={(timestamp) => new Date(timestamp as number).toLocaleString()}
          formatter={(value: number) => [`$${value.toFixed(2)}`, 'Cost']}
        />
        
        <Line 
          type="monotone" 
          dataKey="totalCost" 
          stroke="#82ca9d" 
          strokeWidth={2}
          name={`Key: ${apiKey.substring(0, 8)}...`}
        />
        
        {/* Mark when nodes were assigned this key */}
        {nodeAssignments.map((assignment, index) => (
          <ReferenceLine 
            key={index}
            x={assignment.timestamp} 
            stroke="orange" 
            strokeDasharray="3 3"
            label={<Label value={assignment.nodeId} position="top" fontSize={10} />}
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  );
};
```

4. **Usage in Dashboard Component**
```typescript
import { TotalCostChart, KeyCostChart } from './components/Charts';

export const Dashboard: React.FC = () => {
  const { costData, nodeHistory, selectedKey } = useAppStore();
  
  const chartData = useMemo(() => 
    prepareChartData(costData, nodeHistory), 
    [costData, nodeHistory]
  );
  
  return (
    <div>
      <h2>Total Cost Over Time</h2>
      <TotalCostChart 
        data={chartData}
        nodeJoinEvents={nodeHistory}
      />
      
      {selectedKey && (
        <>
          <h3>Cost for Key: {selectedKey}</h3>
          <KeyCostChart 
            apiKey={selectedKey}
            data={chartData.filter(d => /* filter for specific key */)}
            nodeAssignments={nodeHistory.filter(n => n.apiKey === selectedKey)}
          />
        </>
      )}
    </div>
  );
};
```

**Key Features Implemented:**
- Time series line charts with proper timestamp handling
- Custom dots showing when nodes joined
- Reference lines marking significant events
- Custom tooltips with detailed information
- Responsive design that adapts to container size
- Cumulative cost tracking
- Individual key cost breakdown with node assignment markers

### Phase 6: Authentication & Security

#### 6.1 UI Authentication
- Generate secure token on first run
- Store in app state
- Require token for all HTTP endpoints
- Provide mechanism to reset/regenerate token

#### 6.2 Admin Key Security
- Encrypt admin API key in state
- Never expose in logs or responses
- Validate admin key before making Anthropic API calls

### Phase 7: Build Configuration

#### 7.1 Update Manifest
Edit `pkg/manifest.json`:
```json
{
  "name": "anthropic-api-key-manager",
  "description": "Manages and distributes limited Anthropic API keys",
  "request_capabilities": [
    "homepage:homepage:sys",
    "http-server:distro:sys",
    "vfs:distro:sys"
  ],
  "request_networking": true,
  "public": false
}
```

#### 7.2 Dependencies
Add required crates to `Cargo.toml`:
- `chrono` for timestamp handling
- `rand` for random key selection
- `url` for URL parsing
- `base64` for encoding

### Phase 8: Testing & Deployment

#### 8.1 Test P2P Communication
- Set up multiple test nodes
- Test key request/response flow
- Verify node tracking

#### 8.2 Test Anthropic API Integration
- Test with real admin API key
- Verify cost fetching
- Test key management operations

#### 8.3 UI Testing
- Test authentication flow
- Verify cost charts render correctly
- Test all CRUD operations

## Implementation Order

1. **Start with backend structure** - Set up state and basic hyperprocess
2. **Add P2P messaging** - Implement key distribution logic
3. **Add HTTP endpoints** - Create admin API
4. **Build with `kit build --hyperapp`** - Generate API bindings
5. **Implement frontend** - Create UI using generated bindings
6. **Add Anthropic API integration** - Connect to real API
7. **Add cost tracking and visualization** - Complete analytics features
8. **Test end-to-end** - Verify all functionality

## Key Implementation Notes

### For the Implementor:
1. Start by renaming the skeleton app throughout the codebase
2. Implement backend first, as the UI depends on generated bindings
3. Use JSON strings for complex data types to avoid WIT limitations
4. Remember to include `/our.js` in index.html
5. Set proper timeouts on all remote requests
6. Use the examples in `resources/example-apps/` for P2P patterns
7. Test P2P functionality with fake nodes before deployment
8. Ensure admin API key is properly secured and never logged

### Data Flow:
1. Admin sets up admin API key via UI
2. Admin adds active API keys to the pool
3. Nodes request keys via P2P messages
4. System tracks assignments and prevents duplicates
5. Hourly job fetches costs from Anthropic
6. UI displays cost analytics and node history

### Error Handling:
- Gracefully handle Anthropic API failures
- Retry logic for cost fetching
- Clear error messages in UI
- Logging for debugging P2P issues

This implementation plan provides a complete blueprint for building the Anthropic API key manager as a hyperapp.