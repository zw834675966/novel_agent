import type {
  ApiErrorBody,
  BatchDerivationResult,
  Character,
  CharacterDerivation,
  CreateSceneInput,
  Scene,
} from './types'

export class ApiRequestError extends Error {
  readonly code: string
  readonly fields?: ApiErrorBody['fields']

  constructor(error: ApiErrorBody) {
    super(error.message)
    this.name = 'ApiRequestError'
    this.code = error.code
    this.fields = error.fields
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: {
      Accept: 'application/json',
      ...(init?.body ? { 'Content-Type': 'application/json' } : {}),
      ...init?.headers,
    },
  })

  const payload = (await response.json()) as T | ApiErrorBody

  if (!response.ok) {
    throw new ApiRequestError(payload as ApiErrorBody)
  }

  return payload as T
}

export const api = {
  listCharacters: () => request<Character[]>('/api/characters'),
  createScene: (input: CreateSceneInput) =>
    request<Scene>('/api/scenes', {
      method: 'POST',
      body: JSON.stringify(input),
    }),
  deriveScene: (sceneId: string) =>
    request<BatchDerivationResult[]>(`/api/scenes/${sceneId}/derive`, { method: 'POST' }),
  getScene: (sceneId: string) => request<Scene>(`/api/scenes/${sceneId}`),
  sceneDerivations: (sceneId: string) =>
    request<CharacterDerivation[]>(`/api/scenes/${sceneId}/derivations`),
  resolveCandidate: (candidateId: string, decision: { action: string; summary?: string }) =>
    request<void>(`/api/relationship-candidates/${candidateId}`, {
      method: 'PATCH',
      body: JSON.stringify({ decision }),
    }),
  relationshipHistory: (factId: string) =>
    request<RelationshipRevision[]>(`/api/relationships/${factId}/history`),
  storyGraph: (sceneId: string) =>
    request<GraphSnapshot>(`/api/story-graph?through=${sceneId}`),
}

interface GraphSnapshot {
  nodes: GraphNode[]
  edges: GraphEdge[]
}

interface GraphNode {
  id: string
  name: string
}

interface GraphEdge {
  factId: string
  from: string
  to: string
  relationshipType: string
  summary: string
  scores: { tension?: number; trust?: number; affection?: number; power?: number }
  active: boolean
}

interface RelationshipRevision {
  id: string
  relationshipFactId: string
  sceneId: string
  validFromSceneId: string
  validUntilSceneId?: string
  status: string
  summary: string
  scores: { tension?: number; trust?: number; affection?: number; power?: number }
  evidenceMemoryId?: string
  createdAt: string
}
