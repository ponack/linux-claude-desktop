<script>
  import { invoke } from "@tauri-apps/api/core";
  import { save as saveDialog, open as openDialog } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";

  let { onClose } = $props();

  // Navigation
  let activeSection = $state("general");
  const sections = [
    { id: "general", label: "General", icon: "M12 15a3 3 0 100-6 3 3 0 000 6z M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z" },
    { id: "appearance", label: "Appearance", icon: "M12 2.69l5.66 5.66a8 8 0 11-11.31 0z" },
    { id: "prompts", label: "Prompts", icon: "M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z" },
    { id: "projects", label: "Projects", icon: "M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" },
    { id: "integrations", label: "Integrations", icon: "M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z" },
    { id: "schedules", label: "Schedules", icon: "M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z M12 6v6l4 2" },
    { id: "endpoints", label: "Endpoints", icon: "M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.66 0 3-4.03 3-9s-1.34-9-3-9m0 18c-1.66 0-3-4.03-3-9s1.34-9 3-9" },
    { id: "routing", label: "Routing", icon: "M16 3h5v5 M4 20L21 3 M21 16v5h-5 M15 15l6 6 M4 4l5 5" },
    { id: "knowledge", label: "Knowledge", icon: "M2 3h6a4 4 0 014 4v14a3 3 0 00-3-3H2z M22 3h-6a4 4 0 00-4 4v14a3 3 0 013-3h7z" },
    { id: "data", label: "Data & Usage", icon: "M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4 M12 3v12 M8 11l4 4 4-4" },
    { id: "about", label: "About", icon: "M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z M12 16v-4 M12 8h.01" },
  ];

  // Provider settings
  let provider = $state("anthropic");
  let apiKey = $state("");
  let openaiApiKey = $state("");
  let openaiBaseUrl = $state("https://api.openai.com");
  let ollamaBaseUrl = $state("http://localhost:11434");
  let model = $state("claude-sonnet-4-6");
  let ollamaModels = $state([]);
  let fetchingOllamaModels = $state(false);

  // General settings
  let systemPrompt = $state("");
  let theme = $state("dark");
  let customCss = $state("");
  let updateInterval = $state("86400000");

  // Status
  let saveStatus = $state(""); // "", "saving", "saved", "error"
  let saveError = $state("");

  // Projects
  let projects = $state([]);
  let newProjectName = $state("");
  let newProjectContext = $state("");
  let editingProject = $state(null);
  let newProjectProvider = $state("");
  let newProjectApiKey = $state("");
  let newProjectModel = $state("");
  let newProjectSystemPrompt = $state("");

  // MCP Servers
  let mcpServers = $state([]);
  let newMcpName = $state("");
  let newMcpCommand = $state("");
  let newMcpArgs = $state("");

  // Prompt Library
  let prompts = $state([]);
  let newPromptName = $state("");
  let newPromptContent = $state("");
  let editingPrompt = $state(null);

  // Custom Commands
  let customCommands = $state([]);
  let newCmdName = $state("");
  let newCmdCommand = $state("");
  let newCmdDescription = $state("");

  // Scheduled Prompts
  let scheduledPrompts = $state([]);
  let newSchedName = $state("");
  let newSchedPrompt = $state("");
  let newSchedInterval = $state("3600000");
  let editingSched = $state(null);

  // Memory
  let memoryEntries = $state([]);
  let newMemKey = $state("");
  let newMemValue = $state("");

  // Knowledge Base
  let knowledgeEntries = $state([]);
  let newKbTitle = $state("");
  let newKbContent = $state("");
  let newKbSourceType = $state("manual");
  let newKbUrl = $state("");
  let newKbProjectId = $state("");
  let newKbWatch = $state(false);
  let importingKbUrl = $state(false);
  let editingKnowledge = $state(null);

  // File Watches
  let fileWatches = $state([]);

  // Custom Endpoints
  let customEndpoints = $state([]);
  let newEpName = $state("");
  let newEpBaseUrl = $state("");
  let newEpApiKey = $state("");
  let newEpFormat = $state("openai");
  let newEpModel = $state("");
  let editingEndpoint = $state(null);
  let testingEndpoint = $state(false);
  let testResult = $state("");

  // Model Pricing
  let modelPricing = $state([]);

  // Routing Rules
  let routingRules = $state([]);
  let newRuleName = $state("");
  let newRulePattern = $state("");
  let newRuleTaskType = $state("custom");
  let newRuleProvider = $state("anthropic");
  let newRuleModel = $state("");
  let newRulePriority = $state(0);
  let editingRule = $state(null);

  // Cost Summary
  let costSummary = $state([]);

  // Token Usage Analytics
  let totalUsage = $state(null);

  // Database
  let dbPath = $state("");
  let dbSize = $state(0);
  let dbStatus = $state("");
  let dbError = $state("");

  // About
  let appVersion = $state("");
  let appArch = $state("");
  let appOs = $state("");

  const anthropicModels = [
    { id: "claude-opus-4-6", name: "Claude Opus 4.6 (Most capable)" },
    { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6 (Balanced)" },
    { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5 (Fast)" },
  ];

  const openaiModels = [
    { id: "gpt-4o", name: "GPT-4o (Most capable)" },
    { id: "gpt-4o-mini", name: "GPT-4o Mini (Fast)" },
    { id: "o1", name: "o1 (Reasoning)" },
    { id: "o3-mini", name: "o3-mini (Reasoning, fast)" },
  ];

  let customEndpointModels = $derived(
    customEndpoints.filter(ep => ep.is_enabled && ep.default_model).map(ep => ({ id: ep.default_model, name: `${ep.name}: ${ep.default_model}` }))
  );

  let availableModels = $derived(
    provider === "anthropic" ? anthropicModels :
    provider === "openai" ? openaiModels :
    provider === "custom" ? customEndpointModels :
    ollamaModels.map(m => ({ id: m, name: m }))
  );

  onMount(async () => {
    try {
      provider = await invoke("get_provider");
      const key = await invoke("get_api_key");
      if (key) apiKey = key;
      const oaiKey = await invoke("get_openai_api_key");
      if (oaiKey) openaiApiKey = oaiKey;
      openaiBaseUrl = await invoke("get_openai_base_url");
      ollamaBaseUrl = await invoke("get_ollama_base_url");
      model = await invoke("get_model");
      const sp = await invoke("get_system_prompt");
      if (sp) systemPrompt = sp;
      theme = await invoke("get_theme");
      customCss = await invoke("get_custom_css");
      updateInterval = await invoke("get_update_interval");
      await loadProjects();
      await loadMcpServers();
      await loadPrompts();
      await loadCustomCommands();
      await loadScheduledPrompts();
      await loadMemoryEntries();
      await loadKnowledgeEntries();
      await loadFileWatches();
      await loadCustomEndpoints();
      await loadRoutingRules();
      await loadModelPricing();
      await loadCostSummary();

      try {
        const info = await invoke("get_app_info");
        appVersion = info.version;
        appArch = info.arch;
        appOs = info.os;
      } catch (_) {}

      try { totalUsage = await invoke("get_total_usage"); } catch (_) {}
      try {
        dbPath = await invoke("get_database_path");
        dbSize = await invoke("get_database_size");
      } catch (_) {}

      if (provider === "ollama") fetchOllamaModels();
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  });

  // Auto-save helper with debounce
  let saveTimer;
  async function autoSave(fn) {
    clearTimeout(saveTimer);
    saveStatus = "saving";
    saveError = "";
    try {
      await fn();
      saveStatus = "saved";
      saveTimer = setTimeout(() => { if (saveStatus === "saved") saveStatus = ""; }, 1500);
    } catch (e) {
      saveError = String(e);
      saveStatus = "error";
    }
  }

  // Auto-save individual settings on change
  async function saveProvider() {
    await autoSave(async () => {
      await invoke("set_provider", { provider });
      if (provider === "ollama") fetchOllamaModels();
      else if (provider === "anthropic" && !anthropicModels.find(m => m.id === model)) {
        model = "claude-sonnet-4-6"; await invoke("set_model", { model });
      } else if (provider === "openai" && !openaiModels.find(m => m.id === model)) {
        model = "gpt-4o"; await invoke("set_model", { model });
      } else if (provider === "custom") {
        const enabled = customEndpoints.filter(e => e.is_enabled);
        if (enabled.length > 0) {
          await invoke("set_custom_endpoint_id", { id: enabled[0].id });
          if (enabled[0].default_model) { model = enabled[0].default_model; await invoke("set_model", { model }); }
        }
      }
    });
  }

  async function saveApiKey() { await autoSave(() => invoke("set_api_key", { key: apiKey })); }
  async function saveOpenaiApiKey() { await autoSave(() => invoke("set_openai_api_key", { key: openaiApiKey })); }
  async function saveOpenaiBaseUrl() { await autoSave(() => invoke("set_openai_base_url", { url: openaiBaseUrl })); }
  async function saveOllamaBaseUrl() { await autoSave(() => invoke("set_ollama_base_url", { url: ollamaBaseUrl })); }
  async function saveCustomEndpointId(id) { await autoSave(() => invoke("set_custom_endpoint_id", { id })); }
  async function saveModel() { await autoSave(() => invoke("set_model", { model })); }
  async function saveSystemPrompt() { await autoSave(() => invoke("set_system_prompt", { prompt: systemPrompt })); }
  async function saveUpdateInterval() { await autoSave(() => invoke("set_update_interval", { interval: updateInterval })); }

  async function saveTheme() {
    await autoSave(async () => {
      await invoke("set_theme", { theme });
      document.documentElement.setAttribute("data-theme", theme);
    });
  }

  async function saveCustomCss() {
    await autoSave(async () => {
      await invoke("set_custom_css", { css: customCss });
      let styleEl = document.getElementById("custom-css");
      if (!styleEl) {
        styleEl = document.createElement("style");
        styleEl.id = "custom-css";
        document.head.appendChild(styleEl);
      }
      styleEl.textContent = customCss;
    });
  }

  function formatBytes(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1048576).toFixed(1)} MB`;
  }

  async function backupDatabase() {
    dbStatus = ""; dbError = "";
    try {
      const dest = await saveDialog({
        defaultPath: `ucd-backup-${new Date().toISOString().slice(0, 10)}.db`,
        filters: [{ name: "SQLite Database", extensions: ["db"] }],
      });
      if (!dest) return;
      dbStatus = "backing-up";
      await invoke("backup_database", { destination: dest });
      dbStatus = "backed-up";
      setTimeout(() => { if (dbStatus === "backed-up") dbStatus = ""; }, 3000);
    } catch (e) { dbError = String(e); dbStatus = "error"; }
  }

  async function restoreDatabase() {
    dbStatus = ""; dbError = "";
    try {
      const source = await openDialog({
        filters: [{ name: "SQLite Database", extensions: ["db"] }],
        multiple: false,
      });
      if (!source) return;
      const path = typeof source === "string" ? source : source.path;
      dbStatus = "restoring";
      await invoke("restore_database", { source: path });
      dbStatus = "restored";
    } catch (e) { dbError = String(e); dbStatus = "error"; }
  }

  async function fetchOllamaModels() {
    fetchingOllamaModels = true;
    try {
      ollamaModels = await invoke("fetch_ollama_models", { baseUrl: ollamaBaseUrl });
    } catch (e) { ollamaModels = []; }
    fetchingOllamaModels = false;
  }

  // --- MCP ---
  async function loadMcpServers() {
    try { mcpServers = await invoke("get_mcp_servers"); } catch (e) { mcpServers = []; }
  }
  async function addMcpServer() {
    if (!newMcpName.trim() || !newMcpCommand.trim()) return;
    mcpServers = [...mcpServers, { name: newMcpName.trim(), command: newMcpCommand.trim(), args: newMcpArgs.trim() ? newMcpArgs.trim().split(/\s+/) : [], env: {} }];
    await invoke("set_mcp_servers", { servers: mcpServers });
    newMcpName = ""; newMcpCommand = ""; newMcpArgs = "";
  }
  async function removeMcpServer(index) {
    mcpServers = mcpServers.filter((_, i) => i !== index);
    await invoke("set_mcp_servers", { servers: mcpServers });
  }

  // --- Projects ---
  async function loadProjects() { try { projects = await invoke("get_projects"); } catch (e) {} }
  async function addProject() {
    if (!newProjectName.trim()) return;
    await invoke("create_project", { name: newProjectName.trim(), context: newProjectContext.trim(), provider: newProjectProvider || null, apiKey: newProjectApiKey || null, model: newProjectModel || null, systemPrompt: newProjectSystemPrompt || null });
    newProjectName = ""; newProjectContext = ""; newProjectProvider = ""; newProjectApiKey = ""; newProjectModel = ""; newProjectSystemPrompt = "";
    await loadProjects();
  }
  async function saveProject(project) {
    await invoke("update_project", { id: project.id, name: project.name, context: project.context, provider: project.provider || null, apiKey: project.api_key || null, model: project.model || null, systemPrompt: project.system_prompt || null });
    editingProject = null; await loadProjects();
  }
  async function removeProject(id) { await invoke("delete_project", { id }); await loadProjects(); }

  // --- Prompts ---
  async function loadPrompts() { try { prompts = await invoke("get_prompts"); } catch (e) {} }
  async function addPrompt() {
    if (!newPromptName.trim() || !newPromptContent.trim()) return;
    await invoke("create_prompt", { name: newPromptName.trim(), content: newPromptContent.trim() });
    newPromptName = ""; newPromptContent = ""; await loadPrompts();
  }
  async function savePrompt(prompt) {
    await invoke("update_prompt", { id: prompt.id, name: prompt.name, content: prompt.content });
    editingPrompt = null; await loadPrompts();
  }
  async function removePrompt(id) { await invoke("delete_prompt", { id }); await loadPrompts(); }

  // --- Custom Commands ---
  async function loadCustomCommands() { try { customCommands = await invoke("get_custom_commands"); } catch (e) { customCommands = []; } }
  async function addCustomCommand() {
    if (!newCmdName.trim() || !newCmdCommand.trim()) return;
    customCommands = [...customCommands, { name: newCmdName.trim(), command: newCmdCommand.trim(), description: newCmdDescription.trim() }];
    await invoke("set_custom_commands", { commands: customCommands });
    newCmdName = ""; newCmdCommand = ""; newCmdDescription = "";
  }
  async function removeCustomCommand(index) {
    customCommands = customCommands.filter((_, i) => i !== index);
    await invoke("set_custom_commands", { commands: customCommands });
  }

  // --- Scheduled Prompts ---
  async function loadScheduledPrompts() { try { scheduledPrompts = await invoke("get_scheduled_prompts"); } catch (e) {} }
  async function addScheduledPrompt() {
    if (!newSchedName.trim() || !newSchedPrompt.trim()) return;
    await invoke("create_scheduled_prompt", { name: newSchedName.trim(), prompt: newSchedPrompt.trim(), intervalMs: parseInt(newSchedInterval) || 3600000 });
    newSchedName = ""; newSchedPrompt = ""; newSchedInterval = "3600000"; await loadScheduledPrompts();
  }
  async function saveScheduledPrompt(sp) {
    await invoke("update_scheduled_prompt", { id: sp.id, name: sp.name, prompt: sp.prompt, intervalMs: sp.interval_ms, enabled: sp.enabled });
    editingSched = null; await loadScheduledPrompts();
  }
  async function removeScheduledPrompt(id) { await invoke("delete_scheduled_prompt", { id }); await loadScheduledPrompts(); }

  // --- Memory ---
  async function loadMemoryEntries() { try { memoryEntries = await invoke("get_memory_entries"); } catch (e) {} }
  async function addMemoryEntry() {
    if (!newMemKey.trim() || !newMemValue.trim()) return;
    await invoke("save_memory_entry", { key: newMemKey.trim(), value: newMemValue.trim() });
    newMemKey = ""; newMemValue = "";
    await loadMemoryEntries();
  }
  async function removeMemoryEntry(id) {
    await invoke("delete_memory_entry", { id });
    await loadMemoryEntries();
  }

  // --- Knowledge Base ---
  async function loadKnowledgeEntries() { try { knowledgeEntries = await invoke("get_knowledge_entries", { projectId: null }); } catch (e) {} }
  async function addKnowledgeEntry() {
    if (!newKbTitle.trim()) return;
    if (newKbSourceType === "url") {
      if (!newKbUrl.trim()) return;
      importingKbUrl = true;
      try {
        const [title, content] = await invoke("import_url", { url: newKbUrl.trim() });
        await invoke("create_knowledge_entry", {
          title: newKbTitle.trim() || title,
          content,
          sourceType: "url",
          sourceUrl: newKbUrl.trim(),
          filePath: null,
          projectId: newKbProjectId || null,
        });
      } catch (e) {
        alert("Failed to import URL: " + e);
        importingKbUrl = false;
        return;
      }
      importingKbUrl = false;
    } else {
      if (!newKbContent.trim()) return;
      await invoke("create_knowledge_entry", {
        title: newKbTitle.trim(),
        content: newKbContent.trim(),
        sourceType: "manual",
        sourceUrl: null,
        filePath: null,
        projectId: newKbProjectId || null,
      });
    }
    newKbTitle = ""; newKbContent = ""; newKbUrl = ""; newKbSourceType = "manual"; newKbProjectId = ""; newKbWatch = false;
    await loadKnowledgeEntries();
  }
  async function importFileToKnowledge() {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: "Text Files", extensions: ["txt", "md", "json", "yaml", "yml", "toml", "csv", "xml", "html", "py", "js", "ts", "rs", "go", "java", "c", "cpp", "h", "sh"] }],
      });
      if (!selected) return;
      const path = typeof selected === "string" ? selected : selected.path;
      await invoke("import_file_to_knowledge", { path, projectId: newKbProjectId || null, watch: newKbWatch });
      await loadKnowledgeEntries();
      await loadFileWatches();
    } catch (e) {
      alert("Failed to import file: " + e);
    }
  }
  async function toggleKnowledgeEnabled(entry) {
    await invoke("toggle_knowledge_entry", { id: entry.id, enabled: !entry.enabled });
    await loadKnowledgeEntries();
  }
  async function saveKnowledgeEntry(entry) {
    await invoke("update_knowledge_entry", { id: entry.id, title: entry.title, content: entry.content, enabled: entry.enabled });
    editingKnowledge = null;
    await loadKnowledgeEntries();
  }
  async function removeKnowledgeEntry(id) {
    await invoke("delete_knowledge_entry", { id });
    await loadKnowledgeEntries();
    await loadFileWatches();
  }

  // --- File Watches ---
  async function loadFileWatches() { try { fileWatches = await invoke("get_file_watches"); } catch (e) {} }
  async function removeFileWatch(id) {
    await invoke("delete_file_watch", { id });
    await loadFileWatches();
  }

  // Custom Endpoints
  async function loadCustomEndpoints() { try { customEndpoints = await invoke("get_custom_endpoints"); } catch (e) {} }
  async function addEndpoint() {
    if (!newEpName.trim() || !newEpBaseUrl.trim()) return;
    await invoke("create_custom_endpoint", {
      name: newEpName, baseUrl: newEpBaseUrl, apiKey: newEpApiKey,
      apiFormat: newEpFormat, defaultModel: newEpModel,
    });
    newEpName = ""; newEpBaseUrl = ""; newEpApiKey = ""; newEpFormat = "openai"; newEpModel = "";
    await loadCustomEndpoints();
  }
  async function deleteEndpoint(id) {
    await invoke("delete_custom_endpoint", { id });
    await loadCustomEndpoints();
  }
  async function saveEndpoint(ep) {
    await invoke("update_custom_endpoint", {
      id: ep.id, name: ep.name, baseUrl: ep.base_url, apiKey: ep.api_key,
      apiFormat: ep.api_format, defaultModel: ep.default_model, isEnabled: ep.is_enabled,
    });
    editingEndpoint = null;
    await loadCustomEndpoints();
  }
  async function testEndpointConnection(baseUrl, apiKey, apiFormat) {
    testingEndpoint = true;
    testResult = "";
    try {
      testResult = await invoke("test_custom_endpoint", { baseUrl, apiKey, apiFormat });
    } catch (e) {
      testResult = `Failed: ${e}`;
    }
    testingEndpoint = false;
  }

  // Model Pricing
  async function loadModelPricing() { try { modelPricing = await invoke("get_model_pricing"); } catch (e) {} }

  // Routing Rules
  async function loadRoutingRules() { try { routingRules = await invoke("get_routing_rules"); } catch (e) {} }
  async function addRoutingRule() {
    if (!newRuleName.trim() || !newRulePattern.trim() || !newRuleModel.trim()) return;
    await invoke("create_routing_rule", {
      name: newRuleName, pattern: newRulePattern, taskType: newRuleTaskType,
      targetProvider: newRuleProvider, targetModel: newRuleModel, priority: newRulePriority,
    });
    newRuleName = ""; newRulePattern = ""; newRuleTaskType = "custom"; newRuleModel = ""; newRulePriority = 0;
    await loadRoutingRules();
  }
  async function deleteRoutingRule(id) {
    await invoke("delete_routing_rule", { id });
    await loadRoutingRules();
  }
  async function toggleRoutingRule(rule) {
    await invoke("update_routing_rule", {
      id: rule.id, name: rule.name, pattern: rule.pattern, taskType: rule.task_type,
      targetProvider: rule.target_provider, targetModel: rule.target_model,
      priority: rule.priority, isEnabled: !rule.is_enabled,
    });
    await loadRoutingRules();
  }

  // Cost Summary
  async function loadCostSummary() { try { costSummary = await invoke("get_cost_summary"); } catch (e) {} }

  function formatInterval(ms) {
    if (ms >= 86400000) return `${Math.round(ms / 86400000)}d`;
    if (ms >= 3600000) return `${Math.round(ms / 3600000)}h`;
    if (ms >= 60000) return `${Math.round(ms / 60000)}m`;
    return `${ms}ms`;
  }

  const themePresets = [
    { name: "None", css: "" },
    { name: "Nord", css: `:root, [data-theme="dark"] {\n  --bg-primary: #2e3440;\n  --bg-secondary: #3b4252;\n  --bg-tertiary: #434c5e;\n  --bg-input: #2e3440;\n  --text-primary: #eceff4;\n  --text-secondary: #d8dee9;\n  --text-muted: #7b88a1;\n  --accent: #88c0d0;\n  --accent-hover: #8fbcbb;\n  --border: #4c566a;\n  --user-bubble: #434c5e;\n  --assistant-bubble: #2e3440;\n  --danger: #bf616a;\n  --success: #a3be8c;\n}` },
    { name: "Solarized", css: `:root, [data-theme="dark"] {\n  --bg-primary: #002b36;\n  --bg-secondary: #073642;\n  --bg-tertiary: #094858;\n  --bg-input: #002b36;\n  --text-primary: #fdf6e3;\n  --text-secondary: #eee8d5;\n  --text-muted: #657b83;\n  --accent: #268bd2;\n  --accent-hover: #2aa198;\n  --border: #094858;\n  --user-bubble: #094858;\n  --assistant-bubble: #002b36;\n  --danger: #dc322f;\n  --success: #859900;\n}` },
    { name: "Monokai", css: `:root, [data-theme="dark"] {\n  --bg-primary: #272822;\n  --bg-secondary: #1e1f1a;\n  --bg-tertiary: #3e3d32;\n  --bg-input: #272822;\n  --text-primary: #f8f8f2;\n  --text-secondary: #cfcfc2;\n  --text-muted: #75715e;\n  --accent: #a6e22e;\n  --accent-hover: #b6f23e;\n  --border: #3e3d32;\n  --user-bubble: #3e3d32;\n  --assistant-bubble: #272822;\n  --danger: #f92672;\n  --success: #a6e22e;\n}` },
    { name: "Dracula", css: `:root, [data-theme="dark"] {\n  --bg-primary: #282a36;\n  --bg-secondary: #21222c;\n  --bg-tertiary: #44475a;\n  --bg-input: #282a36;\n  --text-primary: #f8f8f2;\n  --text-secondary: #d4d4dc;\n  --text-muted: #6272a4;\n  --accent: #bd93f9;\n  --accent-hover: #caa4fa;\n  --border: #44475a;\n  --user-bubble: #44475a;\n  --assistant-bubble: #282a36;\n  --danger: #ff5555;\n  --success: #50fa7b;\n}` },
  ];

  function applyPreset(preset) {
    customCss = preset.css;
    saveCustomCss();
  }
</script>

<div class="settings-layout">
  <!-- Sidebar navigation -->
  <nav class="settings-nav">
    <div class="nav-header">
      <h2>Settings</h2>
      <button class="back-btn" onclick={onClose} title="Back to chat">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="19" y1="12" x2="5" y2="12"/><polyline points="12 19 5 12 12 5"/>
        </svg>
      </button>
    </div>
    {#each sections as section (section.id)}
      <button
        class="nav-item"
        class:active={activeSection === section.id}
        onclick={() => activeSection = section.id}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d={section.icon}/>
        </svg>
        <span>{section.label}</span>
      </button>
    {/each}

    {#if saveStatus}
      <div class="save-indicator" class:error={saveStatus === "error"}>
        {#if saveStatus === "saving"}Saving...{:else if saveStatus === "saved"}Saved{:else if saveStatus === "error"}Error{/if}
      </div>
    {/if}
  </nav>

  <!-- Content area -->
  <div class="settings-content">
    <!-- GENERAL -->
    {#if activeSection === "general"}
      <div class="section">
        <h3>Provider & Model</h3>
        <div class="card">
          <div class="field">
            <label for="provider">AI Provider</label>
            <select id="provider" bind:value={provider} onchange={saveProvider}>
              <option value="anthropic">Anthropic (Claude)</option>
              <option value="openai">OpenAI</option>
              <option value="ollama">Ollama (Local)</option>
              <option value="custom">Custom Endpoint</option>
            </select>
          </div>

          {#if provider === "anthropic"}
            <div class="field">
              <label for="api-key">API Key</label>
              <input id="api-key" type="password" bind:value={apiKey} placeholder="sk-ant-..." onblur={saveApiKey} />
              <p class="hint">Get your key from <a href="https://console.anthropic.com" target="_blank" rel="noopener">console.anthropic.com</a></p>
            </div>
          {/if}

          {#if provider === "openai"}
            <div class="field">
              <label for="openai-key">API Key</label>
              <input id="openai-key" type="password" bind:value={openaiApiKey} placeholder="sk-..." onblur={saveOpenaiApiKey} />
            </div>
            <div class="field">
              <label for="openai-url">Base URL</label>
              <input id="openai-url" type="text" bind:value={openaiBaseUrl} placeholder="https://api.openai.com" onblur={saveOpenaiBaseUrl} />
              <p class="hint">Change for OpenAI-compatible APIs (Together, Groq, Azure, etc.)</p>
            </div>
          {/if}

          {#if provider === "ollama"}
            <div class="field">
              <label for="ollama-url">Ollama URL</label>
              <div class="input-row">
                <input id="ollama-url" type="text" bind:value={ollamaBaseUrl} placeholder="http://localhost:11434" onblur={saveOllamaBaseUrl} />
                <button class="btn-sm accent" onclick={fetchOllamaModels} disabled={fetchingOllamaModels}>
                  {fetchingOllamaModels ? "..." : "Refresh"}
                </button>
              </div>
              <p class="hint">
                {ollamaModels.length > 0 ? `Found ${ollamaModels.length} model(s)` : "Make sure Ollama is running"}
              </p>
            </div>
          {/if}

          {#if provider === "custom"}
            <div class="field">
              <label for="custom-ep">Custom Endpoint</label>
              {#if customEndpoints.filter(e => e.is_enabled).length > 0}
                <select id="custom-ep" onchange={(e) => saveCustomEndpointId(e.target.value)}>
                  {#each customEndpoints.filter(e => e.is_enabled) as ep}
                    <option value={ep.id}>{ep.name} ({ep.default_model})</option>
                  {/each}
                </select>
                <p class="hint">Select which custom endpoint to use for conversations.</p>
              {:else}
                <p class="hint">No custom endpoints configured. Add one in the Endpoints tab.</p>
              {/if}
            </div>
          {/if}

          <div class="field">
            <label for="model">Model</label>
            {#if provider === "ollama" && ollamaModels.length === 0}
              <input id="model" type="text" bind:value={model} placeholder="e.g. llama3.2" onblur={saveModel} />
            {:else}
              <select id="model" bind:value={model} onchange={saveModel}>
                {#each availableModels as m}
                  <option value={m.id}>{m.name}</option>
                {/each}
              </select>
            {/if}
          </div>
        </div>

        <h3>System Prompt</h3>
        <div class="card">
          <div class="field">
            <textarea bind:value={systemPrompt} placeholder="You are a helpful assistant..." rows="4" onblur={saveSystemPrompt}></textarea>
            <p class="hint">Optional. Applied to all new conversations.</p>
          </div>
        </div>

        <h3>Updates</h3>
        <div class="card">
          <div class="field">
            <label for="update-interval">Check for Updates</label>
            <select id="update-interval" bind:value={updateInterval} onchange={saveUpdateInterval}>
              <option value="never">Never</option>
              <option value="startup">On startup only</option>
              <option value="1800000">Every 30 minutes</option>
              <option value="3600000">Every hour</option>
              <option value="86400000">Once a day</option>
            </select>
          </div>
        </div>
      </div>

    <!-- APPEARANCE -->
    {:else if activeSection === "appearance"}
      <div class="section">
        <h3>Theme</h3>
        <div class="card">
          <div class="field">
            <label for="theme">Color Mode</label>
            <select id="theme" bind:value={theme} onchange={saveTheme}>
              <option value="dark">Dark</option>
              <option value="light">Light</option>
            </select>
          </div>
        </div>

        <h3>Color Scheme</h3>
        <div class="card">
          <div class="field">
            <div class="preset-row">
              {#each themePresets as preset}
                <button
                  class="preset-btn"
                  class:active={customCss === preset.css}
                  onclick={() => applyPreset(preset)}
                >{preset.name}</button>
              {/each}
            </div>
          </div>
          <div class="field">
            <label>Custom CSS</label>
            <textarea
              bind:value={customCss}
              placeholder={"/* Override CSS variables */\n:root, [data-theme='dark'] {\n  --accent: #ff6b6b;\n}"}
              rows="6"
              class="code-textarea"
              onblur={saveCustomCss}
            ></textarea>
            <p class="hint">Override theme variables or add custom styles. Presets give you a starting point.</p>
          </div>
        </div>
      </div>

    <!-- PROMPTS -->
    {:else if activeSection === "prompts"}
      <div class="section">
        <h3>Prompt Library</h3>
        <p class="section-hint">Save reusable prompts. Insert them into chat with the bookmark button. Use <code>{"{{variable}}"}</code> for placeholders.</p>

        {#each prompts as prompt (prompt.id)}
          <div class="card item-card">
            {#if editingPrompt === prompt.id}
              <input type="text" bind:value={prompt.name} placeholder="Prompt name" />
              <textarea bind:value={prompt.content} placeholder="Prompt content..." rows="3"></textarea>
              <div class="item-actions">
                <button class="btn-sm accent" onclick={() => savePrompt(prompt)}>Save</button>
                <button class="btn-sm" onclick={() => (editingPrompt = null)}>Cancel</button>
              </div>
            {:else}
              <div class="item-header">
                <span class="item-name">{prompt.name}</span>
                <div class="item-actions">
                  <button class="btn-sm" onclick={() => (editingPrompt = prompt.id)}>Edit</button>
                  <button class="btn-sm danger" onclick={() => removePrompt(prompt.id)}>Delete</button>
                </div>
              </div>
              <p class="item-preview">{prompt.content.length > 120 ? prompt.content.slice(0, 120) + '...' : prompt.content}</p>
            {/if}
          </div>
        {/each}

        <div class="card add-card">
          <input type="text" bind:value={newPromptName} placeholder="Prompt name" />
          <textarea bind:value={newPromptContent} placeholder="Prompt content... Use {{variable}} for placeholders" rows="2"></textarea>
          <button class="btn-sm accent" onclick={addPrompt} disabled={!newPromptName.trim() || !newPromptContent.trim()}>
            Add Prompt
          </button>
        </div>
      </div>

    <!-- PROJECTS -->
    {:else if activeSection === "projects"}
      <div class="section">
        <h3>Workspace Projects</h3>
        <p class="section-hint">Projects inject context into conversations and can override provider, model, API key, and system prompt per workspace.</p>

        {#each projects as project (project.id)}
          <div class="card item-card">
            {#if editingProject === project.id}
              <input type="text" bind:value={project.name} placeholder="Project name" />
              <textarea bind:value={project.context} placeholder="Project context/instructions..." rows="3"></textarea>
              <details class="overrides-details" open>
                <summary>Provider overrides</summary>
                <div class="overrides-fields">
                  <select bind:value={project.provider}>
                    <option value="">Default provider</option>
                    <option value="anthropic">Anthropic</option>
                    <option value="openai">OpenAI</option>
                    <option value="ollama">Ollama</option>
                  </select>
                  <input type="password" bind:value={project.api_key} placeholder="API key override (optional)" />
                  <input type="text" bind:value={project.model} placeholder="Model override (optional)" />
                  <textarea bind:value={project.system_prompt} placeholder="System prompt override (optional)" rows="2"></textarea>
                </div>
              </details>
              <div class="item-actions">
                <button class="btn-sm accent" onclick={() => saveProject(project)}>Save</button>
                <button class="btn-sm" onclick={() => (editingProject = null)}>Cancel</button>
              </div>
            {:else}
              <div class="item-header">
                <span class="item-name">{project.name}</span>
                <div class="item-actions">
                  <button class="btn-sm" onclick={() => (editingProject = project.id)}>Edit</button>
                  <button class="btn-sm danger" onclick={() => removeProject(project.id)}>Delete</button>
                </div>
              </div>
              {#if project.context}
                <p class="item-preview">{project.context.length > 120 ? project.context.slice(0, 120) + '...' : project.context}</p>
              {/if}
              {#if project.provider || project.model}
                <div class="item-badges">
                  {#if project.provider}<span class="badge">{project.provider}</span>{/if}
                  {#if project.model}<span class="badge">{project.model}</span>{/if}
                </div>
              {/if}
            {/if}
          </div>
        {/each}

        <div class="card add-card">
          <input type="text" bind:value={newProjectName} placeholder="Project name" />
          <textarea bind:value={newProjectContext} placeholder="Project context/instructions (optional)" rows="2"></textarea>
          <details class="overrides-details">
            <summary>Provider overrides (optional)</summary>
            <div class="overrides-fields">
              <select bind:value={newProjectProvider}>
                <option value="">Default provider</option>
                <option value="anthropic">Anthropic</option>
                <option value="openai">OpenAI</option>
                <option value="ollama">Ollama</option>
              </select>
              <input type="password" bind:value={newProjectApiKey} placeholder="API key override" />
              <input type="text" bind:value={newProjectModel} placeholder="Model override" />
              <textarea bind:value={newProjectSystemPrompt} placeholder="System prompt override" rows="2"></textarea>
            </div>
          </details>
          <button class="btn-sm accent" onclick={addProject} disabled={!newProjectName.trim()}>
            Add Project
          </button>
        </div>
      </div>

    <!-- INTEGRATIONS -->
    {:else if activeSection === "integrations"}
      <div class="section">
        <h3>MCP Servers</h3>
        <p class="section-hint">Connect to Model Context Protocol servers for tool use (Anthropic provider only).</p>

        {#each mcpServers as server, i (i)}
          <div class="card item-card">
            <div class="item-header">
              <span class="item-name">{server.name}</span>
              <button class="btn-sm danger" onclick={() => removeMcpServer(i)}>Remove</button>
            </div>
            <p class="item-preview"><code>{server.command} {server.args.join(' ')}</code></p>
          </div>
        {/each}

        <div class="card add-card">
          <input type="text" bind:value={newMcpName} placeholder="Server name (e.g. filesystem)" />
          <input type="text" bind:value={newMcpCommand} placeholder="Command (e.g. npx)" />
          <input type="text" bind:value={newMcpArgs} placeholder="Arguments (space-separated)" />
          <button class="btn-sm accent" onclick={addMcpServer} disabled={!newMcpName.trim() || !newMcpCommand.trim()}>
            Add MCP Server
          </button>
        </div>

        <h3>Custom Commands</h3>
        <p class="section-hint">Define slash commands that run shell scripts. Type <code>/</code> in chat to use them.</p>

        {#each customCommands as cmd, i (i)}
          <div class="card item-card">
            <div class="item-header">
              <span class="item-name">/{cmd.name}</span>
              <button class="btn-sm danger" onclick={() => removeCustomCommand(i)}>Remove</button>
            </div>
            <p class="item-preview">
              <code>{cmd.command}</code>
              {#if cmd.description}<br/><span class="item-desc">{cmd.description}</span>{/if}
            </p>
          </div>
        {/each}

        <div class="card add-card">
          <input type="text" bind:value={newCmdName} placeholder="Command name (e.g. gitlog)" />
          <input type="text" bind:value={newCmdCommand} placeholder="Shell command (e.g. git log --oneline -10)" />
          <input type="text" bind:value={newCmdDescription} placeholder="Description (optional)" />
          <button class="btn-sm accent" onclick={addCustomCommand} disabled={!newCmdName.trim() || !newCmdCommand.trim()}>
            Add Command
          </button>
        </div>
      </div>

    <!-- SCHEDULES -->
    {:else if activeSection === "schedules"}
      <div class="section">
        <h3>Scheduled Prompts</h3>
        <p class="section-hint">Automatically send prompts at recurring intervals. Creates a new conversation each time.</p>

        {#each scheduledPrompts as sp (sp.id)}
          <div class="card item-card">
            {#if editingSched === sp.id}
              <input type="text" bind:value={sp.name} placeholder="Name" />
              <textarea bind:value={sp.prompt} placeholder="Prompt text..." rows="2"></textarea>
              <select bind:value={sp.interval_ms}>
                <option value={300000}>Every 5 minutes</option>
                <option value={900000}>Every 15 minutes</option>
                <option value={1800000}>Every 30 minutes</option>
                <option value={3600000}>Every hour</option>
                <option value={21600000}>Every 6 hours</option>
                <option value={43200000}>Every 12 hours</option>
                <option value={86400000}>Every day</option>
              </select>
              <div class="item-actions">
                <button class="btn-sm accent" onclick={() => saveScheduledPrompt(sp)}>Save</button>
                <button class="btn-sm" onclick={() => (editingSched = null)}>Cancel</button>
              </div>
            {:else}
              <div class="item-header">
                <div class="item-name-group">
                  <span class="item-name">{sp.name}</span>
                  <span class="badge accent">{formatInterval(sp.interval_ms)}</span>
                  {#if !sp.enabled}<span class="badge muted">paused</span>{/if}
                </div>
                <div class="item-actions">
                  <button class="btn-sm" onclick={() => { sp.enabled = !sp.enabled; saveScheduledPrompt(sp); }}>
                    {sp.enabled ? "Pause" : "Resume"}
                  </button>
                  <button class="btn-sm" onclick={() => (editingSched = sp.id)}>Edit</button>
                  <button class="btn-sm danger" onclick={() => removeScheduledPrompt(sp.id)}>Delete</button>
                </div>
              </div>
              <p class="item-preview">{sp.prompt.length > 100 ? sp.prompt.slice(0, 100) + '...' : sp.prompt}</p>
            {/if}
          </div>
        {/each}

        <div class="card add-card">
          <input type="text" bind:value={newSchedName} placeholder="Schedule name" />
          <textarea bind:value={newSchedPrompt} placeholder="Prompt to send..." rows="2"></textarea>
          <select bind:value={newSchedInterval}>
            <option value="300000">Every 5 minutes</option>
            <option value="900000">Every 15 minutes</option>
            <option value="1800000">Every 30 minutes</option>
            <option value="3600000">Every hour</option>
            <option value="21600000">Every 6 hours</option>
            <option value="43200000">Every 12 hours</option>
            <option value="86400000">Every day</option>
          </select>
          <button class="btn-sm accent" onclick={addScheduledPrompt} disabled={!newSchedName.trim() || !newSchedPrompt.trim()}>
            Add Schedule
          </button>
        </div>
      </div>

    <!-- CUSTOM ENDPOINTS -->
    {:else if activeSection === "endpoints"}
      <h2>Custom Endpoints</h2>
      <p class="section-desc">Add OpenAI-compatible or Anthropic-compatible API endpoints (e.g. Together AI, Groq, Fireworks).</p>

      {#each customEndpoints as ep}
        <div class="setting-card">
          {#if editingEndpoint === ep.id}
            <div class="form-grid">
              <input class="setting-input" bind:value={ep.name} placeholder="Name" />
              <input class="setting-input" bind:value={ep.base_url} placeholder="Base URL" />
              <input class="setting-input" type="password" bind:value={ep.api_key} placeholder="API Key" />
              <select class="setting-select" bind:value={ep.api_format}>
                <option value="openai">OpenAI-compatible</option>
                <option value="anthropic">Anthropic-compatible</option>
              </select>
              <input class="setting-input" bind:value={ep.default_model} placeholder="Default model" />
              <div class="card-actions">
                <button class="btn-sm btn-primary" onclick={() => saveEndpoint(ep)}>Save</button>
                <button class="btn-sm" onclick={() => { editingEndpoint = null; loadCustomEndpoints(); }}>Cancel</button>
              </div>
            </div>
          {:else}
            <div class="card-row">
              <div class="card-info">
                <strong>{ep.name}</strong>
                <span class="text-muted">{ep.base_url} ({ep.api_format})</span>
                <span class="text-muted">Model: {ep.default_model || 'none'}</span>
              </div>
              <div class="card-actions">
                <label class="toggle-label">
                  <input type="checkbox" checked={ep.is_enabled} onchange={() => { ep.is_enabled = !ep.is_enabled; saveEndpoint(ep); }} />
                  Enabled
                </label>
                <button class="btn-sm" onclick={() => { editingEndpoint = ep.id; }}>Edit</button>
                <button class="btn-sm btn-danger" onclick={() => deleteEndpoint(ep.id)}>Delete</button>
              </div>
            </div>
          {/if}
        </div>
      {/each}

      <div class="add-form">
        <h3>Add Endpoint</h3>
        <div class="form-grid">
          <input class="setting-input" bind:value={newEpName} placeholder="Name (e.g. Together AI)" />
          <input class="setting-input" bind:value={newEpBaseUrl} placeholder="Base URL (e.g. https://api.together.xyz)" />
          <input class="setting-input" type="password" bind:value={newEpApiKey} placeholder="API Key" />
          <select class="setting-select" bind:value={newEpFormat}>
            <option value="openai">OpenAI-compatible</option>
            <option value="anthropic">Anthropic-compatible</option>
          </select>
          <input class="setting-input" bind:value={newEpModel} placeholder="Default model" />
        </div>
        <div class="form-actions">
          <button class="btn-sm" onclick={() => testEndpointConnection(newEpBaseUrl, newEpApiKey, newEpFormat)} disabled={testingEndpoint || !newEpBaseUrl}>
            {testingEndpoint ? 'Testing...' : 'Test Connection'}
          </button>
          {#if testResult}<span class="text-muted">{testResult}</span>{/if}
          <button class="btn-sm btn-primary" onclick={addEndpoint} disabled={!newEpName || !newEpBaseUrl}>Add</button>
        </div>
      </div>

      <h3 style="margin-top: 24px;">Model Pricing</h3>
      <p class="section-desc">Cost per million tokens. Pre-populated for common models.</p>
      <div class="pricing-table">
        <div class="pricing-header">
          <span>Model</span><span>Input $/Mtok</span><span>Output $/Mtok</span><span>Provider</span>
        </div>
        {#each modelPricing as p}
          <div class="pricing-row">
            <span>{p.model_pattern}</span>
            <span>${p.input_cost_per_mtok}</span>
            <span>${p.output_cost_per_mtok}</span>
            <span class="text-muted">{p.provider}</span>
          </div>
        {/each}
      </div>

    <!-- MODEL ROUTING -->
    {:else if activeSection === "routing"}
      <h2>Model Routing</h2>
      <p class="section-desc">Auto-select models based on prompt content. Rules are evaluated by priority (highest first).</p>

      {#each routingRules as rule}
        <div class="setting-card">
          <div class="card-row">
            <div class="card-info">
              <strong>{rule.name}</strong>
              <span class="text-muted">Pattern: {rule.pattern}</span>
              <span class="text-muted">Target: {rule.target_provider}/{rule.target_model}</span>
              <span class="text-muted">Priority: {rule.priority} | Type: {rule.task_type}</span>
            </div>
            <div class="card-actions">
              <label class="toggle-label">
                <input type="checkbox" checked={rule.is_enabled} onchange={() => toggleRoutingRule(rule)} />
                Enabled
              </label>
              <button class="btn-sm btn-danger" onclick={() => deleteRoutingRule(rule.id)}>Delete</button>
            </div>
          </div>
        </div>
      {/each}

      <div class="add-form">
        <h3>Add Rule</h3>
        <div class="form-grid">
          <input class="setting-input" bind:value={newRuleName} placeholder="Rule name (e.g. 'Code tasks')" />
          <input class="setting-input" bind:value={newRulePattern} placeholder="Keywords separated by | (e.g. code|debug|function)" />
          <select class="setting-select" bind:value={newRuleTaskType}>
            <option value="code">Code</option>
            <option value="creative">Creative</option>
            <option value="analysis">Analysis</option>
            <option value="quick">Quick</option>
            <option value="custom">Custom</option>
          </select>
          <select class="setting-select" bind:value={newRuleProvider}>
            <option value="anthropic">Anthropic</option>
            <option value="openai">OpenAI</option>
            <option value="ollama">Ollama</option>
            <option value="custom">Custom</option>
          </select>
          <input class="setting-input" bind:value={newRuleModel} placeholder="Target model (e.g. claude-sonnet-4-6)" />
          <input class="setting-input" type="number" bind:value={newRulePriority} placeholder="Priority (higher = first)" />
        </div>
        <div class="form-actions">
          <button class="btn-sm btn-primary" onclick={addRoutingRule} disabled={!newRuleName || !newRulePattern || !newRuleModel}>Add Rule</button>
        </div>
      </div>

    <!-- KNOWLEDGE & CONTEXT -->
    {:else if activeSection === "knowledge"}
      <div class="section">
        <h3>User Memory</h3>
        <p class="section-hint">Persistent key/value facts injected into every conversation's system prompt.</p>

        {#each memoryEntries as entry (entry.id)}
          <div class="card item-card">
            <div class="item-header">
              <span class="item-name">{entry.key}</span>
              <button class="btn-sm danger" onclick={() => removeMemoryEntry(entry.id)}>Delete</button>
            </div>
            <p class="item-preview">{entry.value}</p>
          </div>
        {/each}

        <div class="card add-card">
          <input type="text" bind:value={newMemKey} placeholder="Key (e.g. preferred_language)" />
          <input type="text" bind:value={newMemValue} placeholder="Value (e.g. TypeScript)" />
          <button class="btn-sm accent" onclick={addMemoryEntry} disabled={!newMemKey.trim() || !newMemValue.trim()}>
            Add Memory
          </button>
        </div>

        <h3>Knowledge Base</h3>
        <p class="section-hint">Documents and content injected into conversation context when enabled. Supports manual entry, URL import, and file import.</p>

        {#each knowledgeEntries as entry (entry.id)}
          <div class="card item-card">
            {#if editingKnowledge === entry.id}
              <input type="text" bind:value={entry.title} placeholder="Title" />
              <textarea bind:value={entry.content} placeholder="Content..." rows="4"></textarea>
              <div class="item-actions">
                <button class="btn-sm accent" onclick={() => saveKnowledgeEntry(entry)}>Save</button>
                <button class="btn-sm" onclick={() => (editingKnowledge = null)}>Cancel</button>
              </div>
            {:else}
              <div class="item-header">
                <div class="item-name-group">
                  <button
                    class="toggle-btn"
                    class:enabled={entry.enabled}
                    onclick={() => toggleKnowledgeEnabled(entry)}
                    title={entry.enabled ? "Enabled - click to disable" : "Disabled - click to enable"}
                    aria-label={entry.enabled ? "Disable" : "Enable"}
                  >
                    {entry.enabled ? "ON" : "OFF"}
                  </button>
                  <span class="item-name">{entry.title}</span>
                </div>
                <div class="item-actions">
                  <button class="btn-sm" onclick={() => (editingKnowledge = entry.id)}>Edit</button>
                  <button class="btn-sm danger" onclick={() => removeKnowledgeEntry(entry.id)}>Delete</button>
                </div>
              </div>
              <div class="item-badges">
                <span class="badge">{entry.source_type}</span>
                {#if entry.project_id}<span class="badge accent">project</span>{/if}
                {#if entry.source_url}<span class="badge muted" title={entry.source_url}>URL</span>{/if}
                {#if entry.file_path}<span class="badge muted" title={entry.file_path}>File</span>{/if}
              </div>
              <p class="item-preview">{entry.content.length > 150 ? entry.content.slice(0, 150) + '...' : entry.content}</p>
            {/if}
          </div>
        {/each}

        <div class="card add-card">
          <div class="field">
            <label>Source type</label>
            <select bind:value={newKbSourceType}>
              <option value="manual">Manual text</option>
              <option value="url">Import from URL</option>
            </select>
          </div>
          <input type="text" bind:value={newKbTitle} placeholder="Title" />
          {#if newKbSourceType === "url"}
            <input type="url" bind:value={newKbUrl} placeholder="https://example.com/article" />
          {:else}
            <textarea bind:value={newKbContent} placeholder="Content..." rows="3"></textarea>
          {/if}
          <select bind:value={newKbProjectId}>
            <option value="">Global (all conversations)</option>
            {#each projects as project (project.id)}
              <option value={project.id}>{project.name}</option>
            {/each}
          </select>
          <div class="kb-add-actions">
            <button class="btn-sm accent" onclick={addKnowledgeEntry} disabled={!newKbTitle.trim() || (newKbSourceType === "manual" ? !newKbContent.trim() : !newKbUrl.trim()) || importingKbUrl}>
              {importingKbUrl ? "Importing..." : "Add Entry"}
            </button>
            <button class="btn-sm" onclick={importFileToKnowledge} title="Import a local file">
              Import File
            </button>
            <label class="watch-label">
              <input type="checkbox" bind:checked={newKbWatch} />
              <span>Watch for changes</span>
            </label>
          </div>
        </div>

        {#if fileWatches.length > 0}
          <h3>File Watches</h3>
          <p class="section-hint">Files being monitored for changes (checked every 30 seconds).</p>
          {#each fileWatches as fw (fw.id)}
            <div class="card item-card">
              <div class="item-header">
                <span class="item-preview" style="flex:1; font-family: 'JetBrains Mono', monospace; font-size: 11px;">{fw.file_path}</span>
                <button class="btn-sm danger" onclick={() => removeFileWatch(fw.id)}>Stop</button>
              </div>
            </div>
          {/each}
        {/if}
      </div>

    <!-- DATA & USAGE -->
    {:else if activeSection === "data"}
      <div class="section">
        {#if totalUsage}
          <h3>Token Usage</h3>
          <div class="usage-grid">
            <div class="usage-stat">
              <span class="usage-value">{totalUsage.input_tokens.toLocaleString()}</span>
              <span class="usage-label">Input Tokens</span>
            </div>
            <div class="usage-stat">
              <span class="usage-value">{totalUsage.output_tokens.toLocaleString()}</span>
              <span class="usage-label">Output Tokens</span>
            </div>
            <div class="usage-stat">
              <span class="usage-value">{totalUsage.total_tokens.toLocaleString()}</span>
              <span class="usage-label">Total Tokens</span>
            </div>
            <div class="usage-stat">
              <span class="usage-value">{totalUsage.message_count.toLocaleString()}</span>
              <span class="usage-label">Messages</span>
            </div>
          </div>
        {/if}

        {#if costSummary.length > 0}
          <h3>Cost by Model</h3>
          <div class="pricing-table">
            <div class="pricing-header">
              <span>Model</span><span>Est. Cost</span><span>Input Tokens</span><span>Output Tokens</span>
            </div>
            {#each costSummary as [model, cost, inputTok, outputTok]}
              <div class="pricing-row">
                <span>{model || 'unknown'}</span>
                <span>${cost < 0.01 ? cost.toFixed(4) : cost.toFixed(2)}</span>
                <span>{inputTok.toLocaleString()}</span>
                <span>{outputTok.toLocaleString()}</span>
              </div>
            {/each}
            <div class="pricing-row" style="font-weight: 600; border-top: 2px solid var(--border-color, #444);">
              <span>Total</span>
              <span>${costSummary.reduce((s, c) => s + c[1], 0).toFixed(2)}</span>
              <span>{costSummary.reduce((s, c) => s + c[2], 0).toLocaleString()}</span>
              <span>{costSummary.reduce((s, c) => s + c[3], 0).toLocaleString()}</span>
            </div>
          </div>
        {/if}

        <h3>Database</h3>
        <div class="card">
          <div class="db-info">
            <span class="db-path" title={dbPath}>{dbPath || "..."}</span>
            <span class="db-size">{formatBytes(dbSize)}</span>
          </div>
          <div class="db-actions">
            <button class="btn-action" onclick={backupDatabase} disabled={dbStatus === "backing-up" || dbStatus === "restoring"}>
              {dbStatus === "backing-up" ? "Backing up..." : "Backup"}
            </button>
            <button class="btn-action outline-danger" onclick={restoreDatabase} disabled={dbStatus === "backing-up" || dbStatus === "restoring"}>
              {dbStatus === "restoring" ? "Restoring..." : "Restore"}
            </button>
          </div>
          {#if dbStatus === "backed-up"}
            <div class="status-msg success">Backup saved successfully.</div>
          {/if}
          {#if dbStatus === "restored"}
            <div class="status-msg success">Database restored. Restart the app to load restored data.</div>
          {/if}
          {#if dbStatus === "error"}
            <div class="status-msg error">{dbError}</div>
          {/if}
        </div>
      </div>

    <!-- ABOUT -->
    {:else if activeSection === "about"}
      <div class="section">
        <h3>About Linux Claude Desktop</h3>
        <div class="card about-card">
          <div class="about-row">
            <span class="about-label">Version</span>
            <span class="about-value">{appVersion || "..."}</span>
          </div>
          <div class="about-row">
            <span class="about-label">Operating System</span>
            <span class="about-value">{appOs || "..."}</span>
          </div>
          <div class="about-row">
            <span class="about-label">Architecture</span>
            <span class="about-value">{appArch || "..."}</span>
          </div>
          <div class="about-row">
            <span class="about-label">Source Code</span>
            <a href="https://github.com/ponack/linux-claude-desktop" target="_blank" rel="noopener noreferrer" class="about-link">
              github.com/ponack/linux-claude-desktop
            </a>
          </div>
        </div>
      </div>
    {/if}

    {#if saveError}
      <div class="status-msg error global-error">{saveError}</div>
    {/if}
  </div>
</div>

<style>
  .settings-layout {
    display: flex;
    height: 100%;
    overflow: hidden;
  }

  /* --- Sidebar Nav --- */
  .settings-nav {
    width: 200px;
    min-width: 200px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 16px 0;
    gap: 2px;
    overflow-y: auto;
  }

  .nav-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px 16px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 8px;
  }

  .nav-header h2 {
    font-size: 16px;
    font-weight: 700;
    margin: 0;
  }

  .back-btn {
    color: var(--text-muted);
    padding: 6px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    transition: all 0.15s;
  }
  .back-btn:hover { color: var(--text-primary); background: var(--bg-tertiary); }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    transition: all 0.15s;
    text-align: left;
    border-left: 3px solid transparent;
  }
  .nav-item:hover { color: var(--text-primary); background: var(--bg-tertiary); }
  .nav-item.active {
    color: var(--accent);
    background: rgba(78, 204, 163, 0.08);
    border-left-color: var(--accent);
  }
  .nav-item svg { flex-shrink: 0; opacity: 0.7; }
  .nav-item.active svg { opacity: 1; }

  .save-indicator {
    margin-top: auto;
    padding: 8px 16px;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    text-align: center;
  }
  .save-indicator.error { color: var(--danger); }

  /* --- Content --- */
  .settings-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px 32px 48px;
    min-width: 0;
  }

  .section {
    max-width: 560px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .section h3 {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
    padding-top: 8px;
  }
  .section h3:first-child { padding-top: 0; }

  .section-hint {
    font-size: 12px;
    color: var(--text-muted);
    margin: -8px 0 0;
    line-height: 1.5;
  }
  .section-hint code {
    background: var(--bg-tertiary);
    padding: 1px 4px;
    border-radius: 3px;
    font-size: 11px;
  }

  /* --- Cards --- */
  .card {
    background: var(--bg-tertiary);
    border-radius: 10px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .item-card { gap: 8px; }
  .add-card { gap: 8px; border: 1px dashed var(--border); background: transparent; }

  /* --- Fields --- */
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  input, select {
    padding: 9px 12px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s;
    width: 100%;
    box-sizing: border-box;
    color: var(--text-primary);
  }

  textarea {
    padding: 9px 12px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 13px;
    outline: none;
    resize: vertical;
    min-height: 60px;
    line-height: 1.5;
    color: var(--text-primary);
    transition: border-color 0.15s;
    width: 100%;
    box-sizing: border-box;
  }

  .code-textarea {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
  }

  input:focus, select:focus, textarea:focus { border-color: var(--accent); }
  select {
    cursor: pointer;
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23888' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 10px center;
    padding-right: 30px;
  }
  option { background: var(--bg-secondary); color: var(--text-primary); }

  .hint { font-size: 11px; color: var(--text-muted); margin: 0; }
  .hint a { color: var(--accent); text-decoration: none; }
  .hint a:hover { text-decoration: underline; }

  .input-row { display: flex; gap: 6px; align-items: center; }
  .input-row input { flex: 1; }

  /* --- Presets --- */
  .preset-row { display: flex; gap: 6px; flex-wrap: wrap; }

  .preset-btn {
    padding: 6px 14px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    transition: all 0.15s;
  }
  .preset-btn:hover { background: var(--bg-secondary); color: var(--text-primary); }
  .preset-btn.active { background: var(--accent); color: white; border-color: var(--accent); }

  /* --- Buttons --- */
  .btn-sm {
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    transition: all 0.15s;
    white-space: nowrap;
    align-self: flex-start;
  }
  .btn-sm:hover { background: var(--bg-secondary); color: var(--text-primary); }
  .btn-sm.accent { background: var(--accent); color: white; border: none; }
  .btn-sm.accent:hover { background: var(--accent-hover); }
  .btn-sm.accent:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-sm.danger { color: var(--danger); border-color: var(--danger); }
  .btn-sm.danger:hover { background: rgba(233, 69, 96, 0.1); }

  .btn-action {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    background: var(--accent);
    color: white;
    transition: background 0.15s;
  }
  .btn-action:hover:not(:disabled) { background: var(--accent-hover); }
  .btn-action:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-action.outline-danger {
    background: transparent;
    color: var(--danger);
    border: 1px solid var(--danger);
  }
  .btn-action.outline-danger:hover:not(:disabled) { background: rgba(233, 69, 96, 0.1); }

  /* --- Items --- */
  .item-header { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
  .item-name { font-weight: 600; font-size: 13px; color: var(--text-primary); }
  .item-name-group { display: flex; align-items: center; gap: 6px; }
  .item-preview { font-size: 12px; color: var(--text-muted); line-height: 1.4; margin: 0; }
  .item-preview code { font-size: 11px; background: var(--bg-input); padding: 1px 4px; border-radius: 3px; }
  .item-desc { color: var(--text-muted); font-size: 11px; }
  .item-actions { display: flex; gap: 4px; flex-shrink: 0; }

  .item-badges { display: flex; gap: 4px; margin-top: 2px; }

  .badge {
    font-size: 10px;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg-input);
    color: var(--text-muted);
    font-weight: 500;
  }
  .badge.accent { color: var(--accent); background: rgba(78, 204, 163, 0.15); }
  .badge.muted { color: var(--text-muted); }

  /* --- Overrides --- */
  .overrides-details { margin-top: 4px; }
  .overrides-details summary { font-size: 12px; color: var(--text-muted); cursor: pointer; }
  .overrides-details summary:hover { color: var(--text-secondary); }
  .overrides-fields { display: flex; flex-direction: column; gap: 6px; margin-top: 8px; }
  .overrides-fields select, .overrides-fields input, .overrides-fields textarea {
    padding: 6px 8px; font-size: 12px;
  }

  /* --- Usage Grid --- */
  .usage-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .usage-stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 14px;
    background: var(--bg-tertiary);
    border-radius: 10px;
  }

  .usage-value {
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .usage-label {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 600;
  }

  /* --- Database --- */
  .db-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-input);
    border-radius: 6px;
  }

  .db-path {
    font-size: 11px;
    color: var(--text-muted);
    font-family: "JetBrains Mono", "Fira Code", monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    margin-right: 12px;
  }

  .db-size {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .db-actions { display: flex; gap: 8px; }

  .status-msg {
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
  }
  .status-msg.success { background: rgba(78, 204, 163, 0.15); color: var(--success, #4ecca3); }
  .status-msg.error { background: rgba(233, 69, 96, 0.1); color: var(--danger); }
  .global-error { margin-top: 16px; }

  /* --- About --- */
  .about-card { gap: 0; }

  .about-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }
  .about-row:last-child { border-bottom: none; }

  .about-label {
    font-size: 13px;
    color: var(--text-muted);
  }

  .about-value {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .about-link {
    font-size: 13px;
    color: var(--accent);
    text-decoration: none;
    font-weight: 500;
  }
  .about-link:hover { text-decoration: underline; }

  /* --- Knowledge & Memory --- */
  .toggle-btn {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    border: 1px solid var(--border);
    color: var(--text-muted);
    background: var(--bg-input);
    cursor: pointer;
    transition: all 0.15s;
    letter-spacing: 0.3px;
  }
  .toggle-btn.enabled {
    background: rgba(78, 204, 163, 0.15);
    color: var(--accent);
    border-color: var(--accent);
  }

  .kb-add-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .watch-label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--text-muted);
    cursor: pointer;
    text-transform: none;
    font-weight: 400;
    letter-spacing: 0;
  }
  .watch-label input[type="checkbox"] {
    width: auto;
    margin: 0;
  }

  /* Endpoints & Routing */
  .setting-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color, #444);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 8px;
  }
  .card-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }
  .card-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .card-info strong { font-size: 0.95em; }
  .card-actions {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-shrink: 0;
  }
  .toggle-label {
    display: flex;
    gap: 4px;
    align-items: center;
    font-size: 0.85em;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .toggle-label input { width: auto; margin: 0; }
  .btn-sm {
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--border-color, #444);
    background: var(--bg-primary);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 0.8em;
  }
  .btn-sm:hover { background: var(--bg-secondary); }
  .btn-sm.btn-primary {
    background: var(--accent-color, #6366f1);
    border-color: var(--accent-color, #6366f1);
    color: white;
  }
  .btn-sm.btn-danger { border-color: #ef4444; color: #ef4444; }
  .btn-sm.btn-danger:hover { background: #ef444420; }
  .form-grid {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .add-form {
    margin-top: 12px;
    padding: 12px;
    border: 1px dashed var(--border-color, #444);
    border-radius: 8px;
  }
  .add-form h3 { margin: 0 0 8px; font-size: 0.95em; }
  .form-actions {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 8px;
  }
  .section-desc {
    color: var(--text-secondary);
    font-size: 0.85em;
    margin-bottom: 12px;
  }
  .pricing-table {
    border: 1px solid var(--border-color, #444);
    border-radius: 8px;
    overflow: hidden;
    font-size: 0.85em;
  }
  .pricing-header, .pricing-row {
    display: grid;
    grid-template-columns: 2fr 1fr 1fr 1fr;
    padding: 6px 12px;
    gap: 8px;
  }
  .pricing-header {
    background: var(--bg-secondary);
    font-weight: 600;
    border-bottom: 1px solid var(--border-color, #444);
  }
  .pricing-row { border-bottom: 1px solid var(--border-color, #333); }
  .pricing-row:last-child { border-bottom: none; }
</style>
