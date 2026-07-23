import { useEffect, useState } from 'react'
import { api, ApiRequestError } from '../../api/client'
import type { RelationshipRevision } from '../../api/types'

interface TimelineProps {
  factId: string
}

export function Timeline({ factId }: TimelineProps) {
  const [items, setItems] = useState<RelationshipRevision[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!factId) return
    setLoading(true)
    setError(null)
    void api
      .relationshipHistory(factId)
      .then(setItems)
      .catch((err) => setError(err instanceof ApiRequestError ? err.message : '历史加载失败'))
      .finally(() => setLoading(false))
  }, [factId])

  const sorted = items.slice().sort((a, b) => (a.createdAt > b.createdAt ? 1 : -1))

  return (
    <section className="timeline" aria-label="关系时间线">
      <header>
        <h2>关系时间线</h2>
      </header>
      {error && <p className="error-state" role="alert">{error}</p>}
      {loading && <p className="empty-note">加载中...</p>}
      {!loading && sorted.length === 0 && <p className="empty-note">暂无修订记录</p>}
      <ol className="timeline-list">
        {sorted.map((item) => (
          <li key={item.id} className="timeline-item">
            <div className="timeline-header">
              <strong>{item.status}</strong>
              <span>{item.createdAt}</span>
            </div>
            <p>{item.summary}</p>
            <div className="timeline-meta">
              <span>场景 {item.sceneId}</span>
              <span>
                有效：{item.validFromSceneId} {item.validUntilSceneId ? `→ ${item.validUntilSceneId}` : ''}
              </span>
            </div>
          </li>
        ))}
      </ol>
    </section>
  )
}
