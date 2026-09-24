export function byId<T extends HTMLElement = HTMLElement>(id: string): T {
    const element = document.getElementById(id);

    if (!element) {
        throw new Error(`index.html has no element with id "${id}"`);
    }

    return element as T;
}

export function query<T extends Element = HTMLElement>(
    selector: string,
    root: ParentNode = document,
): T | null {
    return root.querySelector<T>(selector);
}

export function queryAll<T extends Element = HTMLElement>(
    selector: string,
    root: ParentNode = document,
): T[] {
    return [...root.querySelectorAll<T>(selector)];
}
