import { onMount } from "solid-js";
import { Router, Route } from "@solidjs/router";
import { invoke } from "@tauri-apps/api/core";
import { QueuePage } from "./pages/QueuePage";
import { DetailPage } from "./pages/DetailPage";
import { SettingsPage } from "./pages/SettingsPage";
import { ToastContainer } from "./components/ToastContainer";
import { toastError, toastSuccess } from "./stores/toastStore";

function App() {
  onMount(async () => {
    try {
      const res: any = await invoke("health_check");
      if (res && res.ok) {
        console.log("Backend connected:", res.version);
        toastSuccess("Connected to backend");
      } else {
        console.error("Health check failed");
        toastError("Backend health check failed");
      }
    } catch (e) {
      console.error("Backend connection error:", e);
      toastError("Failed to connect to backend");
    }
  });

  return (
    <>
      <div style={{ "min-height": "100vh", "background-color": "#fafafa" }}>
        <Router>
          <Route path="/" component={QueuePage} />
          <Route path="/detail/:id" component={DetailPage} />
          <Route path="/settings" component={SettingsPage} />
        </Router>
      </div>
      <ToastContainer />
    </>
  );
}

export default App;
