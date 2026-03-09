import { invoke } from "@tauri-apps/api/core";

export async function loadData(path: string): Promise<any> {
  try {
    return await invoke("load_data", { path });
  } catch (e) {
    console.error("Failed to load data:", e);
    throw e;
  }
}

export async function playAudio(
  projectPath: string,
  type: string,
  name: string,
  volume: number = 1.0,
) {
  try {
    await invoke("preview_audio", {
      projectPath,
      assetType: type,
      assetName: name,
      volume,
    });
  } catch (e) {
    console.error("Failed to play audio:", e);
  }
}

export async function stopAudio() {
  await invoke("stop_audio");
}
