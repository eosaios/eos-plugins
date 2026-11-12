import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
import { MessageChannel } from 'node:worker_threads';

// Exercise the generated page and real SDK, with an explicit synthetic DOM/host.
// This does not execute a browser's CSP or the desktop native isolation layer.
const html = await readFile(new URL('package/ui/index.html', import.meta.url), 'utf8');
assert(!/<script[^>]+src=|<link[^>]+href=/i.test(html), 'page must be self-contained');
const script = html.match(/<script>([\s\S]*)<\/script>/)[1];
const listeners = new Map();
const elements = Object.fromEntries(['status', 'draft', 'save', 'title'].map(id => [id, {
  textContent: '', value: '', disabled: id === 'save', onclick: undefined,
}]));
const parent = {};
const window = {
  __EOS_PLUGIN_NONCE__: 'synthetic-nonce', parent,
  setTimeout, clearTimeout,
  addEventListener(name, listener) { listeners.set(name, listener); },
  removeEventListener(name) { listeners.delete(name); },
};
const channel = new MessageChannel();
const store = new Map([['draft', 'previous draft']]);
let rejectWrite = false;
channel.port1.on('message', ({id, method, params}) => {
  if (rejectWrite && method === 'storage.set') {
    channel.port1.postMessage({id, error: 'GRANT_REVOKED'}); return;
  }
  let result;
  if (method === 'configuration.get') result = {title: 'Synthetic title'};
  else if (method === 'storage.get') result = store.get(params.key) ?? null;
  else if (method === 'storage.set') { store.set(params.key, params.value); result = null; }
  else throw new Error('Unexpected method: ' + method);
  channel.port1.postMessage({id, result});
});
try {
  vm.runInNewContext(script, {window, document: {querySelector: selector => elements[selector.slice(1)]}});
  const receive = listeners.get('message');
  assert.equal(typeof receive, 'function');
  receive({source: parent, data: {type:'eos-plugin:init', nonce:'wrong'}, ports:[channel.port2]});
  assert.equal(listeners.has('message'), true, 'wrong nonce must not complete handshake');
  receive({source: parent, data: {type:'eos-plugin:init', nonce:'synthetic-nonce'}, ports:[channel.port2]});
  const deadline = Date.now() + 3000;
  while (elements.save.disabled && Date.now() < deadline) await new Promise(resolve => setTimeout(resolve, 5));
  assert.equal(elements.save.disabled, false, elements.status.textContent);
  assert.equal(elements.title.textContent, 'Synthetic title');
  assert.equal(elements.draft.value, 'previous draft');
  elements.draft.value = 'new draft';
  await elements.save.onclick();
  assert.equal(store.get('draft'), 'new draft');
  assert.equal(elements.status.textContent, '已保存');
  rejectWrite = true;
  await elements.save.onclick();
  assert.match(elements.status.textContent, /GRANT_REVOKED/);
  assert.equal(elements.save.disabled, false);
  console.log('Page SDK handshake/configuration/storage and error-display checks passed (synthetic DOM)');
} finally {
  listeners.get('pagehide')?.();
  channel.port1.close(); channel.port2.close();
}
