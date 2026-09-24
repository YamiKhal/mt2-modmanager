const HTML_ESCAPES: Record<string, string> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
};


export function escapeHtml(value: unknown): string {
    return String(value ?? "").replace(/[&<>"']/g, (char) => HTML_ESCAPES[char] ?? char);
}

export function baseName(path: string): string {
    const parts = path.split(/[\\/]/).filter(Boolean);

    return parts.at(-1) ?? path;
}

export function initials(name: string): string {
    const words = name
        .replace(/[^A-Za-z0-9 ]+/g, " ")
        .trim()
        .split(/\s+/)
        .filter(Boolean);

    const first = words[0]?.[0] ?? "?";
    const second = words[1]?.[0] ?? words[0]?.[1] ?? "";

    return (first + second).toUpperCase();
}

export function countOf(count: number, singular: string, plural = singular + "s"): string {
    return `${count} ${count === 1 ? singular : plural}`;
}

export function isWebLink(value: string | undefined): value is string {
    return /^(https?:\/\/|mailto:)\S+$/i.test((value ?? "").trim());
}
