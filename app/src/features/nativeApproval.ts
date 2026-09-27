import * as commands from "../api/commands";
import type { NativeMod } from "../api/types";
import { runAction } from "../ui/busy";
import { log } from "../ui/log";
import { byId } from "../util/dom";
import { escapeHtml } from "../util/text";


// Asked before Apply and Launch. Returns the approved native-code mods (empty when none need asking),
// or null when the player cancels.
export async function approveNativeCode(): Promise<string[] | null> {
    const request = await runAction("Check native code", () => commands.getNativeRequest());

    if (!request) {
        return null;
    }

    if (request.mods.length === 0 || request.remembered) {
        return [];
    }

    const approval = await askApproval(request.mods);

    if (!approval) {
        return null;
    }

    const ids = request.mods.map((native) => native.id);

    if (approval.remember) {
        await commands.rememberNativeApproval(ids);
        log("Native code approved for this load order; not asking again until a native mod is added or removed", "ok");
    }

    return ids;
}

function askApproval(mods: NativeMod[]): Promise<{ remember: boolean } | null> {
    const dialog = byId<HTMLDialogElement>("nativeDlg");
    const remember = byId<HTMLInputElement>("nativeRemember");

    byId("nativeList").innerHTML = mods.map(nativeModHtml).join("");
    remember.checked = false;
    dialog.returnValue = "";

    return new Promise((resolve) => {
        dialog.addEventListener(
            "close",
            () => resolve(dialog.returnValue === "ok" ? { remember: remember.checked } : null),
            { once: true },
        );

        dialog.showModal();
        byId("nativeOk").focus();
    });
}

function nativeModHtml(native: NativeMod): string {
    const files = native.files.map(escapeHtml).join(", ");

    return `<li><b>${escapeHtml(native.name)}</b><span class="files mono">${files}</span></li>`;
}
