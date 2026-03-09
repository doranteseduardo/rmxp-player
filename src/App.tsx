import { useState, useCallback } from "react";
// import { Stage, Container, Text } from "@pixi/react";
// import * as PIXI from "pixi.js";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

// Custom invoke wrapper if api.ts is not fully integrated yet
async function loadData(path: string): Promise<any> {
  return await invoke("load_data", { path });
}

async function playAudio(
  projectPath: string,
  type: string,
  name: string,
  volume: number = 1.0,
) {
  await invoke("preview_audio", {
    projectPath,
    assetType: type,
    assetName: name,
    volume,
  });
}

export default function App() {
  const [data, setData] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [projectPath, setProjectPath] = useState<string | null>(null);

  const openProject = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });

      if (selected && typeof selected === "string") {
        setProjectPath(selected);
        setLoading(true);
        setError(null);
        try {
          console.log(
            "Loading system data from:",
            `${selected}/Data/System.rxdata`,
          );
          // Load System data to verify
          const sys = await loadData(`${selected}/Data/System.rxdata`);
          console.log("Data loaded:", sys);
          setData(sys);

          // Try playing Title BGM if available
          if (sys && sys.title_bgm && sys.title_bgm.name) {
            console.log("Playing BGM:", sys.title_bgm);
            const rmxpVol =
              sys.title_bgm.volume != null ? sys.title_bgm.volume : 100; // Default 100
            const volume = rmxpVol / 100.0;
            playAudio(selected, "bgm", sys.title_bgm.name, volume);
          }
        } catch (e: any) {
          console.error(e);
          setError(e.toString());
        } finally {
          setLoading(false);
        }
      }
    } catch (e) {
      console.error(e);
    }
  }, []);

  return (
    <div
      style={{
        width: "100vw",
        height: "100vh",
        display: "flex",
        flexDirection: "column",
        background: "#222",
      }}
    >
      <div
        style={{
          padding: 10,
          background: "#333",
          color: "white",
          display: "flex",
          gap: "10px",
          alignItems: "center",
        }}
      >
        <button
          onClick={openProject}
          style={{ padding: "8px 16px", cursor: "pointer" }}
        >
          Select Game Folder
        </button>
        {loading && <span> Loading...</span>}
        {error && <span style={{ color: "#ff6666" }}> {error}</span>}
        {projectPath && (
          <span style={{ fontSize: "0.8em", color: "#aaa" }}>
            {" "}
            {projectPath}
          </span>
        )}
      </div>

      <div
        style={{
          flex: 1,
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          color: "white",
          flexDirection: "column",
        }}
      >
        <div>PixiJS Disabled temporarily (Check console for errors)</div>
        {data && (
          <div
            style={{
              marginTop: 20,
              textAlign: "left",
              overflow: "auto",
              maxHeight: "500px",
              width: "80%",
              background: "#111",
              padding: 10,
            }}
          >
            <h2>Loaded Data:</h2>
            <h3>Game Title: {data.game_title}</h3>
            <pre>{JSON.stringify(data, null, 2)}</pre>
          </div>
        )}
      </div>
    </div>
  );
}
