const KEY = "lcd_connection";

export function loadConnection() {
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function saveConnection(conn) {
  localStorage.setItem(KEY, JSON.stringify(conn));
}

export function clearConnection() {
  localStorage.removeItem(KEY);
}
