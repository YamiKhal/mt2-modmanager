import { query, queryAll } from "../util/dom";


export function closeMenus(): void {
    for (const menu of queryAll(".menu.open")) {
        menu.classList.remove("open");
    }
}

function openMenu(menu: HTMLElement): void {
    closeMenus();
    menu.classList.add("open");
}


export function wireMenus(): void {
    for (const title of queryAll(".menutitle")) {
        const menu = title.parentElement!;

        title.addEventListener("click", () => {
            const wasOpen = menu.classList.contains("open");

            closeMenus();

            if (!wasOpen) {
                menu.classList.add("open");
            }
        });

        title.addEventListener("mouseenter", () => {
            const anotherMenuIsOpen = query(".menu.open") !== null && !menu.classList.contains("open");

            if (anotherMenuIsOpen) {
                openMenu(menu);
            }
        });
    }

    document.addEventListener("keydown", (event) => {
        if (event.key === "Escape") {
            closeMenus();
        }
    });
}
