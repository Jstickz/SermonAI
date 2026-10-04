import React from "react";
import ReactDOM from "react-dom/client";
import "@/styles/global.css";
import { OperatorApp } from "./OperatorApp";
import { display } from "@/lib/ipc";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <OperatorApp />
  </React.StrictMode>,
);

// Two frames after the root is scheduled: the first commits, the second has
// painted. That moment is the cold start PRD §9.1 budgets. The reading is
// taken here, not when the call lands, because the call may wait behind
// other commands on the main thread.
requestAnimationFrame(() => {
  requestAnimationFrame(() => {
    void display.operatorReady(Date.now()).catch(() => undefined);
  });
});
