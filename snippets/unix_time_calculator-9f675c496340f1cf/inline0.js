
export function write_to_clipboard(text) {
    if (navigator.clipboard) { navigator.clipboard.writeText(text).catch(() => {}); return true; }
    return false;
}
