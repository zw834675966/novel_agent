import { useEffect, useState } from 'react'

import { api, ApiRequestError } from './api/client'
import type { BatchDerivationResult, Character, Scene } from './api/types'
import { CharacterSidebar } from './characters/CharacterSidebar'
import { DerivationInspector } from './derivations/DerivationInspector'
import { SceneEditor } from './scenes/SceneEditor'
import { CandidateReview } from './features/relationships/CandidateReview'
import { Timeline } from './features/timeline/Timeline'
import { RelationshipGraph } from './features/graph/RelationshipGraph'
import { PromptSettingsPanel } from './features/prompt/PromptSettingsPanel'
import { ProseReaderCard } from './features/reader/ProseReaderCard'

type DeriveState = 'idle' | 'running' | 'complete' | 'failed'
type TabType = 'editor' | 'prose' | 'prompt' | 'graph'

export function App() {
  const [activeTab, setActiveTab] = useState<TabType>('editor')
  const [characters, setCharacters] = useState<Character[]>([])
  const [selectedIds, setSelectedIds] = useState<string[]>([])
  const [query, setQuery] = useState('')
  const [objectiveEvent, setObjectiveEvent] = useState('')
  const [occurredAt, setOccurredAt] = useState('')
  const [scene, setScene] = useState<Scene | null>(null)
  const [creating, setCreating] = useState(false)
  const [feedback, setFeedback] = useState<string | null>(null)
  const [deriveState, setDeriveState] = useState<DeriveState>('idle')
  const [results, setResults] = useState<BatchDerivationResult[]>([])
  const [deriveError, setDeriveError] = useState<string | null>(null)
  const [selectedFactId, setSelectedFactId] = useState<string | null>(null)

  useEffect(() => {
    void api.listCharacters().then(setCharacters).catch(() => setFeedback('人物列表加载失败'))
  }, [])

  const selectedCharacters = characters.filter((character) => selectedIds.includes(character.id))

  function toggleCharacter(characterId: string) {
    setSelectedIds((current) =>
      current.includes(characterId)
        ? current.filter((id) => id !== characterId)
        : [...current, characterId],
    )
  }

  async function createScene() {
    setCreating(true)
    setFeedback(null)
    try {
      const createdScene = await api.createScene({
        objectiveEvent,
        participantIds: selectedIds,
        occurredAt: new Date(occurredAt || Date.now()).toISOString(),
      })
      setScene(createdScene)
      setResults([])
      setDeriveState('idle')
      setFeedback('场景创建成功！现在可点击【开始推导】生成小说正文')
    } catch (error) {
      setFeedback(error instanceof ApiRequestError ? error.message : '场景创建失败')
    } finally {
      setCreating(false)
    }
  }

  async function deriveScene() {
    if (!scene) return

    setDeriveState('running')
    setDeriveError(null)
    try {
      setResults(await api.deriveScene(scene.id))
      setDeriveState('complete')
    } catch (error) {
      setDeriveError(error instanceof ApiRequestError ? error.message : '推导请求失败')
      setDeriveState('failed')
    }
  }

  return (
    <div className="app-root">
      <header className="app-header">
        <div className="app-brand">
          <div className="app-brand-logo">N</div>
          <div className="app-title">
            <h1>Novels Engine 叙事工作台</h1>
            <p>AI 驱动小说角色感知与原著描写拼装引擎</p>
          </div>
        </div>

        <nav className="nav-tabs" aria-label="工作台切换">
          <button
            type="button"
            className={`nav-tab ${activeTab === 'editor' ? 'active' : ''}`}
            onClick={() => setActiveTab('editor')}
          >
            <span>📝</span> 场景写作
          </button>
          <button
            type="button"
            className={`nav-tab ${activeTab === 'prose' ? 'active' : ''}`}
            onClick={() => setActiveTab('prose')}
          >
            <span>📖</span> 正文与防AI遥测
          </button>
          <button
            type="button"
            className={`nav-tab ${activeTab === 'prompt' ? 'active' : ''}`}
            onClick={() => setActiveTab('prompt')}
          >
            <span>⚙️</span> 提示词确定性
          </button>
          <button
            type="button"
            className={`nav-tab ${activeTab === 'graph' ? 'active' : ''}`}
            onClick={() => setActiveTab('graph')}
          >
            <span>🕸️</span> 关系图谱
          </button>
        </nav>

        <div className="model-badge">
          <span className="badge-dot" />
          DeepSeek V4 Flash · 12,477 语料库
        </div>
      </header>

      <div className="workbench-shell">
        <CharacterSidebar
          characters={characters}
          query={query}
          selectedIds={selectedIds}
          onQueryChange={setQuery}
          onSelectionChange={toggleCharacter}
        />

        <div className="center-pane">
          {activeTab === 'editor' && (
            <>
              <SceneEditor
                objectiveEvent={objectiveEvent}
                occurredAt={occurredAt}
                participantCount={selectedCharacters.length}
                submitting={creating}
                feedback={feedback}
                onObjectiveEventChange={setObjectiveEvent}
                onOccurredAtChange={setOccurredAt}
                onSubmit={createScene}
              />

              {scene && (
                <DerivationInspector
                  characters={characters}
                  state={deriveState}
                  results={results}
                  error={deriveError}
                  onDerive={deriveScene}
                />
              )}
            </>
          )}

          {activeTab === 'prose' && <ProseReaderCard />}

          {activeTab === 'prompt' && <PromptSettingsPanel />}

          {activeTab === 'graph' && (
            <>
              {scene ? (
                <RelationshipGraph sceneId={scene.id} />
              ) : (
                <div className="empty-card">
                  <h3>🕸️ 关系图谱视图</h3>
                  <p>请先在左侧选择参与人物并在中央提交【新建场景】，即可在此查看 Cytoscape 动态关系图谱。</p>
                </div>
              )}
            </>
          )}
        </div>

        <div className="right-pane">
          {scene ? (
            <>
              <CandidateReview sceneId={scene.id} onSelectFact={setSelectedFactId} />
              {selectedFactId && <Timeline factId={selectedFactId} />}
            </>
          ) : (
            <div className="guide-card">
              <h3>💡 快速上手指南</h3>
              <ol>
                <li>从左侧勾选参与场景的<strong>角色</strong>（如侦探）。</li>
                <li>在中间填写场景<strong>客观事件</strong>（如发现尸体）及时间。</li>
                <li>点击【创建场景】后发起<strong>感官推导</strong>。</li>
                <li>切换到【正文与防AI遥测】标签查看高品质硬引用正文。</li>
              </ol>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
