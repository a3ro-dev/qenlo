const routes = [...document.querySelectorAll('[data-page]')];
const links = [...document.querySelectorAll('[data-route]')];
const routeOrder = routes.map((page) => page.dataset.page);
const routeTitle = new Map(routes.map((page) => [page.dataset.page, page.querySelector('h1').textContent]));
const toast = document.querySelector('#toast');

function notify(message) {
  toast.textContent = message;
  toast.classList.add('show');
  clearTimeout(notify.timer);
  notify.timer = setTimeout(() => toast.classList.remove('show'), 1800);
}

function currentRoute() {
  const route = location.hash.slice(1);
  return routeOrder.includes(route) ? route : 'overview';
}

function renderRoute() {
  const route = currentRoute();
  routes.forEach((page) => { page.hidden = page.dataset.page !== route; });
  links.forEach((link) => {
    const active = link.dataset.route === route;
    link.classList.toggle('active', active);
    active ? link.setAttribute('aria-current', 'page') : link.removeAttribute('aria-current');
  });
  document.title = `${routeTitle.get(route)} | Qenlo documentation`;
  const index = routeOrder.indexOf(route);
  setAdjacent(document.querySelector('#previous-page'), routeOrder[index - 1], 'Previous');
  setAdjacent(document.querySelector('#next-page'), routeOrder[index + 1], 'Next');
  document.querySelector('#sidebar').classList.remove('open');
  document.querySelector('#menu-button').setAttribute('aria-expanded', 'false');
  window.scrollTo({ top: 0, behavior: 'instant' });
}

function setAdjacent(link, route, label) {
  link.hidden = !route;
  if (!route) return;
  link.href = `#${route}`;
  link.innerHTML = `<small>${label}</small>${routeTitle.get(route)}`;
}

function copy(text, message) {
  navigator.clipboard.writeText(text).then(() => notify(message));
}

document.querySelectorAll('[data-ai-actions]').forEach((container) => {
  container.innerHTML = '<button type="button" data-ai="page">Copy page</button><button type="button" data-ai="chatgpt">Copy for ChatGPT</button><button type="button" data-ai="claude">Copy for Claude</button>';
  container.addEventListener('click', (event) => {
    const button = event.target.closest('[data-ai]');
    if (!button) return;
    const article = button.closest('article');
    const title = article.querySelector('h1').textContent;
    const body = article.innerText.replace(/Copy page|Copy for ChatGPT|Copy for Claude|Copy/g, '').trim();
    const prompts = {
      page: `# ${title}\n\n${body}`,
      chatgpt: `Use this Qenlo documentation to answer my next question accurately. Keep implementation limits explicit and do not invent unsupported behavior.\n\n# ${title}\n\n${body}`,
      claude: `Treat the following as the authoritative Qenlo documentation for my next question. Preserve its safety and research caveats.\n\n# ${title}\n\n${body}`,
    };
    copy(prompts[button.dataset.ai], button.dataset.ai === 'page' ? 'Page copied' : 'Prompt copied');
  });
});

document.querySelectorAll('.copy-code').forEach((button) => {
  button.addEventListener('click', () => copy(button.parentElement.querySelector('code').textContent, 'Code copied'));
});

document.querySelectorAll('.language-tabs').forEach((tabs) => {
  tabs.addEventListener('click', (event) => {
    const button = event.target.closest('[data-language]');
    if (!button) return;
    const article = tabs.closest('article');
    tabs.querySelectorAll('[data-language]').forEach((tab) => tab.setAttribute('aria-selected', String(tab === button)));
    article.querySelectorAll('[data-code-language]').forEach((panel) => { panel.hidden = panel.dataset.codeLanguage !== button.dataset.language; });
  });
});

const search = document.querySelector('#doc-search');
const results = document.querySelector('#search-results');
search.addEventListener('input', () => {
  const query = search.value.trim().toLowerCase();
  if (!query) { results.hidden = true; results.replaceChildren(); return; }
  const matches = routes.filter((page) => `${routeTitle.get(page.dataset.page)} ${page.dataset.summary} ${page.innerText}`.toLowerCase().includes(query)).slice(0, 8);
  results.innerHTML = matches.length ? matches.map((page) => `<a href="#${page.dataset.page}"><strong>${routeTitle.get(page.dataset.page)}</strong><small>${page.dataset.summary}</small></a>`).join('') : '<p>No documentation matched that search.</p>';
  results.hidden = false;
});
results.addEventListener('click', () => { search.value = ''; results.hidden = true; });
document.addEventListener('keydown', (event) => {
  if (event.key === '/' && document.activeElement !== search) { event.preventDefault(); search.focus(); }
  if (event.key === 'Escape') { search.value = ''; results.hidden = true; search.blur(); }
});

document.querySelector('#theme-button').addEventListener('click', () => {
  const current = document.documentElement.dataset.theme || (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
  const next = current === 'dark' ? 'light' : 'dark';
  document.documentElement.dataset.theme = next;
  localStorage.setItem('qenlo-theme', next);
});
const savedTheme = localStorage.getItem('qenlo-theme');
if (savedTheme) document.documentElement.dataset.theme = savedTheme;

document.querySelector('#menu-button').addEventListener('click', (event) => {
  const sidebar = document.querySelector('#sidebar');
  const open = sidebar.classList.toggle('open');
  event.currentTarget.setAttribute('aria-expanded', String(open));
});

document.querySelector('#mobile-search-button').addEventListener('click', (event) => {
  const open = document.querySelector('.search').classList.toggle('open');
  event.currentTarget.setAttribute('aria-expanded', String(open));
  if (open) search.focus();
});

window.addEventListener('hashchange', renderRoute);
renderRoute();
