import { create } from 'zustand';
import { CostData, KeyCostData } from '../types/api-key-manager';
import { ApiKeyInfo as ApiKey, CostRecord, NodeAssignment } from '../../../target/ui/caller-utils';

interface ApiKeyManagerStore {
  // State
  apiKeys: ApiKey[];
  costData: CostRecord[];
  nodeHistory: NodeAssignment[];
  selectedKey: string | null;
  adminKeySet: boolean;
  authToken: string | null;
  loading: boolean;
  error: string | null;
  totalCosts: CostData | null;
  keyCosts: KeyCostData | null;
  
  // Actions
  setApiKeys: (keys: ApiKey[]) => void;
  setCostData: (data: CostRecord[]) => void;
  setNodeHistory: (history: NodeAssignment[]) => void;
  setSelectedKey: (key: string | null) => void;
  setAdminKeySet: (set: boolean) => void;
  setAuthToken: (token: string | null) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  setTotalCosts: (costs: CostData | null) => void;
  setKeyCosts: (costs: KeyCostData | null) => void;
  reset: () => void;
}

const initialState = {
  apiKeys: [],
  costData: [],
  nodeHistory: [],
  selectedKey: null,
  adminKeySet: false,
  authToken: null,
  loading: false,
  error: null,
  totalCosts: null,
  keyCosts: null,
};

export const useApiKeyManagerStore = create<ApiKeyManagerStore>((set) => ({
  ...initialState,
  
  setApiKeys: (keys) => set({ apiKeys: keys }),
  setCostData: (data) => set({ costData: data }),
  setNodeHistory: (history) => set({ nodeHistory: history }),
  setSelectedKey: (key) => set({ selectedKey: key }),
  setAdminKeySet: (isSet) => set({ adminKeySet: isSet }),
  setAuthToken: (token) => set({ authToken: token }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
  setTotalCosts: (costs) => set({ totalCosts: costs }),
  setKeyCosts: (costs) => set({ keyCosts: costs }),
  reset: () => set(initialState),
}));