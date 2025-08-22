import * as callerUtils from '../../../target/ui/caller-utils';
import { 
  ApiKey, 
  NodeAssignment, 
  CostData, 
  KeyCostData,
  AddKeyRequest,
  RemoveKeyRequest,
  KeyStatusRequest,
  CostRangeRequest,
  KeyCostRequest,
  SetAdminKeyRequest
} from '../types/api-key-manager';

export async function addApiKey(apiKey: string): Promise<void> {
  const request: AddKeyRequest = { api_key: apiKey };
  const response = await callerUtils.addApiKey(JSON.stringify(request));
  const result = JSON.parse(response);
  if (!result.success) {
    throw new Error(result.message || 'Failed to add API key');
  }
}

export async function removeApiKey(apiKey: string): Promise<void> {
  const request: RemoveKeyRequest = { api_key: apiKey };
  const response = await callerUtils.removeApiKey(JSON.stringify(request));
  const result = JSON.parse(response);
  if (!result.success) {
    throw new Error(result.message || 'Failed to remove API key');
  }
}

export async function listKeys(): Promise<ApiKey[]> {
  const response = await callerUtils.listKeys('');
  return JSON.parse(response);
}

export async function getKeyStatus(apiKey: string): Promise<{
  status: string;
  assigned_nodes: string[];
  total_cost: number;
}> {
  const request: KeyStatusRequest = { api_key: apiKey };
  const response = await callerUtils.getKeyStatus(JSON.stringify(request));
  return JSON.parse(response);
}

export async function getTotalCosts(startDate?: string, endDate?: string): Promise<CostData> {
  const request: CostRangeRequest = {
    start_date: startDate,
    end_date: endDate
  };
  const response = await callerUtils.getTotalCosts(JSON.stringify(request));
  return JSON.parse(response);
}

export async function getKeyCosts(apiKey: string, startDate?: string, endDate?: string): Promise<KeyCostData> {
  const request: KeyCostRequest = {
    api_key: apiKey,
    start_date: startDate,
    end_date: endDate
  };
  const response = await callerUtils.getKeyCosts(JSON.stringify(request));
  return JSON.parse(response);
}

export async function getNodeHistory(): Promise<NodeAssignment[]> {
  const response = await callerUtils.getNodeHistory('');
  return JSON.parse(response);
}

export async function setAdminKey(adminKey: string): Promise<void> {
  const request: SetAdminKeyRequest = { admin_key: adminKey };
  const response = await callerUtils.setAdminKey(JSON.stringify(request));
  const result = JSON.parse(response);
  if (!result.success) {
    throw new Error(result.message || 'Failed to set admin key');
  }
}

export async function initializeAuth(): Promise<string> {
  const response = await callerUtils.initializeAuth('');
  const result = JSON.parse(response);
  return result.token;
}

export async function refreshCosts(): Promise<void> {
  const response = await callerUtils.refreshCosts('');
  const result = JSON.parse(response);
  if (!result.success) {
    throw new Error(result.message || 'Failed to refresh costs');
  }
}