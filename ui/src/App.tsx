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
      <div style={{ backgroundColor: 'white', padding: '10px', border: '1px solid #ccc', borderRadius: '4px' }}>
        <p style={{ margin: '0 0 5px 0' }}>{`Date: ${new Date(data.timestamp * 1000).toLocaleString()}`}</p>
        <p style={{ margin: '0 0 5px 0', color: '#8884d8' }}>{`Cost: $${data.totalCost.toFixed(2)}`}</p>
        {data.newNodes && data.newNodes.length > 0 && (
          <div>
            <p style={{ margin: '5px 0', fontWeight: 'bold' }}>New nodes:</p>
            {data.newNodes.map((node: string) => (
              <p key={node} style={{ fontSize: '12px', marginLeft: '10px', margin: '2px 0' }}>• {node}</p>
            ))}
          </div>
        )}
      </div>
    );
  }
  return null;
};

// Dashboard Component
const Dashboard: React.FC = () => {
  const { apiKeys, nodeHistory, totalCosts, costData, loading, error } = useApiKeyManagerStore();
  const [chartData, setChartData] = useState<ChartDataPoint[]>([]);
  
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
      <div className="stats-grid">
        <div className="stat-card">
          <h3>Total Keys</h3>
          <p className="stat-value">{apiKeys.length}</p>
        </div>
        <div className="stat-card">
          <h3>Total Cost</h3>
          <p className="stat-value">${totalCosts?.total_cost?.toFixed(2) || '0.00'}</p>
        </div>
        <div className="stat-card">
          <h3>Connected Nodes</h3>
          <p className="stat-value">{nodeHistory.length}</p>
        </div>
      </div>
      
      {chartData.length > 0 ? (
        <div className="chart-container">
          <h3>Cost Over Time</h3>
          <ResponsiveContainer width="100%" height={400}>
            <LineChart data={chartData} margin={{ top: 20, right: 30, left: 20, bottom: 20 }}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis 
                dataKey="timestamp"
                type="number"
                domain={['dataMin', 'dataMax']}
                tickFormatter={(timestamp) => new Date(timestamp * 1000).toLocaleDateString()}
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
              {/* Add vertical lines for node join events */}
              {nodeHistory.map((node, index) => (
                <ReferenceLine 
                  key={index}
                  x={node.issuedAt} 
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
          <p style={{ textAlign: 'center', color: '#666', padding: '2rem' }}>
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
      const response = await api.addApiKey({ apiKey: newKey });
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
      const response = await api.removeApiKey({ apiKey: key });
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
      const response = await api.listKeys();
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
              <td>${key.totalCost.toFixed(2)}</td>
              <td>{key.assignedNodes.length}</td>
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
    try {
      const response = await api.setAdminKey({ adminKey });
      if (!response.success) {
        throw new Error(response.message || 'Failed to set admin key');
      }
      setAdminKey('');
      setMessage('Admin key set successfully');
      useApiKeyManagerStore.getState().setAdminKeySet(true);
    } catch (error) {
      console.error('Error setting admin key:', error);
      setMessage('Failed to set admin key');
    }
  };
  
  const handleRefreshCosts = async () => {
    try {
      const response = await api.refreshCosts();
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
  
  const loadCosts = async () => {
    try {
      const response = await api.getTotalCosts({ startDate: null, endDate: null });
      // Convert TotalCostsResponse to CostData format
      const costs = {
        total_cost: response.totalCost,
        cost_by_key: response.costByKey,
        currency: response.currency
      };
      useApiKeyManagerStore.getState().setTotalCosts(costs);
    } catch (error) {
      console.error('Failed to load costs:', error);
    }
  };
  
  const loadCostData = async () => {
    try {
      const response = await api.getAllCosts();
      const costData: CostRecord[] = response;
      useApiKeyManagerStore.getState().setCostData(costData);
    } catch (error) {
      console.error('Failed to load cost data:', error);
    }
  };
  
  return (
    <div className="admin-panel">
      <h2>Admin Settings</h2>
      
      {!adminKeySet && (
        <div className="admin-key-form">
          <h3>Set Admin API Key</h3>
          <p>Enter your Anthropic admin API key to enable cost tracking</p>
          <input
            type="password"
            value={adminKey}
            onChange={(e) => setAdminKey(e.target.value)}
            placeholder="sk-ant-..."
            className="key-input"
          />
          <button onClick={handleSetAdminKey} className="btn btn-primary">
            Set Admin Key
          </button>
        </div>
      )}
      
      {adminKeySet && (
        <div className="admin-actions">
          <h3>Cost Management</h3>
          <button onClick={handleRefreshCosts} className="btn btn-primary">
            Refresh Costs
          </button>
          <p className="admin-message">{message}</p>
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
      .filter(n => Math.abs(n.issuedAt - timestamp) < 3600)
      .map(n => n.nodeId);
    
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
        const authResponse = await api.initializeAuth();
        const authData = authResponse;
        
        useApiKeyManagerStore.getState().setAuthToken(authData.token);
        
        // Set admin key status if provided
        if (authData.hasAdminKey !== undefined) {
          useApiKeyManagerStore.getState().setAdminKeySet(authData.hasAdminKey);
        }
        
        // Load initial data
        const [keysResponse, historyResponse] = await Promise.all([
          api.listKeys(),
          api.getNodeHistory()
        ]);
        
        const keys: ApiKey[] = keysResponse;
        const history: NodeAssignment[] = historyResponse;
        
        setApiKeys(keys);
        setNodeHistory(history);
        
        // Try to load costs if admin key is set
        if (authData.hasAdminKey) {
          try {
            const [costsResponse, costDataResponse] = await Promise.all([
              api.getTotalCosts({ startDate: null, endDate: null }),
              api.getAllCosts()
            ]);
            // Convert TotalCostsResponse to CostData format
            const costs = {
              total_cost: costsResponse.totalCost,
              cost_by_key: costsResponse.costByKey,
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