import React from "react";
import ReactDOM from "react-dom/client";
import "@/styles/global.css";
import { RemoteApp } from "./RemoteApp";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <RemoteApp />
  </React.StrictMode>,
);
