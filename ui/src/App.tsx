import React, { useEffect, useState } from 'react';
import './App.css';
import { useApiKeyManagerStore } from './store/api-key-manager';
import * as api from '../../target/ui/caller-utils';
import { ChartDataPoint } from './types/api-key-manager';
import { CostRecord, NodeAssignment, ApiKeyInfo as ApiKey } from '../../target/ui/caller-utils';
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip,
  ResponsiveContainer, ReferenceLine, Label, Dot
} from 'recharts';

// Custom dot component to show node joins
const CustomDot = (props: any) => {
  const { cx, cy, payload } = props;
  if (payload.newNodes && payload.newNodes.length > 0) {
    return (
      <g transform={`translate(${cx},${cy})`}>
        <circle r="6" fill="#82ca9d" stroke="#fff" strokeWidth="2" />
        <text x="0" y="-10" textAnchor="middle" fontSize="10" fill="#82ca9d">
          +{payload.newNodes.length}
        </text>
      </g>
    );
  }
  return <circle cx={cx} cy={cy} r="3" fill="#8884d8" />;
};

// Custom tooltip with node information
const CustomTooltip = ({ active, payload, label }: any) => {
  if (active && payload && payload[0]) {
    const data = payload[0].payload;
    return (
      <div style={{ 
        backgroundColor: 'var(--bg-secondary)', 
        padding: '10px', 
        border: '1px solid var(--border-color)', 
        borderRadius: '4px',
        boxShadow: '0 2px 8px var(--shadow)'
      }}>
        <p style={{ margin: '0 0 5px 0', color: 'var(--text-primary)' }}>
          {`Date: ${new Date(data.timestamp * 1000).toLocaleString()}`}
        </p>
        <p style={{ margin: '0 0 5px 0', color: '#8884d8' }}>
          {`Cost: $${data.totalCost.toFixed(2)}`}
        </p>
        {data.newNodes && data.newNodes.length > 0 && (
          <div>
            <p style={{ margin: '5px 0', fontWeight: 'bold', color: 'var(--text-primary)' }}>New nodes:</p>
            {data.newNodes.map((node: string) => (
              <p key={node} style={{ 
                fontSize: '12px', 
                marginLeft: '10px', 
                margin: '2px 0',
                color: 'var(--text-secondary)'
              }}>• {node}</p>
            ))}
          </div>
        )}
      </div>
    );
  }
  return null;
};

