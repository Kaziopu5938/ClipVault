import React from "react";
import ReactDOM from "react-dom/client";
import "./styles.css";
import { App } from "./App";
import { PickerApp } from "./PickerApp";
import { AppProvider } from "./lib/app-context";
import { NavProvider } from "./lib/navigation";
import { ToastProvider } from "./components/Toast";

const picker = new URLSearchParams(window.location.search).has("picker");
if (picker) document.documentElement.classList.add("picker-mode");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {picker ? (
      <AppProvider>
        <ToastProvider>
          <PickerApp />
        </ToastProvider>
      </AppProvider>
    ) : (
      <AppProvider>
        <NavProvider>
          <ToastProvider>
            <App />
          </ToastProvider>
        </NavProvider>
      </AppProvider>
    )}
  </React.StrictMode>,
);
