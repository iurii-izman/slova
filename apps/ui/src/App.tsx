import { createSignal, onMount } from "solid-js";

// Tauri invoke
import { invoke } from "@tauri-apps/api/tauri";

function App() {
  const [status, setStatus] = createSignal("unknown");
  const [version, setVersion] = createSignal("");

  onMount(async () => {
    try {
      const res: any = await invoke("health_check");
      if (res && res.ok) {
        setStatus("connected");
        setVersion(res.version || "");
      } else {
        setStatus("error");
      }
    } catch (e) {
      console.error(e);
      setStatus("error");
    }
  });

  async function emitDemo() {
    try {
      await invoke("emit_demo_event");
      console.log("emit_demo_event invoked");
    } catch (e) {
      console.error(e);
    }
  }

  return (
    <div style={{ padding: "24px", "font-family": "sans-serif" }}>
      <h1>VideoTranscriber</h1>
      <div>
        Backend status: <strong>{status()}</strong>
        {version() && <span> — v{version()}</span>}
      </div>

      <section style={{ margin: "24px 0 0 0" }}>
        <h2>Queue</h2>
        <div
          style={{
            border: "1px dashed #ccc",
            padding: "16px",
            height: "200px",
            display: "flex",
            "align-items": "center",
            "justify-content": "center",
          }}
        >
          {/* Empty queue placeholder */}
        </div>
        <div style={{ "margin-top": "12px" }}>
          <button onClick={emitDemo}>Emit demo event</button>
        </div>
      </section>
    </div>
  );
}

export default App;
