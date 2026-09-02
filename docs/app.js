const routes = [...document.querySelectorAll('[data-page]')];
const links = [...document.querySelectorAll('[data-route]')];
const routeOrder = routes.map((page) => page.dataset.page);
const routeTitle = new Map(routes.map((page) => [page.dataset.page, page.querySelector('h1').textContent]));
const toast = document.querySelector('#toast');

const codeKeywords = new Set([
  'as', 'assert', 'async', 'await', 'break', 'class', 'const', 'continue', 'data',
  'def', 'defer', 'else', 'enum', 'export', 'extends', 'false', 'fn', 'for', 'from', 'fun', 'func', 'if',
  'impl', 'import', 'implementation', 'in', 'interface', 'let', 'match', 'mod', 'mut',
  'new', 'nil', 'None', 'null', 'package', 'pub', 'public', 'return', 'self', 'Self', 'Some',
  'struct', 'throw', 'throws', 'true', 'try', 'type', 'unsafe', 'using', 'val', 'var', 'where', 'while',
  'with', 'yield',
]);

function appendToken(fragment, type, value) {
  const node = type ? document.createElement('span') : document.createTextNode(value);
  if (type) {
    node.className = `tok-${type}`;
    node.textContent = value;
  }
  fragment.append(node);
}

function highlight(code, language) {
  const source = code.textContent;
  const fragment = document.createDocumentFragment();
  const hashComments = !['json', 'typescript', 'javascript', 'rust', 'swift', 'kotlin'].includes(language);
  let index = 0;
  while (index < source.length) {
    const rest = source.slice(index);
    const comment = rest.startsWith('//') || (hashComments && rest[0] === '#');
    if (comment) {
      const end = source.indexOf('\n', index);
      const stop = end === -1 ? source.length : end;
      appendToken(fragment, 'comment', source.slice(index, stop));
      index = stop;
      continue;
    }
    if (rest[0] === '"' || rest[0] === "'" || rest[0] === '`') {
      const quote = rest[0];
      let end = index + 1;
      while (end < source.length) {
        if (source[end] === '\\') end += 2;
        else if (source[end++] === quote) break;
        else if (quote !== '`' && source[end - 1] === '\n') break;
      }
      appendToken(fragment, 'string', source.slice(index, end));
      index = end;
      continue;
    }
    const number = rest.match(/^-?(?:0x[\da-f]+|\d+(?:\.\d+)?)(?:[eE][+-]?\d+)?/i);
    if (number) {
      appendToken(fragment, 'number', number[0]);
      index += number[0].length;
      continue;
    }
    const identifier = rest.match(/^[A-Za-z_$][\w$]*/);
    if (identifier) {
      const value = identifier[0];
      appendToken(fragment, codeKeywords.has(value) ? 'keyword' : null, value);
      index += value.length;
      continue;
    }
    appendToken(fragment, null, rest[0]);
    index += 1;
  }
  code.replaceChildren(fragment);
  code.parentElement.dataset.language = language;
}

document.querySelectorAll('pre > code').forEach((code) => {
  const explicitClass = [...code.classList].find((c) => c.startsWith('language-'))?.replace('language-', '');
  const panel = code.closest('[data-code-language]');
  const page = code.closest('[data-page]')?.dataset.page;
  const language = explicitClass || code.dataset.lang || panel?.dataset.codeLanguage || ({
    python: 'python',
    typescript: 'typescript',
    rust: 'rust',
    go: 'go',
    kotlin: 'kotlin',
    apple: 'swift',
    storage: 'rust',
    telemetry: 'json',
    benchmarks: 'text',
    'device-lab': 'bash',
    publishing: 'bash',
  }[page]) || 'text';
  highlight(code, language);
});

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
