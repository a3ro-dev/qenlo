"""
Generate refined, crisp, editorial social card and render 1200x630 card.png.
"""
import os
from playwright.sync_api import sync_playwright

card_svg_content = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 630" width="1200" height="630" fill="none">
  <!-- Background Canvas -->
  <rect width="1200" height="630" fill="#171C19" />

  <!-- Subtle Editorial Grid Rules -->
  <g stroke="#2E3833" stroke-width="1" opacity="0.6">
    <line x1="80" y1="0" x2="80" y2="630" />
    <line x1="680" y1="0" x2="680" y2="630" />
    <line x1="1120" y1="0" x2="1120" y2="630" />
    <line x1="0" y1="80" x2="1200" y2="80" />
    <line x1="0" y1="550" x2="1200" y2="550" />
  </g>

  <!-- Corner Registration Marks (Research Instrument Style) -->
  <g stroke="#6E7C74" stroke-width="1.5">
    <polyline points="80,95 80,80 95,80" fill="none" />
    <polyline points="1105,80 1120,80 1120,95" fill="none" />
    <polyline points="80,535 80,550 95,550" fill="none" />
    <polyline points="1105,550 1120,550 1120,535" fill="none" />
  </g>

  <!-- Left Column: Typography & Technical Positioning -->
  <g transform="translate(100, 120)">
    <!-- Research Preview Pill -->
    <g transform="translate(0, 0)">
      <rect width="164" height="28" rx="2" fill="#1F2622" stroke="#2E3833" stroke-width="1" />
      <circle cx="16" cy="14" r="3.5" fill="#EF8B79" />
      <text x="28" y="18" font-family="'IBM Plex Mono', monospace" font-size="11" font-weight="400" fill="#AAB3AD" letter-spacing="1">RESEARCH PREVIEW</text>
    </g>

    <!-- Main Headline -->
    <text x="0" y="95" font-family="'IBM Plex Sans', sans-serif" font-size="46" font-weight="600" fill="#F7F5F0">
      <tspan x="0" dy="0">Native vector search,</tspan>
      <tspan x="0" dy="56" fill="#EF8B79">in development.</tspan>
    </text>

    <!-- Approved Factual Description -->
    <text x="0" y="240" font-family="'IBM Plex Sans', sans-serif" font-size="19" font-weight="400" fill="#AAB3AD">
      <tspan x="0" dy="0">Qenlo is a Rust vector-database research project</tspan>
      <tspan x="0" dy="30">for native applications.</tspan>
    </text>

    <!-- Technical Parameter Row -->
    <g transform="translate(0, 315)">
      <g transform="translate(0, 0)">
        <text font-family="'IBM Plex Mono', monospace" font-size="11" fill="#6E7C74" letter-spacing="0.5">TARGET</text>
        <text y="20" font-family="'IBM Plex Mono', monospace" font-size="13" fill="#F7F5F0">DESKTOP FIRST</text>
      </g>
      <g transform="translate(160, 0)">
        <text font-family="'IBM Plex Mono', monospace" font-size="11" fill="#6E7C74" letter-spacing="0.5">RUNTIME</text>
        <text y="20" font-family="'IBM Plex Mono', monospace" font-size="13" fill="#F7F5F0">EMBEDDED RUST</text>
      </g>
      <g transform="translate(320, 0)">
        <text font-family="'IBM Plex Mono', monospace" font-size="11" fill="#6E7C74" letter-spacing="0.5">STATUS</text>
        <text y="20" font-family="'IBM Plex Mono', monospace" font-size="13" fill="#EF8B79">SEARCH PROTOTYPE</text>
      </g>
    </g>
  </g>

  <!-- Right Column: Hero Simplified Vector Emblem -->
  <g transform="translate(860, 305) scale(0.68)">
    <!-- Translate origin to center (256, 256) -->
    <g transform="translate(-256, -256)">
      <!-- Coordinate Lattice Grid -->
      <g stroke="#2E3833" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
        <line x1="256" y1="100" x2="256" y2="412" />
        <line x1="100" y1="256" x2="412" y2="256" />
        <line x1="178" y1="145" x2="334" y2="367" />
        <line x1="334" y1="145" x2="178" y2="367" />
        <polygon points="256,166 334,211 334,301 256,346 178,301 178,211" fill="none" />
        <polygon points="256,206 299,231 299,281 256,306 213,281 213,231" fill="none" stroke-dasharray="4 4" stroke="#6E7C74" />
        <line x1="178" y1="211" x2="334" y2="211" />
        <line x1="178" y1="301" x2="334" y2="301" />
        <line x1="213" y1="170" x2="213" y2="342" />
        <line x1="299" y1="170" x2="299" y2="342" />
      </g>

      <!-- Faint Intermediate Lattice Nodes -->
      <g fill="#6E7C74">
        <polygon points="256,160 261,163 261,169 256,172 251,169 251,163" />
        <polygon points="256,340 261,343 261,349 256,352 251,349 251,343" />
        <polygon points="178,205 183,208 183,214 178,217 173,214 173,208" />
        <polygon points="178,295 183,298 183,304 178,307 173,304 173,298" />
        <polygon points="334,205 339,208 339,214 334,217 329,214 329,208" />
      </g>

      <!-- Discrete Coral Red Accent Node -->
      <g>
        <circle cx="334" cy="301" r="10" fill="none" stroke="#EF8B79" stroke-width="1.5" opacity="0.6" />
        <polygon points="334,293 341,297 341,305 334,309 327,305 327,297" fill="#EF8B79" />
      </g>

      <!-- Primary Structural Outer Boundary -->
      <g stroke="#F7F5F0" stroke-width="5.5" stroke-linecap="square" stroke-linejoin="miter">
        <line x1="256" y1="100" x2="178" y2="145" />
        <line x1="256" y1="100" x2="334" y2="145" />
        <line x1="256" y1="412" x2="178" y2="367" />
        <line x1="256" y1="412" x2="334" y2="367" />
        <line x1="178" y1="145" x2="178" y2="367" />
        <line x1="334" y1="145" x2="334" y2="367" />
        <polyline points="178,145 118,180 118,246 100,256 118,266 118,332 178,367" fill="none" />
        <polyline points="334,145 394,180 394,246 412,256 394,266 394,332 334,367" fill="none" />
      </g>

      <!-- Primary Hexagonal Vertex Nodes -->
      <g fill="#F7F5F0">
        <polygon points="256,86 268,93 268,107 256,114 244,107 244,93" />
        <polygon points="256,398 268,405 268,419 256,426 244,419 244,405" />
        <polygon points="100,242 112,249 112,263 100,270 88,263 88,249" />
        <polygon points="412,242 424,249 424,263 412,270 400,263 400,249" />
        <polygon points="178,134 188,140 188,150 178,156 168,150 168,140" />
        <polygon points="334,134 344,140 344,150 334,156 324,150 324,140" />
        <polygon points="178,356 188,362 188,372 178,378 168,372 168,362" />
        <polygon points="334,356 344,362 344,372 334,378 324,372 324,362" />
      </g>

      <!-- 4 Cardinal Bus Terminals -->
      <g stroke="#F7F5F0" stroke-width="5.5" stroke-linecap="square" stroke-linejoin="miter">
        <line x1="256" y1="86" x2="256" y2="28" />
        <polyline points="244,93 236,75 236,28" fill="none" />
        <polyline points="268,93 276,75 276,28" fill="none" />
        <line x1="256" y1="426" x2="256" y2="484" />
        <polyline points="244,419 236,437 236,484" fill="none" />
        <polyline points="268,419 276,437 276,484" fill="none" />
        <line x1="88" y1="256" x2="28" y2="256" />
        <polyline points="95,244 77,236 28,236" fill="none" />
        <polyline points="95,268 77,276 28,276" fill="none" />
        <line x1="424" y1="256" x2="484" y2="256" />
        <polyline points="417,244 435,236 484,236" fill="none" />
        <polyline points="417,268 435,276 484,276" fill="none" />
      </g>

      <!-- Central Hexagon & Radiating Vector Axes -->
      <polygon points="256,232 277,244 277,268 256,280 235,268 235,244" fill="#F7F5F0" />
      <g stroke="#F7F5F0" stroke-width="5.5" stroke-linecap="square">
        <line x1="256" y1="232" x2="256" y2="190" />
        <line x1="256" y1="280" x2="256" y2="306" />
        <line x1="235" y1="268" x2="199" y2="289" />
        <line x1="277" y1="268" x2="313" y2="289" />
      </g>
    </g>

    <!-- Technical Coordinate Label -->
    <g transform="translate(120, 160)">
      <text font-family="'IBM Plex Mono', monospace" font-size="11" fill="#6E7C74" letter-spacing="1">INDEX_COORD: [0.866, 0.500]</text>
    </g>
  </g>

  <!-- Bottom Technical Footer Bar -->
  <g transform="translate(100, 565)">
    <!-- Small Horizontal Lockup Outlined -->
    <g transform="translate(0, 0)">
      <!-- Mini Mark -->
      <g transform="scale(0.045) translate(0, -100)">
        <polygon points="256,86 268,93 268,107 256,114 244,107 244,93" fill="#F7F5F0" />
        <polygon points="256,398 268,405 268,419 256,426 244,419 244,405" fill="#F7F5F0" />
        <polygon points="100,242 112,249 112,263 100,270 88,263 88,249" fill="#F7F5F0" />
        <polygon points="412,242 424,249 424,263 412,270 400,263 400,249" fill="#F7F5F0" />
        <polygon points="256,232 277,244 277,268 256,280 235,268 235,244" fill="#F7F5F0" />
        <polygon points="334,293 341,297 341,305 334,309 327,305 327,297" fill="#EF8B79" />
      </g>
      <!-- Outlined Wordmark 'Qenlo' (IBM Plex Sans SemiBold) -->
      <g transform="translate(24, -2) scale(0.24)" fill="#F7F5F0" fill-rule="evenodd">
        <!-- Q -->
        <path d="M 21.5 0 C 33.5 0 43 9.5 43 21.5 C 43 26.2 41.5 30.5 39 34 L 47.5 34 L 47.5 41 L 31.5 41 C 28.5 42.2 25.2 43 21.5 43 C 9.5 43 0 33.5 0 21.5 C 0 9.5 9.5 0 21.5 0 Z M 21.5 7.5 C 13.8 7.5 7.5 13.8 7.5 21.5 C 7.5 29.2 13.8 35.5 21.5 35.5 C 25.2 35.5 28.5 34 30.8 31.8 L 30.8 27 L 35.5 27 C 35.5 25.2 35.5 23.4 35.5 21.5 C 35.5 13.8 29.2 7.5 21.5 7.5 Z" />
        <!-- e -->
        <path d="M 70.5 10.5 C 79.5 10.5 86.5 17 87 26 L 57.5 26 C 58 31.8 62 35.8 68.5 35.8 C 73 35.8 76.8 33.8 78.5 30.2 L 85.5 32.5 C 82.5 38.5 76.5 42.5 68.5 42.5 C 57 42.5 50 34.5 50 23.5 C 50 12.5 57.5 10.5 70.5 10.5 Z M 70.2 16.5 C 63.5 16.5 58.5 20.5 57.8 24.8 L 79.5 24.8 C 79 20.5 74.8 16.5 70.2 16.5 Z" />
        <!-- n -->
        <path d="M 94.5 11.5 L 101.5 11.5 L 101.5 16 C 103.8 12.8 108.5 10.5 114.5 10.5 C 122.5 10.5 127.5 15.2 127.5 24 L 127.5 41.5 L 120 41.5 L 120 24.5 C 120 18.5 117 16.5 112.5 16.5 C 106.5 16.5 102 20.8 102 27.5 L 102 41.5 L 94.5 41.5 Z" />
        <!-- l -->
        <path d="M 135 0 L 142.5 0 L 142.5 41.5 L 135 41.5 Z" />
        <!-- o -->
        <path d="M 166 10.5 C 177 10.5 184.5 18.5 184.5 26.5 C 184.5 34.5 177 42.5 166 42.5 C 155 42.5 147.5 34.5 147.5 26.5 C 147.5 18.5 155 10.5 166 10.5 Z M 166 17.2 C 159.5 17.2 155 21.5 155 26.5 C 155 31.5 159.5 35.8 166 35.8 C 172.5 35.8 177 31.5 177 26.5 C 177 21.5 172.5 17.2 166 17.2 Z" />
      </g>
    </g>

    <!-- Technical Project Marker -->
    <text x="920" y="10" font-family="'IBM Plex Mono', monospace" font-size="11" fill="#6E7C74" letter-spacing="1">PROJECT // QENLODB</text>
  </g>
</svg>
'''

svg_path = 'd:/qenloDB/assets/brand/social/card.svg'
png_path = 'd:/qenloDB/assets/brand/social/card.png'

with open(svg_path, 'w', encoding='utf-8') as f:
    f.write(card_svg_content)

print('Updated card.svg')

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page(viewport={'width': 1200, 'height': 630}, device_scale_factor=1)
    page.goto(f'file:///{os.path.abspath(svg_path).replace(os.sep, "/")}')
    page.screenshot(path=png_path)
    browser.close()

print(f'Rendered card.png to {png_path}')
