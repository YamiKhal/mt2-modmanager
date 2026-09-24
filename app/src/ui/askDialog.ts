import { byId } from "../util/dom";

interface ConfirmOptions {
    title: string;
    text: string;
    okLabel?: string;
}

interface PromptOptions extends ConfirmOptions {
    initialValue: string;
}


export async function confirmDialog(options: ConfirmOptions): Promise<boolean> {
    const answer = await openAskDialog(options, null);

    return answer !== null;
}

export function promptDialog(options: PromptOptions): Promise<string | null> {
    return openAskDialog(options, options.initialValue);
}

function openAskDialog(options: ConfirmOptions, initialValue: string | null): Promise<string | null> {
    const dialog = byId<HTMLDialogElement>("ask");
    const text = byId("askText");
    const input = byId<HTMLInputElement>("askInput");
    const hasInput = initialValue !== null;

    byId("askTitle").textContent = options.title;
    text.textContent = options.text;
    text.style.whiteSpace = "pre-wrap";
    byId("askOk").textContent = options.okLabel ?? "OK";

    input.hidden = !hasInput;
    input.value = initialValue ?? "";
    dialog.returnValue = "";

    return new Promise((resolve) => {
        dialog.addEventListener(
            "close",
            () => {
                const confirmed = dialog.returnValue === "ok";

                resolve(confirmed ? input.value.trim() : null);
            },
            { once: true },
        );

        dialog.showModal();

        if (hasInput) {
            input.focus();
            input.select();
        }
    });
}
