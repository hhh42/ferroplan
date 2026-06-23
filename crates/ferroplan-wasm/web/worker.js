// Web Worker: runs the wasm planner OFF the main thread, so a heavy solve never
// freezes the page and can be cancelled (terminate + recreate the worker). The
// main thread falls back to a synchronous in-page solve when workers are
// unavailable (file:// origin, strict CSP, etc.) — see the "run on" toggle.
import init, { plan } from './pkg/ferroplan_wasm.js';

const ready = init();

self.onmessage = async (e) => {
  const { id, domain, problem, mode, flags } = e.data;
  try {
    await ready;
    const res = plan(domain, problem, mode, flags);
    self.postMessage({ id, res });
  } catch (err) {
    self.postMessage({ id, res: JSON.stringify({ error: String(err) }) });
  }
};
