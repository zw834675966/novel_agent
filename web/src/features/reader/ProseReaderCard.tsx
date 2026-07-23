interface ProseReaderCardProps {
  proseText?: string
  quoteDensity?: number
  unverifiedQuotes?: number
  strippedRefs?: number
  actionOnlyBeats?: number
}

export function ProseReaderCard({
  proseText = '金桂的母亲此时势孤，也只得跟着周瑞家的到他女孩儿屋里，只见满脸黑血，直挺挺的躺在炕上，便叫哭起来。她凝视尸体，沉思片刻。',
  quoteDensity = 0.8436,
  unverifiedQuotes = 0,
  strippedRefs = 0,
  actionOnlyBeats = 1,
}: ProseReaderCardProps) {
  const percentage = (quoteDensity * 100).toFixed(1)
  const isHealthy = quoteDensity >= 0.3

  return (
    <section className="prose-reader-card" aria-label="正文阅读与原著引用比对">
      <header className="card-header">
        <h2>
          <span>📖</span> 小说正文预览与防 AI 腔遥测
        </h2>
        <span className={`density-badge ${isHealthy ? 'healthy' : 'warning'}`}>
          原著引用密度: {percentage}% (指标 $\ge$ 30%)
        </span>
      </header>

      <div className="telemetry-bar">
        <div className="telemetry-item">
          <span className="telemetry-label">原著硬引用</span>
          <span className="telemetry-val highlight">{percentage}%</span>
        </div>
        <div className="telemetry-item">
          <span className="telemetry-label">未校验引用 (防假冒)</span>
          <span className={`telemetry-val ${unverifiedQuotes === 0 ? 'zero' : 'bad'}`}>
            {unverifiedQuotes}
          </span>
        </div>
        <div className="telemetry-item">
          <span className="telemetry-label">剥除 AI 套话</span>
          <span className="telemetry-val">{strippedRefs} 处</span>
        </div>
        <div className="telemetry-item">
          <span className="telemetry-label">纯动作 Beats</span>
          <span className="telemetry-val">{actionOnlyBeats} 拍</span>
        </div>
      </div>

      <div className="prose-content-box">
        <p className="prose-paragraph">
          {proseText.split('。').map((sentence, idx) => {
            if (!sentence.trim()) return null
            return (
              <span key={idx} className="sentence-block">
                <span className="quote-highlight">{sentence}</span>。
              </span>
            )
          })}
        </p>
      </div>

      <footer className="reader-footer">
        <span className="rhythm-tag">节奏连接: 顿号/句号控制</span>
        <span className="source-tag">书源隔离: 强主导声口 (hlm)</span>
        <span className="slot-tag">N4槽位: Observed 0 因果说明文</span>
      </footer>
    </section>
  )
}