// Modal Component for showing connected nodes
const ConnectedNodesModal: React.FC<{ isOpen: boolean; onClose: () => void }> = ({ isOpen, onClose }) => {
  const { apiKeys, nodeHistory } = useApiKeyManagerStore();
  
  if (!isOpen) return null;
  
  // Group nodes by API key
  const nodesByKey = new Map<string, { nodes: string[], keyInfo: ApiKey | undefined }>();
  
  nodeHistory.forEach(assignment => {
    if (!nodesByKey.has(assignment.api_key)) {
      const keyInfo = apiKeys.find(k => k.key === assignment.api_key);
      nodesByKey.set(assignment.api_key, { nodes: [], keyInfo });
    }
    const entry = nodesByKey.get(assignment.api_key);
    if (entry && !entry.nodes.includes(assignment.node_id)) {
      entry.nodes.push(assignment.node_id);
    }
  });
  
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Connected Nodes by API Key</h2>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>
        
        {nodesByKey.size === 0 ? (
          <p style={{ textAlign: 'center', color: 'var(--text-muted)' }}>No nodes connected yet</p>
        ) : (
          Array.from(nodesByKey.entries()).map(([key, data]) => (
            <div key={key} style={{ marginBottom: '1.5rem' }}>
              <div style={{ 
                padding: '0.75rem', 
                background: 'var(--code-bg)', 
                borderRadius: '4px',
                marginBottom: '0.5rem'
              }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <code style={{ fontSize: '0.875rem', color: 'var(--text-primary)' }}>
                    {key.substring(0, 20)}...
                  </code>
                  <span className={`status ${data.keyInfo?.status || 'unknown'}`}>
                    {data.keyInfo?.status || 'unknown'}
                  </span>
                </div>
              </div>
              <ul className="node-list">
                {data.nodes.map(nodeId => {
                  const assignment = nodeHistory.find(n => n.node_id === nodeId && n.api_key === key);
                  return (
                    <li key={nodeId} className="node-item">
                      <div className="node-info">
                        <div className="node-name">{nodeId}</div>
                        {assignment && (
                          <div className="node-date">
                            Connected: {new Date(assignment.issued_at * 1000).toLocaleString()}
                          </div>
                        )}
                      </div>
                    </li>
                  );
                })}
              </ul>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

// Dashboard Component
const Dashboard: React.FC = () => {
  const { apiKeys, nodeHistory, totalCosts, costData, loading, error } = useApiKeyManagerStore();
  const [chartData, setChartData] = useState<ChartDataPoint[]>([]);
  const [showNodesModal, setShowNodesModal] = useState(false);
  
  useEffect(() => {
    // Use real cost data if available
    const allCosts: CostRecord[] = [];
    
    // Aggregate all costs from the store
    if (apiKeys && apiKeys.length > 0) {
      apiKeys.forEach(key => {
        // In a real implementation, we'd fetch costs per key
        // For now, we'll use costData if available
      });
    }
    
    // Use costData from store or empty array
    const costs = costData && costData.length > 0 ? costData : [];
    const data = prepareChartData(costs, nodeHistory);
    setChartData(data);
  }, [costData, nodeHistory, apiKeys]);
  
  if (loading) return <div className="loading">Loading...</div>;
  if (error) return <div className="error">Error: {error}</div>;
  
  return (
    <div className="dashboard">
      <ConnectedNodesModal isOpen={showNodesModal} onClose={() => setShowNodesModal(false)} />
      
      <div className="stats-grid">
        <div className="stat-card">
          <h3>Total Keys</h3>
          <p className="stat-value">{apiKeys.length}</p>
        </div>
        <div className="stat-card">
          <h3>Total Cost</h3>
          <p className="stat-value">${totalCosts?.total_cost?.toFixed(2) || '0.00'}</p>
        </div>
        <div 
          className="stat-card clickable" 
          onClick={() => setShowNodesModal(true)}
          style={{ cursor: 'pointer' }}
        >
          <h3>Connected Nodes</h3>
          <p className="stat-value">{nodeHistory.length}</p>
        </div>
      </div>
      
      {chartData.length > 0 ? (
        <div className="chart-container">
          <h3>Cost Over Time</h3>
          <ResponsiveContainer width="100%" height={400}>
            <LineChart data={chartData} margin={{ top: 20, right: 30, left: 20, bottom: 20 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
              <XAxis 
                dataKey="timestamp"
                type="number"
                domain={['dataMin', 'dataMax']}
                tickFormatter={(timestamp) => new Date(timestamp * 1000).toLocaleDateString()}
                stroke="var(--chart-text)"
              />
              <YAxis 
                label={{ value: 'Total Cost ($)', angle: -90, position: 'insideLeft', fill: 'var(--chart-text)' }}
                stroke="var(--chart-text)"
              />
              <Tooltip content={<CustomTooltip />} />
              <Line 
                type="monotone" 
                dataKey="totalCost" 
                stroke="#8884d8" 
                strokeWidth={2}
                dot={<CustomDot />}
              />
              {/* Add vertical lines for node join events */}
              {nodeHistory.map((node, index) => (
                <ReferenceLine 
                  key={index}
                  x={node.issued_at} 
                  stroke="green" 
                  strokeDasharray="5 5"
                  opacity={0.5}
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        </div>
      ) : (
        <div className="chart-container">
          <p style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: '2rem' }}>
            No cost data available yet. Add API keys and refresh costs to see the chart.
          </p>
        </div>
      )}
    </div>
  );
};

// KeyList Component
const KeyList: React.FC = () => {
  const { apiKeys, setSelectedKey } = useApiKeyManagerStore();
  const [newKey, setNewKey] = useState('');
  
  const handleAddKey = async () => {
    if (!newKey.trim()) return;
    try {
      const response = await api.add_api_key({ api_key: newKey });
      if (!response.success) {
        throw new Error(response.message || 'Failed to add key');
      }
      setNewKey('');
      await refreshKeys();
    } catch (error) {
      console.error('Failed to add key:', error);
    }
  };
  
  const handleRemoveKey = async (key: string) => {
    try {
      const response = await api.remove_api_key({ api_key: key });
      if (!response.success) {
        throw new Error(response.message || 'Failed to remove key');
      }
      await refreshKeys();
    } catch (error) {
      console.error('Failed to remove key:', error);
    }
  };
  
  const refreshKeys = async () => {
    try {
      const response = await api.list_keys();
      const keys: ApiKey[] = response;
      useApiKeyManagerStore.getState().setApiKeys(keys);
    } catch (error) {
      console.error('Failed to refresh keys:', error);
    }
  };
  
  return (
    <div className="key-list">
      <div className="key-add-form">
        <input
          type="text"
          value={newKey}
          onChange={(e) => setNewKey(e.target.value)}
          placeholder="Enter API key..."
          className="key-input"
        />
        <button onClick={handleAddKey} className="btn btn-primary">Add Key</button>
      </div>
      
      <table className="key-table">
        <thead>
          <tr>
            <th>API Key</th>
            <th>Status</th>
            <th>Total Cost</th>
            <th>Nodes</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {apiKeys.map((key) => (
            <tr key={key.key}>
              <td className="key-cell">
                <code>{key.key.substring(0, 20)}...</code>
              </td>
              <td>
                <span className={`status ${key.status}`}>{key.status}</span>
              </td>
              <td>${key.total_cost.toFixed(2)}</td>
              <td>{key.assigned_nodes.length}</td>
              <td>
                <button 
                  onClick={() => setSelectedKey(key.key)}
                  className="btn btn-sm"
                >
                  View
                </button>
                <button 
                  onClick={() => handleRemoveKey(key.key)}
                  className="btn btn-sm btn-danger"
                >
                  Remove
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

// AdminPanel Component
const AdminPanel: React.FC = () => {
  const { adminKeySet } = useApiKeyManagerStore();
  const [adminKey, setAdminKey] = useState('');
  const [message, setMessage] = useState('');
  
  const handleSetAdminKey = async () => {
    if (!adminKey.trim()) return;
    
    const isUpdating = adminKeySet;
    
    try {
      const response = await api.set_admin_key({ admin_key: adminKey });
      if (!response.success) {
        throw new Error(response.message || 'Failed to set admin key');
      }
      setAdminKey('');
      setMessage(isUpdating ? 'Admin key updated successfully' : 'Admin key set successfully');
      useApiKeyManagerStore.getState().setAdminKeySet(true);
      
      // If updating an existing key, suggest refreshing costs
      if (isUpdating) {
        setTimeout(() => {
          setMessage('Admin key updated. You may want to refresh costs with the new key.');
        }, 2000);
      }
    } catch (error) {
      console.error('Error setting admin key:', error);
      setMessage(isUpdating ? 'Failed to update admin key' : 'Failed to set admin key');
    }
  };
  
  const handleRefreshCosts = async () => {
    try {
      const response = await api.refresh_costs();
      if (!response.success) {
        throw new Error(response.message || 'Failed to refresh costs');
      }
      setMessage('Costs refreshed successfully');
      await loadCosts();
      
      // Also load the cost data for charts
      await loadCostData();
    } catch (error) {
      setMessage('Failed to refresh costs');
    }
  };
  
  const handleResetCosts = async () => {
    if (!window.confirm('Are you sure you want to reset all cost data? This will clear all historical data and fetch fresh data from the API.')) {
      return;
    }
    
    try {
      const response = await api.reset_costs();
      if (!response.success) {
        throw new Error(response.message || 'Failed to reset costs');
      }
      setMessage('Cost data reset successfully. You can now refresh to fetch fresh data.');
      
      // Clear the local store data
      useApiKeyManagerStore.getState().setCostData([]);
      useApiKeyManagerStore.getState().setTotalCosts({
        total_cost: 0,
        cost_by_key: [],
        currency: 'USD'
      });
    } catch (error) {
      console.error('Failed to reset costs:', error);
      setMessage('Failed to reset cost data');
    }
  };
  
  const loadCosts = async () => {
    try {
      const response = await api.get_total_costs({ start_date: null, end_date: null });
      // Convert TotalCostsResponse to CostData format
      const costs = {
        total_cost: response.total_cost,
        cost_by_key: response.cost_by_key,
        currency: response.currency
      };
      useApiKeyManagerStore.getState().setTotalCosts(costs);
    } catch (error) {
      console.error('Failed to load costs:', error);
    }
  };
  
  const loadCostData = async () => {
    try {
      const response = await api.get_all_costs();
      const costData: CostRecord[] = response;
      useApiKeyManagerStore.getState().setCostData(costData);
    } catch (error) {
      console.error('Failed to load cost data:', error);
    }
  };
  
  return (
    <div className="admin-panel">
      <h2>Admin Settings</h2>
      
      <div className="admin-key-form">
        <h3>{adminKeySet ? 'Update Admin API Key' : 'Set Admin API Key'}</h3>
        <p>
          {adminKeySet 
            ? 'Enter a new Anthropic admin API key to replace the existing one'
            : 'Enter your Anthropic admin API key to enable cost tracking'}
        </p>
        <input
          type="password"
          value={adminKey}
          onChange={(e) => setAdminKey(e.target.value)}
          placeholder="sk-ant-..."
          className="key-input"
        />
        <button onClick={handleSetAdminKey} className="btn btn-primary">
          {adminKeySet ? 'Update Admin Key' : 'Set Admin Key'}
        </button>
        {adminKeySet && (
          <p style={{ marginTop: '0.5rem', fontSize: '0.875rem', color: 'var(--text-secondary)' }}>
            ✓ Admin key is currently set
          </p>
        )}
      </div>
      
      {adminKeySet && (
        <div className="admin-actions">
          <h3>Cost Management</h3>
          <div style={{ display: 'flex', gap: '1rem', marginBottom: '1rem' }}>
            <button onClick={handleRefreshCosts} className="btn btn-primary">
              Refresh Costs
            </button>
            <button onClick={handleResetCosts} className="btn btn-danger">
              Reset Plots
            </button>
          </div>
          {message && <p className="admin-message">{message}</p>}
        </div>
      )}
    </div>
  );
};

// Helper function to prepare chart data
function prepareChartData(costs: CostRecord[], nodeHistory: NodeAssignment[]): ChartDataPoint[] {
  if (!costs || costs.length === 0) {
    // Return empty data if no costs available
    return [];
  }
  
  // Group costs by timestamp and calculate cumulative totals
  const costsByTime = new Map<number, number>();
  costs.forEach(record => {
    const existing = costsByTime.get(record.timestamp) || 0;
    costsByTime.set(record.timestamp, existing + record.amount);
  });
  
  // Sort timestamps and create cumulative data points
  const sortedTimestamps = Array.from(costsByTime.keys()).sort((a, b) => a - b);
  const dataPoints: ChartDataPoint[] = [];
  let cumulativeCost = 0;
  
  sortedTimestamps.forEach(timestamp => {
    const cost = costsByTime.get(timestamp) || 0;
    cumulativeCost += cost;
    
    // Find nodes that joined around this time (within 1 hour window)
    const newNodes = nodeHistory
      .filter(n => Math.abs(n.issued_at - timestamp) < 3600)
      .map(n => n.node_id);
    
    dataPoints.push({
      timestamp,
      date: new Date(timestamp * 1000).toLocaleDateString(),
      totalCost: cumulativeCost,
      currency: 'USD',
      newNodes: newNodes.length > 0 ? newNodes : undefined
    });
  });
  
  return dataPoints;
}

// Main App Component
function App() {
  const { setLoading, setError, setApiKeys, setNodeHistory, setTotalCosts } = useApiKeyManagerStore();
  const [activeTab, setActiveTab] = useState<'dashboard' | 'keys' | 'admin'>('dashboard');
  
  useEffect(() => {
    // Initialize the app
    const init = async () => {
      setLoading(true);
      try {
        // Initialize auth and check admin key status
        const authResponse = await api.initialize_auth();
        const authData = authResponse;
        
        useApiKeyManagerStore.getState().setAuthToken(authData.token);
        
        // Set admin key status if provided
        if (authData.has_admin_key !== undefined) {
          useApiKeyManagerStore.getState().setAdminKeySet(authData.has_admin_key);
        }
        
        // Load initial data
        const [keysResponse, historyResponse] = await Promise.all([
          api.list_keys(),
          api.get_node_history()
        ]);
        
        const keys: ApiKey[] = keysResponse;
        const history: NodeAssignment[] = historyResponse;
        
        setApiKeys(keys);
        setNodeHistory(history);
        
        // Try to load costs if admin key is set
        if (authData.has_admin_key) {
          try {
            const [costsResponse, costDataResponse] = await Promise.all([
              api.get_total_costs({ start_date: null, end_date: null }),
              api.get_all_costs()
            ]);
            // Convert TotalCostsResponse to CostData format
            const costs = {
              total_cost: costsResponse.total_cost,
              cost_by_key: costsResponse.cost_by_key,
              currency: costsResponse.currency
            };
            const costData = costDataResponse;
            setTotalCosts(costs);
            useApiKeyManagerStore.getState().setCostData(costData);
          } catch (error) {
            console.log('Failed to load costs:', error);
          }
        }
      } catch (error) {
        setError(error instanceof Error ? error.message : 'Failed to initialize');
      } finally {
        setLoading(false);
      }
    };
    
    init();
  }, []);
  
  return (
    <div className="app">
      <header className="app-header">
        <h1>🔑 Anthropic API Key Manager</h1>
        <nav className="nav-tabs">
          <button 
            className={`tab ${activeTab === 'dashboard' ? 'active' : ''}`}
            onClick={() => setActiveTab('dashboard')}
          >
            Dashboard
          </button>
          <button 
            className={`tab ${activeTab === 'keys' ? 'active' : ''}`}
            onClick={() => setActiveTab('keys')}
          >
            API Keys
          </button>
          <button 
            className={`tab ${activeTab === 'admin' ? 'active' : ''}`}
            onClick={() => setActiveTab('admin')}
          >
            Admin
          </button>
        </nav>
      </header>
      
      <main className="app-main">
        {activeTab === 'dashboard' && <Dashboard />}
        {activeTab === 'keys' && <KeyList />}
        {activeTab === 'admin' && <AdminPanel />}
      </main>
    </div>
  );
}

export default App;