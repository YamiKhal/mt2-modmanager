import { open } from "@tauri-apps/plugin-dialog";


export async function pickModFiles(): Promise<string[] | null> {
    const picked = await open({
        multiple: true,
        title: "Add mod: pick a mod .zip, or the manifest.json inside a mod folder",
        filters: [{ name: "Mod (.zip or manifest.json)", extensions: ["zip", "json"] }],
    });

    return picked;
}

export function pickFolder(title?: string): Promise<string | null> {
    return open({ directory: true, title });
}

export function pickGameDataZip(): Promise<string | null> {
    return open({ filters: [{ name: "Game data", extensions: ["zip"] }] });
}
