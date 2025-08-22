## Anthropic limited API key manager

### Code references

#### Manage anthropic workspaces
```
# Create workspace
curl --request POST "https://api.anthropic.com/v1/organizations/workspaces" \
  --header "anthropic-version: 2023-06-01" \
  --header "x-api-key: $ANTHROPIC_ADMIN_KEY" \
  --data '{"name": "Production"}'

# List workspaces
curl "https://api.anthropic.com/v1/organizations/workspaces?limit=10&include_archived=false" \
  --header "anthropic-version: 2023-06-01" \
  --header "x-api-key: $ANTHROPIC_ADMIN_KEY"

# Archive workspace
curl --request POST "https://api.anthropic.com/v1/organizations/workspaces/{workspace_id}/archive" \
  --header "anthropic-version: 2023-06-01" \
  --header "x-api-key: $ANTHROPIC_ADMIN_KEY"
```

#### Manage API keys
```
# List API keys
curl "https://api.anthropic.com/v1/organizations/api_keys?limit=10&status=active&workspace_id=wrkspc_xxx" \
  --header "anthropic-version: 2023-06-01" \
  --header "x-api-key: $ANTHROPIC_ADMIN_KEY"

# Update API key
curl --request POST "https://api.anthropic.com/v1/organizations/api_keys/{api_key_id}" \
  --header "anthropic-version: 2023-06-01" \
  --header "x-api-key: $ANTHROPIC_ADMIN_KEY" \
  --data '{
    "status": "inactive",
    "name": "New Key Name"
  }'
```

#### Read cost
Request:
```
curl "https://api.anthropic.com/v1/organizations/cost_report\
?starting_at=2025-08-01T00:00:00Z\
&group_by[]=workspace_id\
&group_by[]=description\
&limit=1" \
  --header "anthropic-version: 2023-06-01" \
  --header "content-type: application/json" \
  --header "x-api-key: $ANTHROPIC_ADMIN_KEY"
```
Response:
```
{
  "data": [
    {
      "starting_at": "2025-08-01T00:00:00Z",
      "ending_at": "2025-08-02T00:00:00Z",
      "results": [
        {
          "currency": "USD",
          "amount": "123.78912",
          "workspace_id": "wrkspc_01JwQvzr7rXLA5AGx3HKfFUJ",
          "description": "Claude Sonnet 4 Usage - Input Tokens",
          "cost_type": "tokens",
          "context_window": "0-200k",
          "model": "claude-sonnet-4-20250514",
          "service_tier": "standard",
          "token_type": "uncached_input_tokens"
        }
      ]
    }
  ],
  "has_more": true,
  "next_page": "page_MjAyNS0wNS0xNFQwMDowMDowMFo="
}
```

#### Send HTTP requests using
```
// hyperware_process_lib src/http/client.rs

/// Make an HTTP request using http-client and await its response.
///
/// Returns HTTP response from the `http` crate if successful, with the body type as bytes.
#[cfg(feature = "hyperapp")]
pub async fn send_request_await_response(
    method: Method,
    url: url::Url,
    headers: Option<HashMap<String, String>>,
    timeout: u64,
    body: Vec<u8>,
) -> std::result::Result<http::Response<Vec<u8>>, HttpClientError> {
```

### Goal/Prompt

anthropic-api-key-manager is a hyperapp that manages anthropic API keys that are very limited in their spend and deletes them after a specified period of time.

It does this by using the Anthropic Admin API.

It maintains a set of anthropic API keys in a HashSet.
There is a HashSet of active keys and a HashSet of historical keys that are no longer active

It maintains a HashMap of api_key : Vec<node> (where node is a String: a node identity like `foo.os`) that it has issued API keys to.
It maintains a HashMap of node : timestamp and will only issue an API key for a node once (i.e. if it is not in the HashSet).

A node can request an api key via a p2p hyperware message.
If that node is not in the HashSet of nodes, the key manager randomly selects one of the active keys
It then send that key in p2p response to the requestor; adds that node to the HashMap of node : timestamp (now: i.e., when key was issued); adds that node to the HashMap of api_key : node

It queries the anthropic api for cost of all active keys every hour and logs in a HashMap of key : HashMap of time : cost (or some other more convenient structure that stores the same data)

It serves an authenticated UI that allows managing API keys: adding, removing; reviewing costs.
It shows a plot of total cost vs time
On that plot, also show when nodes have joined (and their node name)
Any api key can be selected and it will show the total cost vs time
On that plot, show when nodes for that api key have joined (and their node name)
