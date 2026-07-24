import type { BatchDerivationResult, Character } from '../api/types'

type DeriveState = 'idle' | 'running' | 'complete' | 'failed'

interface DerivationInspectorProps {
  characters: Character[]
  state: DeriveState
  results: BatchDerivationResult[]
  error: string | null
  onDerive: () => void
}

const senses = [
  ['视觉', 'visual'],
  ['听觉', 'auditory'],
  ['嗅觉', 'olfactory'],
  ['触触', 'tactile'],
  ['味觉', 'gustatory'],
  ['情绪', 'emotion'],
  ['动作', 'gesture'],
  ['氛围', 'atmosphere'],
] as const

export function DerivationInspector({
  characters,
  state,
  results,
  error,
  onDerive,
}: DerivationInspectorProps) {
  const participantNames = new Map(characters.map((character) => [character.id, character.name]))

  return (
    <aside className="derivation-inspector" aria-label="推导结果">
      <h2>推导结果</h2>
      <button type="button" onClick={onDerive} disabled={state === 'running'}>
        {state === 'running' ? '推导中' : '开始推导'}
      </button>
      {state === 'idle' && <p className="empty-note">暂无推导结果</p>}
      {state === 'failed' && (
        <div className="error-state" role="alert">
          <p>{error}</p>
          <button type="button" onClick={onDerive}>
            重试
          </button>
        </div>
      )}
      {state === 'complete' && (
        <ul className="derivation-list">
          {results.map((result) => {
            const derivation = result.derivation

            return (
              <li key={result.characterId} className="derivation-result">
                <h3>{participantNames.get(result.characterId) ?? '参与人物'}</h3>
                {result.error ? (
                  <p className="result-error">{result.error.message}</p>
                ) : (
                  <>
                    <p>{derivation?.newMemory.content}</p>
                    <dl>
                      {derivation &&
                        senses.map(([label, key]) => (
                          <div key={key}>
                            <dt>{label}</dt>
                            <dd>
                              {(derivation.sensations as any)[key]?.join('、') || '无'}
                            </dd>
                          </div>
                        ))}
                    </dl>
                  </>
                )}
              </li>
            )
          })}
        </ul>
      )}
    </aside>
  )
}
