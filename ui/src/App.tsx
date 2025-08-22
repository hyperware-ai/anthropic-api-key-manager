import React, { useEffect, useState } from 'react';
import './App.css';
import { useApiKeyManagerStore } from './store/api-key-manager';
import * as api from './utils/api';
import { ChartDataPoint, CostRecord, NodeAssignment } from './types/api-key-manager';
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip,
  ResponsiveContainer
} from 'recharts';

// Dashboard Component
const Dashboard: React.FC = () => {
  const { apiKeys, nodeHistory, totalCosts, loading, error } = useApiKeyManagerStore();
  const [chartData, setChartData] = useState<ChartDataPoint[]>([]);
  
  useEffect(() => {
    if (totalCosts && nodeHistory) {
      const data = prepareChartData([], nodeHistory);
      setChartData(data);
    }
  }, [totalCosts, nodeHistory]);
  
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
      
      {chartData.length > 0 && (
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
              <Tooltip 
                labelFormatter={(timestamp) => new Date((timestamp as number) * 1000).toLocaleString()}
                formatter={(value: number) => [`$${value.toFixed(2)}`, 'Cost']}
              />
              <Line 
                type="monotone" 
                dataKey="totalCost" 
                stroke="#8884d8" 
                strokeWidth={2}
              />
            </LineChart>
          </ResponsiveContainer>
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
      await api.addApiKey(newKey);
      setNewKey('');
      await refreshKeys();
    } catch (error) {
      console.error('Failed to add key:', error);
    }
  };
  
  const handleRemoveKey = async (key: string) => {
    try {
      await api.removeApiKey(key);
      await refreshKeys();
    } catch (error) {
      console.error('Failed to remove key:', error);
    }
  };
  
  const refreshKeys = async () => {
    try {
      const keys = await api.listKeys();
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
    try {
      await api.setAdminKey(adminKey);
      setAdminKey('');
      setMessage('Admin key set successfully');
      useApiKeyManagerStore.getState().setAdminKeySet(true);
    } catch (error) {
      setMessage('Failed to set admin key');
    }
  };
  
  const handleRefreshCosts = async () => {
    try {
      await api.refreshCosts();
      setMessage('Costs refreshed successfully');
      await loadCosts();
    } catch (error) {
      setMessage('Failed to refresh costs');
    }
  };
  
  const loadCosts = async () => {
    try {
      const costs = await api.getTotalCosts();
      useApiKeyManagerStore.getState().setTotalCosts(costs);
    } catch (error) {
      console.error('Failed to load costs:', error);
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
function prepareChartData(_costs: CostRecord[], _nodeHistory: NodeAssignment[]): ChartDataPoint[] {
  const dataPoints: ChartDataPoint[] = [];
  let cumulativeCost = 0;
  
  // Simulate some data for demonstration
  const now = Date.now() / 1000;
  for (let i = 0; i < 10; i++) {
    const timestamp = now - (10 - i) * 86400; // Past 10 days
    cumulativeCost += Math.random() * 50;
    
    dataPoints.push({
      timestamp,
      date: new Date(timestamp * 1000).toLocaleDateString(),
      totalCost: cumulativeCost,
      currency: 'USD',
    });
  }
  
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
        // Initialize auth
        const token = await api.initializeAuth();
        useApiKeyManagerStore.getState().setAuthToken(token);
        
        // Load initial data
        const [keys, history] = await Promise.all([
          api.listKeys(),
          api.getNodeHistory()
        ]);
        
        setApiKeys(keys);
        setNodeHistory(history);
        
        // Try to load costs
        try {
          const costs = await api.getTotalCosts();
          setTotalCosts(costs);
        } catch (error) {
          console.log('Admin key not set, costs unavailable');
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