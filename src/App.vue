<template>
  <div class="app">
    <!-- Header -->
    <div class="header">
      <div class="header-left">
        <div class="status-dot" :class="{ active: isReady }"></div>
        <h1>Local Chat</h1>
      </div>
      <button class="clear-btn" @click="clearChat" title="Clear chat">↺</button>
    </div>

    <!-- Messages -->
    <div class="messages" ref="messagesEl">
      <div v-if="messages.length === 0" class="empty-state">
        <div class="empty-icon">🤖</div>
        <p>Ask me anything!</p>
        <p class="empty-sub">Powered by Qwen3 — running locally on your device</p>
      </div>

      <div
        v-for="(msg, i) in messages"
        :key="i"
        class="message"
        :class="msg.role"
      >
        <div class="bubble">
          <span class="role-label">{{ msg.role === 'user' ? 'You' : 'AI' }}</span>
          <p>{{ msg.content }}</p>
        </div>
      </div>

      <div v-if="loading" class="message assistant">
        <div class="bubble">
          <span class="role-label">AI</span>
          <div class="typing">
            <span></span><span></span><span></span>
          </div>
        </div>
      </div>
    </div>

    <!-- Error -->
    <div v-if="error" class="error-bar">
      ⚠️ {{ error }}
    </div>

    <!-- Input -->
    <div class="input-area">
      <button
        class="mic-btn"
        :class="{ listening: isListening }"
        @click="toggleVoice"
        title="Voice input"
      >
        {{ isListening ? '🔴' : '🎤' }}
      </button>

      <textarea
        v-model="input"
        class="input"
        placeholder="Type a message or click 🎤 to speak..."
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

    <p class="hint">Enter to send · Shift+Enter for new line · 🎤 for voice</p>
  </div>
</template>

<script setup lang="ts">
import { ref, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface Message {
  role: 'user' | 'assistant'
  content: string
}

const messages = ref<Message[]>([])
const input = ref('')
const loading = ref(false)
const error = ref('')
const isReady = ref(true)
const isListening = ref(false)
const messagesEl = ref<HTMLElement>()
const inputEl = ref<HTMLTextAreaElement>()

let recognition: any = null

// --- Voice input setup ---
function setupVoice() {
  const SpeechRecognition =
    (window as any).SpeechRecognition ||
    (window as any).webkitSpeechRecognition

  if (!SpeechRecognition) {
    error.value = 'Voice input not supported in this browser'
    return
  }

  recognition = new SpeechRecognition()
  recognition.continuous = false
  recognition.interimResults = true
  recognition.lang = 'en-US'

  recognition.onresult = (event: any) => {
    const transcript = Array.from(event.results)
      .map((r: any) => r[0].transcript)
      .join('')
    input.value = transcript
  }

  recognition.onend = () => {
    isListening.value = false
    if (input.value.trim()) sendMessage()
  }

  recognition.onerror = () => {
    isListening.value = false
    error.value = 'Voice input error. Try again.'
  }
}

function toggleVoice() {
  if (!recognition) setupVoice()
  if (!recognition) return

  if (isListening.value) {
    recognition.stop()
    isListening.value = false
  } else {
    error.value = ''
    input.value = ''
    recognition.start()
    isListening.value = true
  }
}

// --- Chat ---
async function sendMessage() {
  const text = input.value.trim()
  if (!text || loading.value) return

  error.value = ''
  input.value = ''
  resetResize()

  messages.value.push({ role: 'user', content: text })
  scrollToBottom()

  loading.value = true

  try {
    const reply = await invoke<string>('chat', {
      messages: messages.value
    })

    messages.value.push({ role: 'assistant', content: reply })
    scrollToBottom()

    // Speak the reply
    speakText(reply)

  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
    nextTick(() => inputEl.value?.focus())
  }
}

// --- Text to Speech ---
function speakText(text: string) {
  if (!window.speechSynthesis) return
  window.speechSynthesis.cancel()
  const utterance = new SpeechSynthesisUtterance(text)
  utterance.rate = 1.0
  utterance.pitch = 1.0
  utterance.volume = 1.0
  window.speechSynthesis.speak(utterance)
}

// --- Helpers ---
function clearChat() {
  messages.value = []
  error.value = ''
  window.speechSynthesis?.cancel()
}

async function scrollToBottom() {
  await nextTick()
  if (messagesEl.value) {
    messagesEl.value.scrollTop = messagesEl.value.scrollHeight
  }
}

function autoResize(e: Event) {
  const el = e.target as HTMLTextAreaElement
  el.style.height = 'auto'
  el.style.height = Math.min(el.scrollHeight, 160) + 'px'
}

function resetResize() {
  if (inputEl.value) inputEl.value.style.height = 'auto'
}
</script>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }

body {
  font-family: system-ui, -apple-system, sans-serif;
  background: #0d0d0d;
  color: #f0f0f0;
  height: 100vh;
  overflow: hidden;
}

.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  max-width: 760px;
  margin: 0 auto;
}

/* Header */
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
  font-size: 18px;
  font-weight: 600;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #444;
}

.status-dot.active {
  background: #4caf50;
  box-shadow: 0 0 6px #4caf50;
}

.clear-btn {
  background: none;
  border: 1px solid #2a2a2a;
  color: #888;
  padding: 6px 12px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 16px;
  transition: all 0.2s;
}

.clear-btn:hover { color: #f0f0f0; border-color: #444; }

/* Messages */
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

.empty-icon { font-size: 48px; margin-bottom: 12px; }

.empty-state p { font-size: 16px; margin-bottom: 6px; }

.empty-sub { font-size: 12px; color: #444; }

.message {
  display: flex;
}

.message.user { justify-content: flex-end; }
.message.assistant { justify-content: flex-start; }

.bubble {
  max-width: 75%;
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
  color: #666;
  margin-bottom: 4px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.message.user .role-label { color: #5577cc; }
.message.assistant .role-label { color: #4caf50; }

/* Typing indicator */
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

.typing span:nth-child(2) { animation-delay: 0.2s; }
.typing span:nth-child(3) { animation-delay: 0.4s; }

@keyframes bounce {
  0%, 80%, 100% { transform: translateY(0); }
  40% { transform: translateY(-6px); }
}

/* Error */
.error-bar {
  margin: 0 20px;
  padding: 10px 14px;
  background: #2a1010;
  border: 1px solid #5a2020;
  border-radius: 8px;
  font-size: 13px;
  color: #ff8080;
}

/* Input area */
.input-area {
  display: flex;
  align-items: flex-end;
  gap: 10px;
  padding: 16px 20px;
  border-top: 1px solid #1e1e1e;
}

.mic-btn {
  background: #1a1a1a;
  border: 1px solid #2a2a2a;
  border-radius: 10px;
  width: 44px;
  height: 44px;
  font-size: 18px;
  cursor: pointer;
  transition: all 0.2s;
  flex-shrink: 0;
}

.mic-btn:hover { border-color: #444; }
.mic-btn.listening {
  background: #2a1010;
  border-color: #cc4444;
  animation: pulse 1s infinite;
}

@keyframes pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(204,68,68,0.4); }
  50% { box-shadow: 0 0 0 6px rgba(204,68,68,0); }
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

.input:focus { border-color: #444; }
.input:disabled { opacity: 0.5; }

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

.send-btn:disabled { opacity: 0.3; cursor: not-allowed; }
.send-btn:hover:not(:disabled) { opacity: 0.85; }

.hint {
  text-align: center;
  font-size: 11px;
  color: #333;
  padding: 6px 0 10px;
}
</style>