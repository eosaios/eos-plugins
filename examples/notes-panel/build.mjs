import { build } from 'esbuild';
import { mkdir, writeFile } from 'node:fs/promises';

// No resource server is exposed by srcdoc: ship one inline HTML document.
const result = await build({
  entryPoints: ['src/main.ts'], bundle: true, write: false,
  format: 'iife', platform: 'browser', target: 'es2020',
});
const script = result.outputFiles[0].text.replace(/<\/script/gi, '<\\/script');
await mkdir('package/ui', { recursive: true });
await writeFile('package/ui/index.html', `<!doctype html><html lang="zh"><meta charset="utf-8">
<title>便签示例</title><style>
body{font:16px system-ui;max-width:600px;margin:24px auto;padding:16px;color:#26323e}
textarea{box-sizing:border-box;width:100%;min-height:180px;font:inherit;padding:12px}
button{font:inherit;padding:8px 20px;margin-top:12px}p{min-height:24px}
</style><body><h1 id="title">便签</h1><label for="draft">内容</label>
<textarea id="draft" maxlength="20000"></textarea><button id="save" disabled>保存</button>
<p id="status" role="status">正在连接宿主…</p><script>${script}</script></body></html>`);
console.log('Built package/ui/index.html (inline SDK, script and styles)');
