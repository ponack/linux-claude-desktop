export function makeApi(conn) {
  const base = conn.url.replace(/\/$/, "");
  const headers = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${conn.token}`,
  };

  async function request(method, path, body) {
    const resp = await fetch(`${base}${path}`, {
      method,
      headers,
      body: body != null ? JSON.stringify(body) : undefined,
    });
    if (!resp.ok) {
      const text = await resp.text().catch(() => "");
      let msg = `HTTP ${resp.status}`;
      try {
        msg = JSON.parse(text).error || msg;
      } catch {}
      throw new Error(msg);
    }
    return resp.json();
  }

  return {
    listConversations: () => request("GET", "/api/conversations"),
    getConversation: (id) => request("GET", `/api/conversations/${id}`),
    createConversation: (title) => request("POST", "/api/conversations", { title }),
    sendMessage: (id, content) =>
      request("POST", `/api/conversations/${id}/messages`, { content }),
  };
}
