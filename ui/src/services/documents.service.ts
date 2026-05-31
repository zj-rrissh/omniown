export interface DocumentSummary {
  id: number
  filename: string
  storedPath: string
  folderType: string
  category: string
  riskLevel: string
  processingStatus: string
  updatedAt: string
  fileExt: string | null
  fileSize: number | null
}

export interface DocumentDetail extends DocumentSummary {
  originalPath: string | null
  domain: string
  docType: string
  content: string | null
  summary: string | null
  tags: string | null
  privacyScore: number
  summaryStatus: string
  createdAt: string
  importedAt: string
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