// Two frames after load, as SermonAI's operator window does.
requestAnimationFrame(() => requestAnimationFrame(() => {
  window.__TAURI__.core.invoke("operator_window_ready").catch(() => {});
}));
