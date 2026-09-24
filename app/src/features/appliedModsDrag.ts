import { queryAll } from "../util/dom";

type DropHandler = (draggedId: string, targetId: string, placeAfter: boolean) => void;

const DRAG_THRESHOLD_PX = 5;
const ROW_SELECTOR = "#appliedList .row";


export function enableRowDragging(row: HTMLElement, onDrop: DropHandler): void {
    // Pointer events instead of HTML5 drag and drop: the Windows webview disables that while
    // file drops onto the window are enabled.
    row.addEventListener("pointerdown", (down) => {
        const pressedCheckbox = (down.target as Element).closest("input") !== null;

        if (down.button !== 0 || pressedCheckbox) {
            return;
        }

        trackDrag(row, down, onDrop);
    });
}

function trackDrag(row: HTMLElement, down: PointerEvent, onDrop: DropHandler): void {
    let dragging = false;
    let target: HTMLElement | null = null;
    let placeAfter = false;

    const onMove = (move: PointerEvent): void => {
        if (!dragging) {
            if (Math.abs(move.clientY - down.clientY) < DRAG_THRESHOLD_PX) {
                return;
            }

            dragging = true;
            row.setPointerCapture(down.pointerId);
            row.classList.add("dragging");
        }

        clearDropMarkers();

        const under = document.elementFromPoint(move.clientX, move.clientY)?.closest<HTMLElement>(ROW_SELECTOR);

        target = under && under !== row ? under : null;

        if (target) {
            const bounds = target.getBoundingClientRect();

            placeAfter = move.clientY > bounds.top + bounds.height / 2;
            target.classList.add(placeAfter ? "dropafter" : "dropbefore");
        }
    };

    const onEnd = (): void => {
        row.removeEventListener("pointermove", onMove);
        row.removeEventListener("pointerup", onEnd);
        row.removeEventListener("pointercancel", onEnd);
        row.classList.remove("dragging");
        clearDropMarkers();

        const draggedId = row.dataset.id;
        const targetId = target?.dataset.id;

        if (dragging && draggedId && targetId) {
            onDrop(draggedId, targetId, placeAfter);
        }
    };

    row.addEventListener("pointermove", onMove);
    row.addEventListener("pointerup", onEnd);
    row.addEventListener("pointercancel", onEnd);
}

function clearDropMarkers(): void {
    for (const row of queryAll(ROW_SELECTOR)) {
        row.classList.remove("dropbefore", "dropafter");
    }
}
