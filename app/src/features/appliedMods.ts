import type { LibraryMod, Manifest } from "../api/types";
import { appState, appliedModIds, appliedMods } from "../app/state";
import { byId } from "../util/dom";
import { escapeHtml } from "../util/text";
import { enableRowDragging } from "./appliedModsDrag";
import { saveLoadOrder, setModEnabled } from "./modActions";
import { renderModDetails } from "./modDetails";


export function renderAppliedMods(): void {
    const list = byId<HTMLOListElement>("appliedList");
    const mods = appliedMods();

    list.innerHTML = "";
    byId("appliedEmpty").hidden = mods.length > 0;

    mods.forEach((mod, index) => {
        if (mod.manifest) {
            list.append(createRow(mod, mod.manifest, index));
        }
    });

    updateSideButtons();
}

function createRow(mod: LibraryMod, manifest: Manifest, index: number): HTMLLIElement {
    const row = document.createElement("li");

    row.className = "row";
    row.classList.toggle("disabled", !mod.enabled);
    row.classList.toggle("sel", appState.appliedSelection === manifest.id);
    row.dataset.id = manifest.id;
    row.title = "Drag to change the load order";
    row.innerHTML = `
        <input type="checkbox" class="check" ${mod.enabled ? "checked" : ""} aria-label="Use ${escapeHtml(manifest.name)}">
        <span class="ridx">${index + 1}</span>
        <div class="rtitle"><b>${escapeHtml(manifest.name)}</b></div>
        <span class="rver">${escapeHtml(manifest.version)}</span>`;

    const checkbox = row.querySelector("input")!;

    // A click on the check box shouldn't also select the row.
    checkbox.addEventListener("click", (event) => event.stopPropagation());
    checkbox.addEventListener("change", () => {
        void setModEnabled(manifest.id, manifest.name, checkbox.checked);
    });

    row.addEventListener("click", () => {
        appState.appliedSelection = manifest.id;
        appState.detailsModKey = manifest.id;
        renderAppliedMods();
        renderModDetails();
    });

    enableRowDragging(row, moveMod);

    return row;
}


function moveMod(draggedId: string, targetId: string, placeAfter: boolean): void {
    const order = appliedModIds().filter((id) => id !== draggedId);
    const targetIndex = order.indexOf(targetId);

    order.splice(targetIndex + (placeAfter ? 1 : 0), 0, draggedId);
    void saveLoadOrder(order);
}


function updateSideButtons(): void {
    const ids = appliedModIds();
    const selectedIndex = ids.indexOf(appState.appliedSelection ?? "");

    byId<HTMLButtonElement>("btnMoveUp").disabled = selectedIndex <= 0;
    byId<HTMLButtonElement>("btnMoveDown").disabled = selectedIndex < 0 || selectedIndex >= ids.length - 1;
    byId<HTMLButtonElement>("btnUnapply").disabled = selectedIndex < 0;
}
