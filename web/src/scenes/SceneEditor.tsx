interface SceneEditorProps {
  objectiveEvent: string
  occurredAt: string
  participantCount: number
  submitting: boolean
  feedback: string | null
  onObjectiveEventChange: (value: string) => void
  onOccurredAtChange: (value: string) => void
  onSubmit: () => void
}

export function SceneEditor({
  objectiveEvent,
  occurredAt,
  participantCount,
  submitting,
  feedback,
  onObjectiveEventChange,
  onOccurredAtChange,
  onSubmit,
}: SceneEditorProps) {
  return (
    <main className="scene-editor">
      <header className="scene-editor-header">
        <p className="eyebrow">场景工作台</p>
        <h1>新建场景与剧情推导</h1>
      </header>

      <form
        className="scene-form"
        onSubmit={(event) => {
          event.preventDefault()
          onSubmit()
        }}
      >
        <div className="form-group">
          <label htmlFor="objective-event">客观事件 (场景基础描述)</label>
          <textarea
            id="objective-event"
            rows={5}
            value={objectiveEvent}
            onChange={(event) => onObjectiveEventChange(event.target.value)}
            placeholder="请输入场景中发生的客观可观察事件（如：慈善拍卖会上姜宁与陆沉重逢；或古宅中发现发现一具尸体...）"
            required
          />
        </div>

        <div className="form-group">
          <label htmlFor="occurred-at">发生时间</label>
          <input
            id="occurred-at"
            type="datetime-local"
            value={occurredAt}
            onChange={(event) => onOccurredAtChange(event.target.value)}
            required
          />
        </div>

        <div className="form-footer">
          <p className="participant-count">
            <span>👥</span> 参与人物：已选择 <strong>{participantCount}</strong> 人
          </p>
          <button
            type="submit"
            className="btn-primary"
            disabled={submitting || participantCount === 0}
          >
            {submitting ? '场景创建中...' : '创建场景并开始推导'}
          </button>
        </div>
      </form>

      {feedback && (
        <p className="form-feedback" role="status">
          {feedback}
        </p>
      )}
    </main>
  )
}
