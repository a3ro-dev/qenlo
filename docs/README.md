<div class="docs-landing">
  <header class="docs-landing__nav" aria-label="Documentation home navigation">
    <a class="docs-landing__brand" href="../" aria-label="Qenlo home">
      <img src="../assets/brand/logo/lockup-reversed.svg" alt="Qenlo" width="94" height="28">
      <span>docs</span>
    </a>
    <nav class="docs-landing__nav-links" aria-label="Primary">
      <a href="#/architecture">architecture</a>
      <a href="#/verification">verification</a>
      <a href="https://github.com/a3ro-dev/qenlo" target="_blank" rel="noopener noreferrer">github</a>
    </nav>
  </header>
  <main id="main-content">
    <section class="docs-hero" aria-labelledby="docs-title">
      <div class="docs-hero__copy">
        <p class="docs-kicker"><span aria-hidden="true"></span> documentation / v0.1.0-alpha.4</p>
        <h1 id="docs-title">Know what exists.<br><strong>Know how it was found.</strong></h1>
        <p class="docs-hero__lede">Qenlo is an embedded vector store for durable, exact, metadata-filtered retrieval. Canonical records stay local. Every search path reports what actually ran.</p>
        <div class="docs-hero__actions">
          <a class="docs-button docs-button--primary" href="#/quickstart">Start the quickstart <span aria-hidden="true">→</span></a>
          <a class="docs-button docs-button--secondary" href="#/concepts">Read the core concepts</a>
        </div>
      </div>
      <div class="execution-trace" aria-label="Example Qenlo search execution trace">
        <div class="execution-trace__bar">
          <span>execution.report</span>
          <span class="execution-trace__status"><i aria-hidden="true"></i> complete</span>
        </div>
        <ol class="execution-trace__steps">
          <li><span>01</span><div><b>canonical store</b><small>durable records decide what exists</small></div><code>gen 0042</code></li>
          <li><span>02</span><div><b>metadata filter</b><small>eligible rows are resolved first</small></div><code>1,284 rows</code></li>
          <li><span>03</span><div><b>exact search</b><small>the selected route does the work</small></div><code>gpu-wgpu</code></li>
          <li><span>04</span><div><b>execution report</b><small>fallbacks and timings stay visible</small></div><code>verified</code></li>
        </ol>
        <div class="execution-trace__footer"><span>requested <b>auto</b></span><span>actual <b>gpu-wgpu</b></span><span>fallback <b>none</b></span></div>
      </div>
    </section>
    <section class="docs-principles" aria-label="Core properties">
      <p><span>01</span><strong>local-first</strong> no database server</p>
      <p><span>02</span><strong>exact by default</strong> deterministic results</p>
      <p><span>03</span><strong>observable</strong> routes explain themselves</p>
      <p><span>04</span><strong>embedded</strong> one canonical record store</p>
    </section>
    <section class="docs-paths" aria-labelledby="docs-paths-title">
      <div class="docs-section-heading">
        <p>Choose a path</p>
        <h2 id="docs-paths-title">Get to the right level of detail.</h2>
      </div>
      <div class="docs-path-list">
        <a href="#/quickstart">
          <span class="docs-path-list__number">01</span>
          <span><strong>Build with Qenlo</strong><small>Create, mutate, search, close, and reopen a collection.</small></span>
          <span class="docs-path-list__arrow" aria-hidden="true">↗</span>
        </a>
        <a href="#/architecture">
          <span class="docs-path-list__number">02</span>
          <span><strong>Understand the system</strong><small>Trace canonical storage, derived indexes, recovery, and routing.</small></span>
          <span class="docs-path-list__arrow" aria-hidden="true">↗</span>
        </a>
        <a href="#/verification">
          <span class="docs-path-list__number">03</span>
          <span><strong>Inspect the evidence</strong><small>See test coverage, device records, methodology, and open gates.</small></span>
          <span class="docs-path-list__arrow" aria-hidden="true">↗</span>
        </a>
      </div>
    </section>
    <section class="docs-sdks" aria-labelledby="docs-sdks-title">
      <div>
        <p class="docs-kicker">SDKs and bindings</p>
        <h2 id="docs-sdks-title">Use the language your application already speaks.</h2>
      </div>
      <nav class="docs-sdk-links" aria-label="SDK documentation">
        <a href="#/sdks/rust"><span>Rust</span><small>native API</small></a>
        <a href="#/sdks/python"><span>Python</span><small>native extension</small></a>
        <a href="#/sdks/typescript"><span>TypeScript</span><small>Node binding</small></a>
        <a href="#/sdks/go"><span>Go</span><small>FFI driver</small></a>
        <a href="#/sdks/kotlin"><span>Kotlin</span><small>Android</small></a>
        <a href="#/sdks/swift"><span>Swift</span><small>Apple</small></a>
      </nav>
    </section>
    <aside class="docs-boundary" aria-labelledby="docs-boundary-title">
      <p>Research-grade alpha</p>
      <div>
        <h2 id="docs-boundary-title">Measured before it is marketed.</h2>
        <p>Benchmark results belong to their recorded data, hardware, source revision, runtime, and timing boundary. They are evidence, not universal performance claims.</p>
      </div>
      <a href="#/benchmark-protocol">Read the benchmark protocol <span aria-hidden="true">→</span></a>
    </aside>
  </main>
  <footer class="docs-landing__footer">
    <span>embedded filtered vector search in Rust</span>
    <nav aria-label="Footer">
      <a href="#/feature-matrix">feature matrix</a>
      <a href="#/prioritized-roadmap">roadmap</a>
      <a href="../QENLO-RESEARCH-PAPER.pdf">research paper</a>
    </nav>
  </footer>
</div>
