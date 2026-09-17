import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { Toast } from "./components/Toast";
import "./styles.css";
import "./theme.css";
import "./app.css";

function windowLabel(): string {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
}

const isToast = windowLabel() === "toast";
if (isToast) document.documentElement.classList.add("is-toast");

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{isToast ? <Toast /> : <App />}</React.StrictMode>
);
