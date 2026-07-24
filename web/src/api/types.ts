export interface ApiFieldError {
  field: string
  message: string
}

export interface ApiErrorBody {
  code: string
  message: string
  fields?: ApiFieldError[]
}

export interface Character {
  id: string
  name: string
  personality: string[]
  skills: string[]
}

export interface Scene {
  id: string
  objectiveEvent: string
  participantIds: string[]
  occurredAt: string
}

export interface SensorySelection {
  visual: string[]
  auditory: string[]
  olfactory: string[]
  tactile: string[]
  gustatory: string[]
  emotion: string[]
  gesture: string[]
  atmosphere: string[]
}

export interface Memory {
  characterId: string
  content: string
  source: string
  certainty: string
  createdAt: string
}

export interface PlotDevelopment {
  kind: string
  reason: string
}

export interface RelationshipCandidate {
  id: string
  fromCharacterId: string
  toCharacterId: string
  relationshipType: string
  summary: string
  tensionScore?: number
  trustScore?: number
  affectionScore?: number
  powerScore?: number
  confidence: number
  status: string
  createdAt: string
}

export interface RelationshipRevision {
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

export interface CharacterDerivation {
  characterId: string
  sensations: SensorySelection
  newMemory: Memory
  plotDevelopment: PlotDevelopment[]
  relationshipCandidates: RelationshipCandidate[]
}

export interface BatchDerivationResult {
  characterId: string
  derivation: CharacterDerivation | null
  error: ApiErrorBody | null
}

export interface CreateSceneInput {
  objectiveEvent: string
  participantIds: string[]
  occurredAt: string
}
