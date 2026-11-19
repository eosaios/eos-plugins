import { connectPlugin } from '../../../sdk/typescript/src/client';

const status = document.querySelector<HTMLParagraphElement>('#status')!;
const draft = document.querySelector<HTMLTextAreaElement>('#draft')!;
const save = document.querySelector<HTMLButtonElement>('#save')!;

async function start() {
  const client = await connectPlugin();
  window.addEventListener('pagehide', () => client.close());
  const configuration = await client.request<{ title: string }>('configuration.get');
  document.querySelector('#title')!.textContent = configuration.title;
  const previous = await client.request<string | null>('storage.get', { key: 'draft' });
  draft.value = typeof previous === 'string' ? previous : '';
  status.textContent = '已连接。内容仅保存在当前插件安装内。';
  save.disabled = false;
  save.onclick = async () => {
    save.disabled = true;
    try {
      await client.request('storage.set', { key: 'draft', value: draft.value });
      status.textContent = '已保存';
    } catch (error) {
      status.textContent = String(error);
    } finally {
      save.disabled = false;
    }
  };
}
start().catch(error => { status.textContent = String(error); });
