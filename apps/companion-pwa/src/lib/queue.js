const KEY = "lcd_msg_queue";

export function enqueue(conversationId, content) {
  const q = getAll();
  q.push({ id: crypto.randomUUID(), conversationId, content, queuedAt: Date.now() });
  localStorage.setItem(KEY, JSON.stringify(q));
}

export function getAll() {
  try {
    return JSON.parse(localStorage.getItem(KEY) || "[]");
  } catch {
    return [];
  }
}

export function remove(id) {
  const q = getAll().filter((item) => item.id !== id);
  localStorage.setItem(KEY, JSON.stringify(q));
}

export function countForConversation(conversationId) {
  return getAll().filter((item) => item.conversationId === conversationId).length;
}
