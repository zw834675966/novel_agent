import { useState } from 'react'

export function PromptSettingsPanel() {
  const [temperature, setTemperature] = useState(0.0)
  const [seed, setSeed] = useState(42)
  const [activeTab, setActiveTab] = useState<'system' | 'derivation' | 'narration'>('system')

  const systemPromptPreview = `你是一个小说角色感官推导引擎。
你只输出符合 schema 约束的 JSON 格式。
严禁输出任何 markdown 格式化标记、代码块标记（\`\`\`json）或任何额外文字。`

  const derivationPromptPreview = `== 角色背景 ==
角色：姜宁
性格：坚韧, 投资人
技能：反向收购

== 场景客观事件 ==
慈善拍卖会上与陆沉重逢

== 前情记忆 ==
- [亲眼所见/确定] 十年前家道破产

== 原著候选词集（只准从以下分类中按 id 挑选引用）==
visual:
  - visual.bloodstain: "血迹"
  - visual.candlelight: "烛光"
...`

  return (
    <section className="prompt-settings-panel" aria-label="提示词与确定性控制">
      <header className="panel-header">
        <h2>
          <span>⚙️</span> 提示词工程与确定性解码控制
        </h2>
        <div className="determinism-status">
          <span className="badge-glow" /> 100% 确定性生成模式开启
        </div>
      </header>

      <div className="settings-grid">
        <div className="setting-item">
          <label htmlFor="temperature-range">Temperature (采样温度): {temperature.toFixed(1)}</label>
          <input
            id="temperature-range"
            type="range"
            min="0"
            max="1"
            step="0.1"
            value={temperature}
            onChange={(e) => setTemperature(parseFloat(e.target.value))}
          />
          <span className="hint-text">设为 0.0 启用贪婪解码 (Greedy Decoding)，消除无谓随机抖动</span>
        </div>

        <div className="setting-item">
          <label htmlFor="seed-input">Random Seed: {seed}</label>
          <input
            id="seed-input"
            type="number"
            value={seed}
            onChange={(e) => setSeed(parseInt(e.target.value) || 42)}
          />
          <span className="hint-text">固定 Seed 确保相同 Prompt 下输出 bitwise-identical</span>
        </div>
      </div>

      <div className="prompt-tabs">
        <button
          type="button"
          className={activeTab === 'system' ? 'tab-btn active' : 'tab-btn'}
          onClick={() => setActiveTab('system')}
        >
          System Prompt
        </button>
        <button
          type="button"
          className={activeTab === 'derivation' ? 'tab-btn active' : 'tab-btn'}
          onClick={() => setActiveTab('derivation')}
        >
          Derivation Prompt Template
        </button>
        <button
          type="button"
          className={activeTab === 'narration' ? 'tab-btn active' : 'tab-btn'}
          onClick={() => setActiveTab('narration')}
        >
          Narration Prompt Template
        </button>
      </div>

      <div className="prompt-preview-box">
        <pre>
          <code>
            {activeTab === 'system' && systemPromptPreview}
            {activeTab === 'derivation' && derivationPromptPreview}
            {activeTab === 'narration' && "== 叙事骨架与硬引用组装规则 ==\n正文关键描写必须按 id 严格注入..."}
          </code>
        </pre>
      </div>
    </section>
  )
}
