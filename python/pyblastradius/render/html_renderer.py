"""HTML/D3.js interactive renderer for dependency trees."""

import json
from pyblastradius.trees import TreeNode


def render_html(tree: TreeNode, title: str = "Dependency Tree", theme: str = "light") -> str:
    """
    Render tree as interactive HTML with D3.js.

    Args:
        tree: TreeNode to render
        title: Page title
        theme: "light" or "dark"

    Returns:
        Complete HTML document as string
    """
    tree_data = tree.to_dict()
    tree_json = json.dumps(tree_data, indent=2)

    # Color mapping by node type
    node_colors = {
        "Service": "#06b6d4",        # cyan
        "Database": "#d946ef",       # magenta
        "Module": "#3b82f6",         # blue
        "Pipeline": "#eab308",       # yellow
        "Topic": "#eab308",
        "Queue": "#eab308",
        "Cache": "#eab308",
        "VectorDb": "#d946ef",
        "Dashboard": "#22c55e",      # green
        "Agent": "#06b6d4",
        "ExternalService": "#ef4444", # red
        "Table": "#d946ef",
        "Test": "#22c55e",
    }

    background = "#0f172a" if theme == "dark" else "#ffffff"
    text_color = "#e2e8f0" if theme == "dark" else "#1e293b"
    grid_color = "#334155" if theme == "dark" else "#e2e8f0"

    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <script src="https://d3js.org/d3.v7.min.js"></script>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background-color: {background};
            color: {text_color};
            padding: 20px;
        }}

        .container {{
            max-width: 100%;
            margin: 0 auto;
        }}

        h1 {{
            margin-bottom: 10px;
            font-size: 28px;
            font-weight: 600;
        }}

        .info {{
            margin-bottom: 20px;
            padding: 12px;
            background-color: rgba(59, 130, 246, 0.1);
            border-left: 4px solid #3b82f6;
            border-radius: 4px;
            font-size: 14px;
        }}

        .controls {{
            margin-bottom: 20px;
            display: flex;
            gap: 10px;
            flex-wrap: wrap;
        }}

        .controls button {{
            padding: 8px 16px;
            background-color: #3b82f6;
            color: white;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 14px;
            font-weight: 500;
            transition: background-color 0.2s;
        }}

        .controls button:hover {{
            background-color: #2563eb;
        }}

        #graph {{
            width: 100%;
            height: 600px;
            border: 1px solid {grid_color};
            border-radius: 8px;
            background-color: {background};
            position: relative;
            overflow: hidden;
        }}

        .node {{
            cursor: pointer;
            stroke: {text_color};
            stroke-width: 2px;
        }}

        .node:hover {{
            stroke-width: 3px;
            filter: brightness(1.2);
        }}

        .link {{
            stroke: {grid_color};
            stroke-opacity: 0.6;
            pointer-events: none;
        }}

        .link.cycle {{
            stroke-dasharray: 5,5;
            stroke-opacity: 0.4;
        }}

        .label {{
            font-size: 12px;
            pointer-events: none;
            dominant-baseline: middle;
            text-anchor: middle;
            fill: {text_color};
            font-weight: 500;
        }}

        .tooltip {{
            position: absolute;
            background-color: rgba(0, 0, 0, 0.8);
            color: white;
            padding: 8px 12px;
            border-radius: 4px;
            font-size: 12px;
            pointer-events: none;
            max-width: 300px;
            word-wrap: break-word;
            z-index: 1000;
            display: none;
        }}

        .legend {{
            margin-top: 20px;
            padding: 16px;
            background-color: rgba(59, 130, 246, 0.05);
            border: 1px solid {grid_color};
            border-radius: 8px;
        }}

        .legend-title {{
            font-weight: 600;
            margin-bottom: 12px;
            font-size: 14px;
        }}

        .legend-items {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 12px;
        }}

        .legend-item {{
            display: flex;
            align-items: center;
            gap: 8px;
            font-size: 13px;
        }}

        .legend-color {{
            width: 16px;
            height: 16px;
            border-radius: 50%;
            border: 2px solid {text_color};
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🌳 {title}</h1>

        <div class="info">
            <strong>Root:</strong> <span id="rootName"></span> |
            <strong>Nodes:</strong> <span id="nodeCount"></span> |
            <strong>Depth:</strong> <span id="maxDepth"></span> |
            <strong>Cycles:</strong> <span id="cycleCount"></span>
        </div>

        <div class="controls">
            <button onclick="resetZoom()">🔄 Reset View</button>
            <button onclick="toggleLabels()">🏷️ Toggle Labels</button>
            <button onclick="expandAll()">📖 Expand All</button>
            <button onclick="collapseAll()">📁 Collapse All</button>
        </div>

        <div id="graph"></div>

        <div id="tooltip" class="tooltip"></div>

        <div class="legend">
            <div class="legend-title">Node Types</div>
            <div class="legend-items">
                <div class="legend-item"><div class="legend-color" style="background-color: #06b6d4;"></div>Service</div>
                <div class="legend-item"><div class="legend-color" style="background-color: #d946ef;"></div>Database</div>
                <div class="legend-item"><div class="legend-color" style="background-color: #3b82f6;"></div>Module</div>
                <div class="legend-item"><div class="legend-color" style="background-color: #eab308;"></div>Cache/Queue</div>
                <div class="legend-item"><div class="legend-color" style="background-color: #22c55e;"></div>Test</div>
                <div class="legend-item"><div class="legend-color" style="background-color: #ef4444;"></div>External</div>
            </div>
        </div>
    </div>

    <script id="tree-data" type="application/json">
{tree_json}
    </script>

    <script>
        // Parse tree data
        const treeData = JSON.parse(document.getElementById('tree-data').textContent);

        // Configuration
        const width = document.getElementById('graph').offsetWidth;
        const height = 600;
        const nodeRadius = 20;
        let showLabels = true;

        // Color mapping
        const nodeColors = {json.dumps(node_colors)};

        // Initialize SVG
        const svg = d3.select('#graph')
            .append('svg')
            .attr('width', '100%')
            .attr('height', '100%')
            .attr('viewBox', [0, 0, width, height]);

        const container = svg.append('g');

        // Add zoom behavior
        const zoom = d3.zoom()
            .on('zoom', (event) => {{
                container.attr('transform', event.transform);
            }});

        svg.call(zoom);

        // Create hierarchy
        const root = d3.hierarchy(treeData);
        const treeLayout = d3.tree().size([width - 100, height - 100]);
        treeLayout(root);

        // Flatten nodes for easier access
        const nodes = root.descendants();
        const links = root.links();

        // Draw links
        const linkGroup = container.append('g');
        linkGroup.selectAll('line')
            .data(links)
            .enter()
            .append('line')
            .attr('class', d => d.target.data.is_cycle ? 'link cycle' : 'link')
            .attr('x1', d => d.source.x)
            .attr('y1', d => d.source.y)
            .attr('x2', d => d.target.x)
            .attr('y2', d => d.target.y);

        // Draw nodes
        const nodeGroup = container.append('g');
        const nodeElements = nodeGroup.selectAll('circle')
            .data(nodes)
            .enter()
            .append('circle')
            .attr('class', 'node')
            .attr('cx', d => d.x)
            .attr('cy', d => d.y)
            .attr('r', nodeRadius)
            .attr('fill', d => nodeColors[d.data.node_type] || '#999')
            .on('mouseover', (event, d) => {{
                showTooltip(event, d.data);
            }})
            .on('mouseout', () => {{
                hideTooltip();
            }});

        // Draw labels
        const labelGroup = container.append('g');
        const labelElements = labelGroup.selectAll('text')
            .data(nodes)
            .enter()
            .append('text')
            .attr('class', 'label')
            .attr('x', d => d.x)
            .attr('y', d => d.y)
            .text(d => d.data.label.substring(0, 15));

        // Update info
        document.getElementById('rootName').textContent = treeData.label;
        document.getElementById('nodeCount').textContent = nodes.length;
        document.getElementById('maxDepth').textContent = d3.max(nodes, d => d.depth);
        document.getElementById('cycleCount').textContent = nodes.filter(d => d.data.is_cycle).length;

        // Tooltip
        function showTooltip(event, data) {{
            const tooltip = document.getElementById('tooltip');
            tooltip.innerHTML = `
                <strong>${{data.label}}</strong><br/>
                Type: ${{data.node_type}}<br/>
                Depth: ${{data.depth}}
                ${{data.is_cycle ? '<br/>🔄 Cycle detected' : ''}}
                ${{Object.keys(data.metadata || {{}}).length > 0 ?
                    '<br/>' + Object.entries(data.metadata).map(([k,v]) => `${{k}}: ${{typeof v === 'object' ? JSON.stringify(v) : v}}`).join('<br/>')
                    : ''}}
            `;
            tooltip.style.display = 'block';
            tooltip.style.left = (event.pageX + 10) + 'px';
            tooltip.style.top = (event.pageY - 10) + 'px';
        }}

        function hideTooltip() {{
            document.getElementById('tooltip').style.display = 'none';
        }}

        // Control functions
        function resetZoom() {{
            svg.transition().duration(750).call(
                zoom.transform,
                d3.zoomIdentity.translate(50, 50)
            );
        }}

        function toggleLabels() {{
            showLabels = !showLabels;
            labelElements.style('display', showLabels ? 'block' : 'none');
        }}

        function expandAll() {{
            // This would require more complex logic to recursively expand
            console.log('Expand all not yet implemented');
        }}

        function collapseAll() {{
            // This would require more complex logic to recursively collapse
            console.log('Collapse all not yet implemented');
        }}

        // Initial zoom reset
        resetZoom();
    </script>
</body>
</html>"""

    return html
