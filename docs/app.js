// Qenlo Documentation Application Script
const routes = [...document.querySelectorAll('.docs-article')];
const navLinks = [...document.querySelectorAll('.nav-link')];
const routeMap = new Map(routes.map((el) => [el.dataset.page, el]));
const routeOrder = routes.map((el) => el.dataset.page);
const routeTitles = new Map(routes.map((el) => [
  el.dataset.page,
  el.querySelector('h1')?.textContent.trim() || el.dataset.page
]));

const toast = document.querySelector('#toast');
const searchInput = document.querySelector('#doc-search');
const searchModal = document.querySelector('#search-modal');
const sidebar = document.querySelector('#sidebar');
const backdrop = document.querySelector('#sidebar-backdrop');
const prevBtn = document.querySelector('#prev-page-btn');
const nextBtn = document.querySelector('#next-page-btn');

function showToast(message) {
  if (!toast) return;
  toast.textContent = message;
  toast.classList.add('show');
  clearTimeout(showToast.timer);
  showToast.timer = setTimeout(() => toast.classList.remove('show'), 2000);
}

function resolveRoute() {
  const hash = window.location.hash.replace(/^#/, '').trim();
  if (!hash) return 'overview';
  
  if (routeMap.has(hash)) return hash;
  
  // If hash is an element ID inside an article, find its parent article
  const targetEl = document.getElementById(hash);
  if (targetEl) {
    const parentArticle = targetEl.closest('.docs-article');
    if (parentArticle && parentArticle.dataset.page) {
      return parentArticle.dataset.page;
    }
  }
  
  return 'overview';
}

function navigate(pageId, targetHash) {
  const activeArticle = routeMap.get(pageId) || routeMap.get('overview');
  if (!activeArticle) return;
  
  routes.forEach((article) => {
    article.classList.toggle('active', article === activeArticle);
  });
  
  navLinks.forEach((link) => {
    const isActive = link.dataset.route === activeArticle.dataset.page;
    link.classList.toggle('active', isActive);
    if (isActive) {
      link.setAttribute('aria-current', 'page');
    } else {
      link.removeAttribute('aria-current');
    }
  });
  
  const title = routeTitles.get(activeArticle.dataset.page);
  document.title = `${title} | Qenlo Documentation`;
  
  const currentIndex = routeOrder.indexOf(activeArticle.dataset.page);
  const prevPage = routeOrder[currentIndex - 1];
  const nextPage = routeOrder[currentIndex + 1];
  
  if (prevBtn) {
    if (prevPage) {
      prevBtn.hidden = false;
      prevBtn.href = `#${prevPage}`;
      prevBtn.querySelector('.page-nav-title').textContent = routeTitles.get(prevPage);
    } else {
      prevBtn.hidden = true;
    }
  }
  
  if (nextBtn) {
    if (nextPage) {
      nextBtn.hidden = false;
      nextBtn.href = `#${nextPage}`;
      nextBtn.querySelector('.page-nav-title').textContent = routeTitles.get(nextPage);
    } else {
      nextBtn.hidden = true;
    }
  }
  
  if (sidebar) sidebar.classList.remove('open');
  if (backdrop) backdrop.classList.remove('active');
  
  if (targetHash && targetHash !== activeArticle.dataset.page) {
    const element = document.getElementById(targetHash);
    if (element) {
      setTimeout(() => element.scrollIntoView({ behavior: 'smooth', block: 'start' }), 50);
      return;
    }
  }
  window.scrollTo({ top: 0, behavior: 'instant' });
}

function handleHashChange() {
  const rawHash = window.location.hash.replace(/^#/, '').trim();
  const pageId = resolveRoute();
  navigate(pageId, rawHash);
}

// Code Copy Buttons
document.querySelectorAll('.btn-copy').forEach((btn) => {
  btn.addEventListener('click', async () => {
    const container = btn.closest('.code-container');
    const code = container?.querySelector('pre code')?.textContent || '';
    try {
      await navigator.clipboard.writeText(code);
      const originalText = btn.textContent;
      btn.textContent = 'Copied!';
      btn.classList.add('copied');
      setTimeout(() => {
        btn.textContent = originalText;
        btn.classList.remove('copied');
      }, 2000);
    } catch {
      showToast('Failed to copy code');
    }
  });
});

// Search functionality
if (searchInput && searchModal) {
  function performSearch() {
    const query = searchInput.value.trim().toLowerCase();
    if (!query) {
      searchModal.hidden = true;
      searchModal.innerHTML = '';
      return;
    }
    
    const results = [];
    routes.forEach((article) => {
      const pageId = article.dataset.page;
      const title = routeTitles.get(pageId) || pageId;
      const summary = article.dataset.summary || '';
      const text = article.textContent || '';
      
      if (title.toLowerCase().includes(query) || summary.toLowerCase().includes(query) || text.toLowerCase().includes(query)) {
        results.push({ pageId, title, summary });
      }
    });
    
    if (results.length === 0) {
      searchModal.innerHTML = '<div class="search-empty">No results found for "' + query.replace(/</g, '&lt;') + '"</div>';
    } else {
      searchModal.innerHTML = results.slice(0, 8).map((r) => `
        <a class="search-item" href="#${r.pageId}">
          <div class="search-item-title">${r.title}</div>
          <div class="search-item-desc">${r.summary}</div>
        </a>
      `).join('');
    }
    searchModal.hidden = false;
  }

  searchInput.addEventListener('input', performSearch);
  
  searchModal.addEventListener('click', (e) => {
    const item = e.target.closest('.search-item');
    if (item) {
      searchModal.hidden = true;
      searchInput.value = '';
    }
  });
  
  document.addEventListener('click', (e) => {
    if (!e.target.closest('.search-box') && !e.target.closest('#search-modal')) {
      searchModal.hidden = true;
    }
  });
  
  document.addEventListener('keydown', (e) => {
    if ((e.key === '/' || (e.key === 'k' && (e.metaKey || e.ctrlKey))) && document.activeElement !== searchInput) {
      e.preventDefault();
      searchInput.focus();
    }
    if (e.key === 'Escape') {
      searchModal.hidden = true;
      searchInput.blur();
    }
  });
}

// Theme Toggle
const themeBtn = document.querySelector('#theme-button');
if (themeBtn) {
  function toggleTheme() {
    const currentTheme = document.documentElement.dataset.theme || (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
    const nextTheme = currentTheme === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = nextTheme;
    localStorage.setItem('qenlo-theme', nextTheme);
  }
  
  themeBtn.addEventListener('click', toggleTheme);
  const saved = localStorage.getItem('qenlo-theme');
  if (saved) document.documentElement.dataset.theme = saved;
}

// Mobile navigation
const menuBtn = document.querySelector('#menu-button');
if (menuBtn && sidebar && backdrop) {
  menuBtn.addEventListener('click', () => {
    sidebar.classList.toggle('open');
    backdrop.classList.toggle('active');
  });
  
  backdrop.addEventListener('click', () => {
    sidebar.classList.remove('open');
    backdrop.classList.remove('active');
  });
}

// Initial navigation
window.addEventListener('hashchange', handleHashChange);
handleHashChange();

