<template>
  <div class="app">
    <!-- Sidebar -->
    <div class="sidebar">
      <button class="new-chat-btn" @click="newChat">+ New Chat</button>
      <div class="chat-list">
        <div
          v-for="chat in chats"
          :key="chat.id"
          class="chat-item"
          :class="{ active: currentChatId === chat.id }"
          @click="switchChat(chat.id)"
        >
          <span class="chat-title">{{ chat.title }}</span>
          <button
            class="delete-btn"
            @click.stop="deleteChat(chat.id)"
            title="Delete"
          >
            ✕
          </button>
        </div>
      </div>
    </div>

    <!-- Main -->
    <div class="main">
      <div class="header">
        <div class="header-left">
          <div class="status-dot active"></div>
          <h1>{{ currentChat?.title || "Local Chat" }}</h1>
        </div>
      </div>

      <div class="messages" ref="messagesEl">
        <div v-if="!currentMessages.length" class="empty-state">
          <div class="empty-icon">🤖</div>
          <p>Ask me anything!</p>
          <p class="empty-sub">
            Powered by Qwen3 — running locally on your device
          </p>
        </div>

        <div
          v-for="(msg, i) in currentMessages"
          :key="i"
          class="message"
          :class="msg.role"
        >
          <div class="bubble">
            <span class="role-label">{{
              msg.role === "user" ? "You" : "AI"
            }}</span>
            <p v-if="msg.role === 'user'">{{ msg.content }}</p>
            <div
              v-else
              class="markdown"
              v-html="renderMarkdown(msg.content)"
            ></div>
          </div>
        </div>

        <div
          v-if="
            loading &&
            currentMessages[currentMessages.length - 1]?.content === ''
          "
          class="message assistant"
        >
          <div class="bubble">
            <span class="role-label">AI</span>
            <div class="typing"><span></span><span></span><span></span></div>
          </div>
        </div>
      </div>

      <div v-if="error" class="error-bar">⚠️ {{ error }}</div>

      <div class="input-area">
        <textarea
          v-model="input"
          class="input"
          placeholder="Type a message..."
          :disabled="loading"
          rows="1"
          @keydown.enter.exact.prevent="sendMessage"
          @input="autoResize"
          ref="inputEl"
        />
        <button
          class="send-btn"
          :disabled="loading || !input.trim()"
          @click="sendMessage"
        >
          ➤
        </button>
      </div>

      <p class="hint">Enter to send · Shift+Enter for new line</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Message {
  role: "user" | "assistant";
  content: string;
}

interface Chat {
  id: string;
  title: string;
  messages: Message[];
}

const chats = ref<Chat[]>([]);
const currentChatId = ref<string | null>(null);
const input = ref("");
const loading = ref(false);
const error = ref("");
const messagesEl = ref<HTMLElement>();
const inputEl = ref<HTMLTextAreaElement>();
let unlisten: any = null;

const currentChat = computed(
  () => chats.value.find((c) => c.id === currentChatId.value) || null,
);

const currentMessages = computed(() => currentChat.value?.messages || []);

// --- Load chats on startup ---
onMounted(async () => {
  try {
    const saved = await invoke<Chat[]>("load_chats");
    chats.value = saved;
    if (saved.length > 0) {
      currentChatId.value = saved[0].id;
    } else {
      newChat();
    }
  } catch {
    newChat();
  }
});

// --- New chat ---
function newChat() {
  const id = Date.now().toString();
  const chat: Chat = {
    id,
    title: "New Chat",
    messages: [],
  };
  chats.value.unshift(chat);
  currentChatId.value = id;
}

// --- Switch chat ---
function switchChat(id: string) {
  currentChatId.value = id;
  nextTick(() => scrollToBottom());
}

// --- Delete chat ---
async function deleteChat(id: string) {
  try {
    await invoke("delete_chat", { id });
    chats.value = chats.value.filter((c) => c.id !== id);
    if (currentChatId.value === id) {
      if (chats.value.length > 0) {
        currentChatId.value = chats.value[0].id;
      } else {
        newChat();
      }
    }
  } catch (e) {
    error.value = String(e);
  }
}

// --- Save current chat ---
async function saveCurrentChat() {
  if (!currentChat.value) return;
  try {
    await invoke("save_chat", { chat: currentChat.value });
  } catch {}
}

