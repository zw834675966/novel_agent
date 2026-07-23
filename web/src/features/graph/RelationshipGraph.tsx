import { useEffect, useRef } from 'react'
import cytoscape, { Core } from 'cytoscape'
import { api } from '../../api/client'

interface RelationshipGraphProps {
  sceneId: string
}

export function RelationshipGraph({ sceneId }: RelationshipGraphProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const cyRef = useRef<Core | null>(null)

  useEffect(() => {
    if (!containerRef.current) return

    let cancelled = false
    api
      .storyGraph(sceneId)
      .then((snapshot) => {
        if (cancelled || !containerRef.current) return

        if (cyRef.current) {
          cyRef.current.destroy()
        }

        const elements = [
          ...snapshot.nodes.map((node) => ({
            data: { id: node.id, label: node.name },
          })),
          ...snapshot.edges.map((edge, index) => ({
            data: {
              id: `edge-${index}`,
              source: edge.from,
              target: edge.to,
              label: edge.relationshipType,
              summary: edge.summary,
              active: edge.active,
            },
          })),
        ]

        cyRef.current = cytoscape({
          container: containerRef.current,
          elements,
          layout: { name: 'cose' },
          style: [
            {
              selector: 'node',
              style: {
                label: 'data(label)',
                'background-color': '#0f6e64',
                color: '#fff',
              },
            },
            {
              selector: 'edge',
              style: {
                label: 'data(label)',
                width: 2,
                'line-color': '#9ca3af',
                'target-arrow-color': '#9ca3af',
                'target-arrow-shape': 'triangle',
                'curve-style': 'bezier',
              },
            },
            {
              selector: 'edge[active = false]',
              style: {
                'line-color': '#e5e7e9',
                'target-arrow-color': '#e5e7e9',
                'text-opacity': 0.6,
              },
            },
          ],
        })
      })
      .catch(() => {
        // graph load failure is non-blocking
      })

    return () => {
      cancelled = true
      if (cyRef.current) {
        cyRef.current.destroy()
        cyRef.current = null
      }
    }
  }, [sceneId])

  return (
    <section className="relationship-graph" aria-label="关系图谱">
      <header>
        <h2>关系图谱</h2>
      </header>
      <div ref={containerRef} className="graph-container" />
    </section>
  )
}
