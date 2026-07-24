import { useEffect, useState } from 'react'
import { api, ApiRequestError } from '../../api/client'
import type { RelationshipCandidate } from '../../api/types'

interface CandidateReviewProps {
  sceneId: string
  onSelectFact?: (factId: string) => void
}

export function CandidateReview({ sceneId, onSelectFact }: CandidateReviewProps) {
  const [candidates, setCandidates] = useState<RelationshipCandidate[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [actioningId, setActioningId] = useState<string | null>(null)

  async function loadCandidates() {
    setLoading(true)
    setError(null)
    try {
      const details = await api.sceneDerivations(sceneId)
      const all = details.flatMap((d) => d.relationshipCandidates ?? [])
      setCandidates(all.filter((c) => c.status === 'Pending'))
    } catch (err) {
      setError(err instanceof ApiRequestError ? err.message : '候选加载失败')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void loadCandidates()
  }, [sceneId])

  async function accept(candidate: RelationshipCandidate) {
    setActioningId(candidate.id)
    try {
      await api.resolveCandidate(candidate.id, {
        action: 'accept',
        summary: candidate.summary,
      })
      await loadCandidates()
    } catch (err) {
      setError(err instanceof ApiRequestError ? err.message : '接受失败')
    } finally {
      setActioningId(null)
    }
  }

  async function reject(candidate: RelationshipCandidate) {
    setActioningId(candidate.id)
    try {
      await api.resolveCandidate(candidate.id, { action: 'reject' })
      await loadCandidates()
    } catch (err) {
      setError(err instanceof ApiRequestError ? err.message : '拒绝失败')
    } finally {
      setActioningId(null)
    }
  }

  return (
    <section className="candidate-review" aria-label="关系候选审核">
      <header>
        <h2>待确认关系</h2>
        <button type="button" onClick={loadCandidates} disabled={loading}>
          {loading ? '刷新中' : '刷新'}
        </button>
      </header>
      {error && <p className="error-state" role="alert">{error}</p>}
      {candidates.length === 0 && <p className="empty-note">暂无待确认关系</p>}
      <ul className="candidate-list">
        {candidates.map((candidate) => (
          <li key={candidate.id} className="candidate-card">
            <p>
              {candidate.fromCharacterId} → {candidate.toCharacterId}：{candidate.relationshipType}
            </p>
            <p>{candidate.summary}</p>
            <div className="candidate-meta">
              <span>置信度 {Math.round(candidate.confidence * 100)}%</span>
              <span>{candidate.createdAt}</span>
            </div>
            <div className="candidate-actions">
              <button type="button" onClick={() => accept(candidate)} disabled={actioningId === candidate.id}>
                {actioningId === candidate.id ? '处理中' : '接受'}
              </button>
              <button type="button" onClick={() => reject(candidate)} disabled={actioningId === candidate.id}>
                拒绝
              </button>
              {onSelectFact && (
                <button type="button" onClick={() => onSelectFact(candidate.id)}>
                  历史
                </button>
              )}
            </div>
          </li>
        ))}
      </ul>
    </section>
  )
}
