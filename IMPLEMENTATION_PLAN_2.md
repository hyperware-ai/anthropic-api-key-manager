# Anthropic API Key Manager - Implementation Plan Phase 2

## Current Implementation Status

The implementor has completed the initial backend structure and basic UI framework. The following components are in place:

### ✅ Completed:
1. **Backend Core Structure**
   - State management with all required fields
   - Basic HTTP endpoints for key management
   - P2P remote endpoint for key distribution
   - Authentication token generation

2. **Frontend Framework**
   - React UI with three tabs (Dashboard, Keys, Admin)
   - Zustand store for state management
   - API utility functions
   - Basic charting setup with Recharts

3. **Build Configuration**
   - Proper hyperprocess macro configuration
   - WIT world setup
   - UI bindings

## 🚧 Remaining Work

### Critical Missing Features

#### 1. **Anthropic API Integration (HIGH PRIORITY)**
The current implementation has **stub data** for cost tracking. The `refresh_costs` function only generates random data:

```rust
// Line 344-351 in lib.rs - THIS IS A STUB!
for key in self.active_keys.iter() {
    let cost = rand::random::<f64>() * 10.0;  // FAKE DATA
    // ...
}
```

**TODO:**
- Implement actual HTTP client calls to Anthropic API
- Add methods for:
  - Creating workspaces
  - Listing API keys
  - Updating key status
  - Fetching real cost reports
- Use `hyperware_process_lib::http::client::send_request_await_response`

#### 2. **HTTP Handler Pattern Update (MEDIUM PRIORITY)**
Current implementation uses the legacy pattern with `request_body: String`:

```rust
#[http]
async fn add_api_key(&mut self, request_body: String) -> Result<String, String>
```

**TODO:**
- Update to modern pattern: `#[http(method = "POST", path = "/api/keys/add")]`
- Remove `request_body` parameters
- Use typed request structs directly

#### 3. **Periodic Cost Polling (HIGH PRIORITY)**
No automatic hourly polling is implemented.

**TODO:**
- Add timer capability request to manifest
- Implement periodic task using timer (use `hyperware_process_lib::hyperapp::sleep(time_in_ms: u64).await;` or `hyperware_process_lib::timer::set_timer(duration: u64, context: Option<Context>)` and handle the Response from `timer:distro:sys`)
- Query costs every hour automatically
- Store historical cost data

#### 4. **Real Chart Data (MEDIUM PRIORITY)**
Current charts use simulated data:

```typescript
// Line 238-257 in App.tsx - FAKE DATA!
function prepareChartData() {
    // Simulate some data for demonstration
    for (let i = 0; i < 10; i++) {
        cumulativeCost += Math.random() * 50;
    }
}
```

**TODO:**
- Connect charts to real cost data from backend
- Add node join markers on timeline
- Implement individual key cost charts
- Add proper data filtering by date range

#### 5. **Error Handling & Resilience (MEDIUM PRIORITY)**
Limited error handling throughout.

**TODO:**
- Add retry logic for Anthropic API calls
- Better error messages in UI
- Graceful degradation when API unavailable
- Logging for debugging

## Key Files Requiring Changes

### Backend (`anthropic-api-key-manager/src/lib.rs`):
- Lines 327-366: Replace stub `refresh_costs` with real API integration
- Lines 140-312: Update HTTP handlers to modern pattern
- Add new timer-based periodic polling
- Add key expiration logic

### Frontend (`ui/src/App.tsx`):
- Lines 238-257: Replace fake chart data with real data
- Add individual key cost charts
- Add node join markers to charts
- Add expiration controls to KeyList

### API Utils (`ui/src/utils/api.ts`):
- Already properly structured, minimal changes needed

## Notes for Implementor

⚠️ **CRITICAL STUBS TO REPLACE:**
1. `refresh_costs` function (line 327-366) - Currently generates random data
2. `prepareChartData` function in App.tsx (line 238-257) - Currently generates fake chart data

💡 **TIPS:**
- Start with Anthropic API integration - everything else depends on real data
- Use the examples in `instructions.md` for Anthropic API curl commands
- Test with fake nodes first before real deployment
- Remember to add `timer:distro:sys` capability for periodic polling

## Success Criteria

The implementation will be complete when:
- [ ] Real costs are fetched from Anthropic API
- [ ] Keys automatically expire and are deleted
- [ ] Charts show actual cost data with node markers
- [ ] Costs are automatically refreshed hourly
- [ ] All HTTP handlers use modern pattern
- [ ] Authentication is properly enforced
- [ ] Error handling is robust
- [ ] P2P key distribution is tested and working
