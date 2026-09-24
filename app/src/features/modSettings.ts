import * as commands from "../api/commands";
import type { ConfigOption, LibraryMod, ResolvedSettings, SettingValue } from "../api/types";
import { refresh } from "../app/refresh";
import { runAction } from "../ui/busy";
import { toast } from "../ui/toast";
import { query, queryAll } from "../util/dom";
import { escapeHtml } from "../util/text";
import { controlHtml, defaultValueText, formatNumber } from "./settingControls";

const RESET_ICON = `<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 3a5 5 0 1 1-4.9 6h1.6A3.5 3.5 0 1 0 8 4.5V7L4.5 3.75 8 .5z"/></svg>`;


export function settingsSectionHtml(mod: LibraryMod): string {
    if (mod.config_error) {
        return `
            <section class="settings">
                <h4>Settings</h4>
                <p class="setnote bad">Settings unavailable: ${escapeHtml(mod.config_error)}</p>
            </section>`;
    }

    if (mod.config.length === 0) {
        return "";
    }

    const settings = mod.settings;
    const resetAllDisabled = settings.changed.length > 0 ? "" : "disabled";

    return `
        <section class="settings">
            <div class="sethead">
                <h4>Settings</h4>
                <button class="btn small" data-set="resetall" ${resetAllDisabled}>Reset all</button>
            </div>
            ${updateNotesHtml(settings)}
            <div class="setlist">${rowsHtml(mod.config, settings)}</div>
        </section>`;
}

function updateNotesHtml(settings: ResolvedSettings): string {
    if (settings.notes.length === 0) {
        return "";
    }

    const version = settings.saved_for ? "version " + escapeHtml(settings.saved_for) : "an older version";
    const notes = settings.notes.map((note) => `<li>${escapeHtml(note)}</li>`).join("");

    return `
        <div class="setnote">
            <p>Your settings were saved for ${version} of this mod. Some didn't fit this version:</p>
            <ul>${notes}</ul>
            <button class="btn small" data-set="accept">OK</button>
        </div>`;
}

function rowsHtml(options: ConfigOption[], settings: ResolvedSettings): string {
    const changed = new Set(settings.changed);
    let currentGroup = "";

    return options
        .map((option) => {
            const startsGroup = option.group && option.group !== currentGroup;
            const heading = startsGroup ? `<h5>${escapeHtml(option.group)}</h5>` : "";

            if (option.group) {
                currentGroup = option.group;
            }

            return heading + rowHtml(option, settings.values[option.key], changed.has(option.key));
        })
        .join("");
}

function rowHtml(option: ConfigOption, value: SettingValue, isChanged: boolean): string {
    const key = escapeHtml(option.key);
    const label = escapeHtml(option.label);

    const labelClass = option.description ? "setlabel info" : "setlabel";
    const labelTitle = option.description ? ` title="${escapeHtml(option.description)}"` : "";

    const resetTitle = `Back to the default (${escapeHtml(defaultValueText(option))})`;

    return `
        <div class="setrow${isChanged ? " changed" : ""}">
            <label class="${labelClass}" for="set-${key}"${labelTitle}>${label}</label>
            <div class="setctl">${controlHtml(option, value)}</div>
            <button class="resetone" data-reset="${key}" title="${resetTitle}" aria-label="Reset ${label}" ${isChanged ? "" : "hidden"}>
                ${RESET_ICON}
            </button>
        </div>`;
}


export function wireSettings(box: HTMLElement, mod: LibraryMod): void {
    if (!mod.manifest || mod.config.length === 0 || mod.config_error) {
        return;
    }

    const modId = mod.manifest.id;
    const optionsByKey = new Map(mod.config.map((option) => [option.key, option]));

    linkLabelsToControls(box);

    for (const control of queryAll<HTMLInputElement | HTMLSelectElement>("[data-key]", box)) {
        const option = optionsByKey.get(control.dataset.key ?? "");

        if (option) {
            wireControl(control, option, modId);
        }
    }

    for (const button of queryAll("[data-reset]", box)) {
        button.addEventListener("click", () => void saveSettings(modId, {}, [button.dataset.reset ?? ""]));
    }

    query('[data-set="resetall"]', box)?.addEventListener("click", () => {
        void saveSettings(modId, {}, mod.config.map((option) => option.key));
    });

    // Saving nothing stores the values again for this version, which clears the notes.
    query('[data-set="accept"]', box)?.addEventListener("click", () => void saveSettings(modId, {}, []));
}

function linkLabelsToControls(box: HTMLElement): void {
    for (const row of queryAll(".setrow", box)) {
        const firstControl = query("[data-key]", row);

        if (firstControl) {
            firstControl.id = "set-" + firstControl.dataset.key;
        }

        const description = query(".setlabel", row)?.title;

        if (description) {
            for (const control of queryAll("[data-key]", row)) {
                control.setAttribute("aria-description", description);
            }
        }
    }
}

function wireControl(
    control: HTMLInputElement | HTMLSelectElement,
    option: ConfigOption,
    modId: string,
): void {
    if (control instanceof HTMLInputElement && control.type === "range") {
        const numberField = control.parentElement?.querySelector<HTMLInputElement>("input.num");

        control.addEventListener("input", () => {
            if (numberField) {
                numberField.value = formatNumber(option, control.value);
            }
        });
    }

    control.addEventListener("change", () => {
        const value = readControl(control, option);

        if (value === null) {
            toast(`${option.label}: enter a number`, true);
            void refresh();

            return;
        }

        void saveSettings(modId, { [option.key]: value }, []);
    });
}

function readControl(control: HTMLInputElement | HTMLSelectElement, option: ConfigOption): SettingValue | null {
    if (option.type === "bool") {
        return (control as HTMLInputElement).checked;
    }

    if (option.type === "int" || option.type === "float") {
        const text = control.value.trim();
        const number = Number(text);

        if (text === "" || Number.isNaN(number)) {
            return null;
        }

        return option.type === "int" ? Math.round(number) : number;
    }

    return control.value;
}


async function saveSettings(
    modId: string,
    changes: Record<string, SettingValue>,
    reset: string[],
): Promise<void> {
    const focusSelector = focusedControlSelector();

    await runAction("Settings", async () => {
        await commands.setModSettings(modId, changes, reset);
        await refresh();

        if (focusSelector) {
            query(focusSelector)?.focus();
        }
    });
}

function focusedControlSelector(): string | null {
    const focused = document.activeElement;

    if (!(focused instanceof HTMLElement) || !focused.closest(".setrow")) {
        return null;
    }

    const key = CSS.escape(focused.dataset.key ?? "");
    let kind = "";

    if (focused instanceof HTMLInputElement && focused.type === "range") {
        kind = '[type="range"]';
    } else if (focused.classList.contains("num")) {
        kind = ".num";
    }

    return `#tab-desc [data-key="${key}"]${kind}`;
}
