import { getVersion } from "@tauri-apps/api/app";
import { byId } from "../util/dom";


export async function openAbout(): Promise<void> {
    try {
        byId("aboutVer").textContent = "version " + (await getVersion());
    } catch {
        // Without a version the dialog still makes sense.
    }

    byId<HTMLDialogElement>("about").showModal();
}
