import type { ConfigOption, SettingValue } from "../api/types";
import { escapeHtml } from "../util/text";


export function formatNumber(option: ConfigOption, value: SettingValue): string {
    const number = Number(value);

    if (option.type !== "float") {
        return String(Math.round(number));
    }

    if (!option.step) {
        return String(number);
    }

    const decimals = (String(option.step).split(".")[1] ?? "").length;

    return number.toFixed(decimals);
}

export function defaultValueText(option: ConfigOption): string {
    switch (option.type) {
        case "bool":
            return option.default ? "enabled" : "disabled";

        case "choice": {
            const choice = option.options?.find((candidate) => candidate.value === String(option.default));

            return choice?.label ?? String(option.default);
        }

        case "string":
            return option.default === "" ? "empty" : `“${option.default}”`;

        default:
            return formatNumber(option, option.default) + (option.unit ? " " + option.unit : "");
    }
}


export function controlHtml(option: ConfigOption, value: SettingValue): string {
    const key = escapeHtml(option.key);

    switch (option.type) {
        case "bool":
            return `<input type="checkbox" class="check" data-key="${key}" ${value ? "checked" : ""}>`;

        case "choice":
            return choiceHtml(option, key, value);

        case "string":
            return `<input type="text" data-key="${key}" value="${escapeHtml(value)}" spellcheck="false">`;

        default:
            return numberHtml(option, key, value);
    }
}

function choiceHtml(option: ConfigOption, key: string, value: SettingValue): string {
    const choices = (option.options ?? [])
        .map((choice) => {
            const selected = String(value) === choice.value ? "selected" : "";

            return `<option value="${escapeHtml(choice.value)}" ${selected}>${escapeHtml(choice.label)}</option>`;
        })
        .join("");

    return `<select data-key="${key}">${choices}</select>`;
}

function numberHtml(option: ConfigOption, key: string, value: SettingValue): string {
    const limits = limitAttributes(option);
    const shown = escapeHtml(formatNumber(option, value));
    const unit = option.unit ? `<span class="unit">${escapeHtml(option.unit)}</span>` : "";

    if (option.input === "slider") {
        return `
            <input type="range" data-key="${key}"${limits} value="${escapeHtml(value)}">
            <input type="number" class="num" data-key="${key}"${limits} value="${shown}" aria-label="${escapeHtml(option.label)}">${unit}`;
    }

    return `<input type="number" class="num wide" data-key="${key}"${limits} value="${shown}">${unit}`;
}

function limitAttributes(option: ConfigOption): string {
    const min = option.min != null ? ` min="${option.min}"` : "";
    const max = option.max != null ? ` max="${option.max}"` : "";
    const step = option.step ?? (option.type === "int" ? 1 : "any");

    return `${min}${max} step="${step}"`;
}
