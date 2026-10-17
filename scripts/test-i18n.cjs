// Run with: node scripts/test-i18n.cjs (no browser dependencies).
const { readFileSync } = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const html = readFileSync(require('node:path').join(__dirname, '../web/index.html'), 'utf8');
const init = html.slice(html.indexOf('  // Language preference:'), html.indexOf('  function defaultServerUrl()'));
function setup({ header = '', cookie = '', browser = ['en-US'], prefix = '' } = {}) {
  const buttons = [];
  function element() { return { dataset: {}, hidden: true, setAttribute() {}, addEventListener(event, fn) { this[event] = fn; }, focus() {} }; }
  const menu = element();
  menu.appendChild = button => buttons.push(button);
  const document = {
    cookie, documentElement: {}, querySelectorAll: () => [],
    getElementById: id => id === 'languageMenu' ? menu : element(),
    createElement: element, addEventListener() {},
  };
  const context = { document, navigator: { languages: browser }, PATH_PREFIX: prefix, location: { protocol: 'https:', reload() {} } };
  vm.createContext(context);
  vm.runInContext(init.replace('"__POCKETSHARE_ACCEPT_LANGUAGE__"', JSON.stringify(header)) + '\nglobalThis.result = { language, t, normalizeLanguage, TRANSLATIONS };', context);
  return { ...context.result, document, buttons };
}
for (const [header, expected] of [
  ['fr-CA,zh;q=0.9', 'fr'], ['en;q=0.2,zh-TW;q=0.9', 'zh-TW'],
  ['zh-Hant-HK', 'zh-TW'], ['zh-HK', 'zh-TW'], ['zh-CN', 'zh'],
  ['ja-JP', 'ja'], ['ko-KR', 'ko'], ['de-DE', 'de'], ['es-MX', 'es'],
  ['jp', 'ja'], ['kr', 'ko'], ['fr;q=0,en;q=0.1', 'en'],
]) assert.equal(setup({ header }).language, expected, header);
assert.equal(setup({ header: 'fr', cookie: 'share.lang=zh-TW' }).language, 'zh-TW');
assert.equal(setup({ cookie: 'share.lang=%broken', browser: ['ko'] }).language, 'ko');
assert.equal(setup({ cookie: 'share.lang=invalid', browser: ['ja'] }).language, 'ja');
assert.equal(setup({ browser: ['xx'] }).language, 'en');
for (const code of ['en', 'zh', 'zh-TW', 'ja', 'ko', 'fr', 'de', 'es']) {
  const app = setup({ header: code });
  assert.equal(app.document.documentElement.lang, code === 'zh' ? 'zh-CN' : code);
  for (const [key, values] of Object.entries(app.TRANSLATIONS)) {
    assert.equal(values.length, 7, key);
    assert.ok(app.t(key));
  }
}
const app = setup({ prefix: '/share' });
app.buttons[2].click();
assert.match(app.document.cookie, /^share.lang=zh-TW; Path=\/share; Max-Age=31536000; SameSite=Lax; Secure$/);
// Every static marker and runtime call must have a translation.
for (const match of html.matchAll(/data-i18n(?:-(?:title|aria-label|placeholder))?="([^"]+)"|\bt\(["']([^"']+)["']\)/g)) {
  assert.ok(Object.hasOwn(app.TRANSLATIONS, match[1] || match[2]), match[0]);
}
console.log('i18n checks passed: eight languages, header weights, aliases, fallback, cookie priority and persistence, translation coverage');
