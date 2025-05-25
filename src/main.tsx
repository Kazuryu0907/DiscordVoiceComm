import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import {getVersion} from "@tauri-apps/api/app";

getVersion().then(console.log);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
