/**
 * HTTP Export integration service
 */

import { invokeAuth } from '../client'
import type {
  HttpExportConfig,
  SaveHttpExportConfigRequest,
  HttpExportRequest,
  HttpExportResponse,
  ValidateTemplateResponse,
  TestConnectionResponse,
  ExportHistoryRecord,
  MessageResponse,
} from '@/types'

/**
 * List all HTTP export configurations
 */
export async function listConfigs(): Promise<HttpExportConfig[]> {
  return invokeAuth<HttpExportConfig[]>('list_http_export_configs')
}

/**
 * Save (create or update) an HTTP export configuration
 */
export async function saveConfig(request: SaveHttpExportConfigRequest): Promise<MessageResponse> {
  return invokeAuth<MessageResponse>('save_http_export_config', { request })
}

/**
 * Delete an HTTP export configuration
 */
export async function deleteConfig(configId: string): Promise<MessageResponse> {
  return invokeAuth<MessageResponse>('delete_http_export_config', { configId })
}

/**
 * Execute HTTP export for work items
 */
export async function executeExport(request: HttpExportRequest): Promise<HttpExportResponse> {
  return invokeAuth<HttpExportResponse>('execute_http_export', { request })
}

/**
 * Test HTTP export connection
 */
export async function testConnection(configId: string): Promise<TestConnectionResponse> {
  return invokeAuth<TestConnectionResponse>('test_http_export_connection', { configId })
}

/**
 * Validate a payload template
 */
export async function validateTemplate(template: string): Promise<ValidateTemplateResponse> {
  return invokeAuth<ValidateTemplateResponse>('validate_http_export_template', { template })
}

/**
 * Get export history — which items have been successfully exported to a config
 */
export async function getExportHistory(configId: string, workItemIds: string[]): Promise<ExportHistoryRecord[]> {
  return invokeAuth<ExportHistoryRecord[]>('get_http_export_history', { configId, workItemIds })
}
