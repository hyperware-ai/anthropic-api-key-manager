export interface ApiKey {
  key: string;
  status: string;
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
  node_id: string;  // Keep as snake_case to match what backend sends
  api_key: string;
  issued_at: number;
}

export interface CostData {
  total_cost: number;
  cost_by_key: [string, number][];  // Changed to match the TypeScript generated type
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