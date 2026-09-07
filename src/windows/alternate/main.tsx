import React from "react";
import ReactDOM from "react-dom/client";
import "@/styles/global.css";
import { AlternateApp } from "./AlternateApp";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <AlternateApp />
  </React.StrictMode>,
);
