export interface DocumentSummary {
  id: number
  filename: string
  stored_path: string
  folder_type: string
  category: string
  risk_level: string
  processing_status: string
  updated_at: string
  file_ext: string | null
  file_size: number | null
}

export interface DocumentDetail extends DocumentSummary {
  original_path: string | null
  domain: string
  doc_type: string
  content: string | null
  summary: string | null
  tags: string | null
  privacy_score: number
  summary_status: string
  created_at: string
  imported_at: string
}




import { getJson, withContext } from './api-client'

export async function fetchDocuments(limit = 50): Promise<DocumentSummary[]> {
  try {
    const data = await getJson<{ documents: DocumentSummary[] }>(`/api/documents?limit=${limit}`)
    return data.documents
  } catch (error) {
    throw withContext(error, 'Failed to load document list')
  }
}

export async function fetchDocument(id: number): Promise<DocumentDetail> {
  try {
    const data = await getJson<{ document: DocumentDetail }>(`/api/documents/${id}`)
    return data.document
  } catch (error) {
    throw withContext(error, `Failed to load document #${id}`)
  }
}