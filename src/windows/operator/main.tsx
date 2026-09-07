import React from "react";
import ReactDOM from "react-dom/client";
import "@/styles/global.css";
import { OperatorApp } from "./OperatorApp";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <OperatorApp />
  </React.StrictMode>,
);
