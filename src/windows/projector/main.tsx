import React from "react";
import ReactDOM from "react-dom/client";
import "@/styles/global.css";
import { ProjectorApp } from "./ProjectorApp";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ProjectorApp />
  </React.StrictMode>,
);
