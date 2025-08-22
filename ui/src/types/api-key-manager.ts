export interface ApiKey {
  key: string;
  status: 'active' | 'inactive';
  total_cost: number;
  assigned_nodes: string[];
  created_at: number;
}

export interface CostRecord {
  timestamp: number;
  amount: number;
  currency: string;
  description: string;
}

export interface NodeAssignment {
  node_id: string;
  api_key: string;
  issued_at: number;
}

export interface CostData {
  total_cost: number;
  cost_by_key: Record<string, number>;
  currency: string;
}

export interface KeyCostData {
  api_key: string;
  costs: CostRecord[];
  total: number;
}

export interface ChartDataPoint {
  timestamp: number;
  date: string;
  totalCost: number;
  currency: string;
  newNodes?: string[];
}

export interface AddKeyRequest {
  api_key: string;
}

export interface RemoveKeyRequest {
  api_key: string;
}

export interface KeyStatusRequest {
  api_key: string;
}

export interface CostRangeRequest {
  start_date?: string;
  end_date?: string;
}

export interface KeyCostRequest {
  api_key: string;
  start_date?: string;
  end_date?: string;
}

export interface SetAdminKeyRequest {
  admin_key: string;
}