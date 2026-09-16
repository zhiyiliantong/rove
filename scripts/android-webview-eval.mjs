// Development-only inspection through an explicitly forwarded ADB WebView port.
// Does not add a product API or expose an agent listener.
const expression = process.argv[2];
if (!expression) throw new Error('Usage: node scripts/android-webview-eval.mjs <expression>');
const pages = await (await fetch('http://127.0.0.1:19222/json')).json();
const page = pages.find(p => p.url === 'http://tauri.localhost/' && p.title.startsWith('Rove'));
if (!page) throw new Error('Rove WebView not found');
const ws = new WebSocket(page.webSocketDebuggerUrl);
const timeout = setTimeout(() => { console.error('WebView evaluation timed out'); process.exit(1); }, 20000);
ws.addEventListener('open', () => ws.send(JSON.stringify({id:1, method:'Runtime.evaluate', params:{expression, awaitPromise:true, returnByValue:true}})));
ws.addEventListener('message', event => {
  const reply = JSON.parse(event.data);
  if (reply.id !== 1) return;
  clearTimeout(timeout);
  if (reply.error || reply.result.exceptionDetails) {
    console.error(JSON.stringify(reply.error || reply.result.exceptionDetails));
    process.exitCode = 1;
  } else console.log(JSON.stringify(reply.result.result.value, null, 2));
  ws.close();
});
ws.addEventListener('error', () => { clearTimeout(timeout); console.error('WebView connection failed'); process.exitCode = 1; });
