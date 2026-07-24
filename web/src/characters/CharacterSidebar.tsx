import type { Character } from '../api/types'

interface CharacterSidebarProps {
  characters: Character[]
  query: string
  selectedIds: string[]
  onQueryChange: (query: string) => void
  onSelectionChange: (characterId: string) => void
}

export function CharacterSidebar({
  characters,
  query,
  selectedIds,
  onQueryChange,
  onSelectionChange,
}: CharacterSidebarProps) {
  // Deduplicate or filter visible characters
  const visibleCharacters = characters.filter((character) =>
    character.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
  )

  return (
    <aside className="character-sidebar" aria-label="人物世界观">
      <div className="sidebar-header">
        <h2>
          <span>👤</span> 人物档案与状态
        </h2>
        <span className="count-badge">{selectedIds.length} / {characters.length} 已选</span>
      </div>

      <div className="search-box">
        <input
          id="character-search"
          type="search"
          placeholder="搜索角色名字或标签..."
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
        />
      </div>

      <ul className="character-list">
        {visibleCharacters.map((character, idx) => {
          const isSelected = selectedIds.includes(character.id)
          return (
            <li key={character.id}>
              <label className={`character-option ${isSelected ? 'selected' : ''}`}>
                <input
                  type="checkbox"
                  aria-label={character.name}
                  checked={isSelected}
                  onChange={() => onSelectionChange(character.id)}
                />
                <div className="character-info">
                  <div className="character-name-row">
                    <span className="character-name">
                      {character.name} #{idx + 1}
                    </span>
                    <span className="state-tag alive">Alive</span>
                  </div>
                  {character.personality && character.personality.length > 0 && (
                    <div className="character-tags">
                      {character.personality.map((tag) => (
                        <span key={tag} className="tag-badge">
                          {tag}
                        </span>
                      ))}
                      {character.skills?.map((skill) => (
                        <span key={skill} className="tag-badge skill">
                          {skill}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </label>
            </li>
          )
        })}
      </ul>
      {visibleCharacters.length === 0 && <p className="empty-note">未找到符合条件的人物</p>}
    </aside>
  )
}