// --- Markdown renderer ---
function renderMarkdown(text: string): string {
  if (!text) return "";
  let html = text;
  html = html
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  html = html.replace(
    /```(\w*)\n?([\s\S]*?)```/g,
    (_, lang, code) =>
      `<pre><code class="lang-${lang}">${code.trim()}</code></pre>`,
  );
  html = html.replace(/`([^`]+)`/g, "<code>$1</code>");
  html = html.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  html = html.replace(/\*([^*]+)\*/g, "<em>$1</em>");
  html = html.replace(/^### (.+)$/gm, "<h3>$1</h3>");
  html = html.replace(/^## (.+)$/gm, "<h2>$1</h2>");
  html = html.replace(/^# (.+)$/gm, "<h1>$1</h1>");
  html = html.replace(/^[\-\*] (.+)$/gm, "<li>$1</li>");
  html = html.replace(/(<li>[\s\S]*?<\/li>)/g, "<ul>$1</ul>");
  html = html.replace(/^\d+\. (.+)$/gm, "<oli>$1</oli>");
  html = html.replace(
    /(<oli>[\s\S]*?<\/oli>)/g,
    (match) =>
      "<ol>" +
      match.replace(/<oli>/g, "<li>").replace(/<\/oli>/g, "</li>") +
      "</ol>",
  );
  html = html.replace(/^---$/gm, "<hr>");
  html = html
    .split(/\n\n+/)
    .map((block) => {
      block = block.trim();
      if (!block) return "";
      if (/^<(h[1-3]|ul|ol|pre|hr)/.test(block)) return block;
      return `<p>${block.replace(/\n/g, "<br>")}</p>`;
    })
    .join("\n");
  return html;
}

// --- Send message ---
async function sendMessage() {
  const text = input.value.trim();
  if (!text || loading.value || !currentChat.value) return;

  error.value = "";
  input.value = "";
  resetResize();

  // Auto-title the chat from first message
  if (currentChat.value.messages.length === 0) {
    currentChat.value.title =
      text.slice(0, 30) + (text.length > 30 ? "..." : "");
  }

  currentChat.value.messages.push({ role: "user", content: text });
  scrollToBottom();
  loading.value = true;

  currentChat.value.messages.push({ role: "assistant", content: "" });
  const assistantIndex = currentChat.value.messages.length - 1;

  unlisten = await listen<string>("chat-chunk", (event) => {
    if (currentChat.value) {
      currentChat.value.messages[assistantIndex].content = event.payload;
      scrollToBottom();
    }
  });

  try {
    const final = await invoke<string>("chat", {
      messages: currentChat.value.messages.slice(0, -1),
    });

    currentChat.value.messages[assistantIndex].content = final;
    scrollToBottom();
    await saveCurrentChat();
  } catch (e) {
    if (
      currentChat.value &&
      !currentChat.value.messages[assistantIndex].content
    ) {
      currentChat.value.messages[assistantIndex].content = "⚠️ " + String(e);
    }
    error.value = String(e);
  } finally {
    loading.value = false;
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
    nextTick(() => inputEl.value?.focus());
  }
}

async function scrollToBottom() {
  await nextTick();
  if (messagesEl.value) {
    messagesEl.value.scrollTop = messagesEl.value.scrollHeight;
  }
}

function autoResize(e: Event) {
  const el = e.target as HTMLTextAreaElement;
  el.style.height = "auto";
  el.style.height = Math.min(el.scrollHeight, 160) + "px";
}

function resetResize() {
  if (inputEl.value) inputEl.value.style.height = "auto";
}
</script>

<style>
* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  font-family:
    system-ui,
    -apple-system,
    sans-serif;
  background: #0d0d0d;
  color: #f0f0f0;
  height: 100vh;
  overflow: hidden;
}

.app {
  display: flex;
  height: 100vh;
}

/* Sidebar */
.sidebar {
  width: 240px;
  min-width: 240px;
  background: #111;
  border-right: 1px solid #1e1e1e;
  display: flex;
  flex-direction: column;
  padding: 12px;
  gap: 8px;
}

.new-chat-btn {
  background: #1e1e1e;
  color: #f0f0f0;
  border: 1px solid #2a2a2a;
  border-radius: 8px;
  padding: 10px 14px;
  font-size: 13px;
  cursor: pointer;
  text-align: left;
  transition: all 0.2s;
}
.new-chat-btn:hover {
  background: #2a2a2a;
  border-color: #444;
}

.chat-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.chat-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s;
  gap: 6px;
}
.chat-item:hover {
  background: #1e1e1e;
}
.chat-item.active {
  background: #1e1e2e;
  border: 1px solid #2a2a4a;
}

.chat-title {
  font-size: 13px;
  color: #ccc;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}
.chat-item.active .chat-title {
  color: #f0f0f0;
}

.delete-btn {
  background: none;
  border: none;
  color: #555;
  cursor: pointer;
  font-size: 11px;
  padding: 2px 4px;
  border-radius: 4px;
  opacity: 0;
  transition: all 0.15s;
}
.chat-item:hover .delete-btn {
  opacity: 1;
}
.delete-btn:hover {
  color: #ff6060;
  background: #2a1010;
}

/* Main */
.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid #1e1e1e;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}
.header h1 {
  font-size: 16px;
  font-weight: 600;
  color: #ccc;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #4caf50;
  box-shadow: 0 0 6px #4caf50;
}

.messages {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.empty-state {
  margin: auto;
  text-align: center;
  color: #555;
}
.empty-icon {
  font-size: 48px;
  margin-bottom: 12px;
}
.empty-state p {
  font-size: 16px;
  margin-bottom: 6px;
}
.empty-sub {
  font-size: 12px;
  color: #444;
}

.message {
  display: flex;
}
.message.user {
  justify-content: flex-end;
}
.message.assistant {
  justify-content: flex-start;
}

.bubble {
  max-width: 80%;
  padding: 12px 16px;
  border-radius: 16px;
  font-size: 14px;
  line-height: 1.6;
}

.message.user .bubble {
  background: #1a1a2e;
  border: 1px solid #2a2a4a;
  border-bottom-right-radius: 4px;
}

.message.assistant .bubble {
  background: #1a1a1a;
  border: 1px solid #2a2a2a;
  border-bottom-left-radius: 4px;
}

.role-label {
  display: block;
  font-size: 11px;
  font-weight: 600;
  margin-bottom: 6px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.message.user .role-label {
  color: #5577cc;
}
.message.assistant .role-label {
  color: #4caf50;
}

.markdown p {
  margin-bottom: 10px;
}
.markdown p:last-child {
  margin-bottom: 0;
}
.markdown h1 {
  font-size: 18px;
  font-weight: 700;
  margin: 12px 0 6px;
  color: #f0f0f0;
}
.markdown h2 {
  font-size: 16px;
  font-weight: 700;
  margin: 10px 0 5px;
  color: #e0e0e0;
}
.markdown h3 {
  font-size: 14px;
  font-weight: 700;
  margin: 8px 0 4px;
  color: #d0d0d0;
}
.markdown ul {
  padding-left: 20px;
  margin: 6px 0 10px;
  list-style: disc;
}
.markdown ol {
  padding-left: 20px;
  margin: 6px 0 10px;
  list-style: decimal;
}
.markdown li {
  margin-bottom: 4px;
}
.markdown code {
  background: #2a2a2a;
  border: 1px solid #3a3a3a;
  border-radius: 4px;
  padding: 1px 6px;
  font-family: "Consolas", monospace;
  font-size: 13px;
  color: #e8c97a;
}
.markdown pre {
  background: #141414;
  border: 1px solid #2a2a2a;
  border-radius: 8px;
  padding: 14px;
  overflow-x: auto;
  margin: 8px 0;
}
.markdown pre code {
  background: none;
  border: none;
  padding: 0;
  color: #a8d8a8;
  font-size: 13px;
}
.markdown strong {
  color: #ffffff;
  font-weight: 600;
}
.markdown em {
  color: #aaaacc;
  font-style: italic;
}
.markdown hr {
  border: none;
  border-top: 1px solid #2a2a2a;
  margin: 12px 0;
}

.typing {
  display: flex;
  gap: 4px;
  align-items: center;
  height: 20px;
}
.typing span {
  width: 6px;
  height: 6px;
  background: #555;
  border-radius: 50%;
  animation: bounce 1.2s infinite;
}
.typing span:nth-child(2) {
  animation-delay: 0.2s;
}
.typing span:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes bounce {
  0%,
  80%,
  100% {
    transform: translateY(0);
  }
  40% {
    transform: translateY(-6px);
  }
}

.error-bar {
  margin: 0 20px;
  padding: 10px 14px;
  background: #2a1010;
  border: 1px solid #5a2020;
  border-radius: 8px;
  font-size: 13px;
  color: #ff8080;
}

.input-area {
  display: flex;
  align-items: flex-end;
  gap: 10px;
  padding: 16px 20px;
  border-top: 1px solid #1e1e1e;
}

.input {
  flex: 1;
  background: #1a1a1a;
  border: 1px solid #2a2a2a;
  border-radius: 10px;
  color: #f0f0f0;
  font-size: 14px;
  padding: 12px 14px;
  resize: none;
  outline: none;
  font-family: inherit;
  line-height: 1.5;
  transition: border-color 0.2s;
  min-height: 44px;
  max-height: 160px;
}
.input:focus {
  border-color: #444;
}
.input:disabled {
  opacity: 0.5;
}

.send-btn {
  background: #f0f0f0;
  color: #0d0d0d;
  border: none;
  border-radius: 10px;
  width: 44px;
  height: 44px;
  font-size: 18px;
  cursor: pointer;
  transition: opacity 0.2s;
  flex-shrink: 0;
}
.send-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}
.send-btn:hover:not(:disabled) {
  opacity: 0.85;
}

.hint {
  text-align: center;
  font-size: 11px;
  color: #333;
  padding: 6px 0 10px;
}
</style>
