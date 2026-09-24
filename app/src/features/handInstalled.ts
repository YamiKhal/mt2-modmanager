import * as commands from "../api/commands";
import { refresh } from "../app/refresh";
import { currentStatus } from "../app/state";
import { runAction } from "../ui/busy";
import { log } from "../ui/log";
import { toast } from "../ui/toast";
import { byId } from "../util/dom";
import { baseName, escapeHtml } from "../util/text";


export function showHandInstalled(): void {
    const list = byId<HTMLUListElement>("handList");
    const dialog = byId<HTMLDialogElement>("handDlg");

    list.innerHTML = "";

    for (const path of currentStatus().unmanaged) {
        list.append(createRow(path, dialog));
    }

    if (!dialog.open) {
        dialog.showModal();
    }
}

function createRow(path: string, dialog: HTMLDialogElement): HTMLLIElement {
    const name = baseName(path);
    const row = document.createElement("li");

    row.innerHTML = `<span class="mono">${escapeHtml(name)}</span><button class="btn small">Move into manager</button>`;

    row.querySelector("button")!.addEventListener("click", () => {
        void runAction("Move", async () => {
            const result = await commands.adoptMod(path);

            log(`Moved ${name} into the manager as ${result.id}`, "ok");
            toast(`Moved “${name}” into the manager. Launch to apply.`);
            await refresh();

            if (currentStatus().unmanaged.length > 0) {
                showHandInstalled();
            } else {
                dialog.close();
            }
        });
    });

    return row;
}


export function wireHandInstalled(): void {
    byId("handInstalled").addEventListener("click", showHandInstalled);
}
