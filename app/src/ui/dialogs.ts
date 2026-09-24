import { queryAll } from "../util/dom";


export function wireDialogCloseButtons(): void {
    for (const button of queryAll("dialog [data-close]")) {
        button.addEventListener("click", () => button.closest("dialog")?.close());
    }
}
