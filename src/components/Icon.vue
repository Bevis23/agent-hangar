<script setup lang="ts">
defineProps<{ name: string; filled?: boolean; size?: string }>();
</script>

<template>
  <svg
    viewBox="0 0 24 24"
    class="icon"
    :style="size ? { width: size, height: size } : undefined"
    :fill="filled ? 'currentColor' : 'none'"
    stroke="currentColor"
    :stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <template v-if="name === 'search'">
      <circle cx="11" cy="11" r="8" /><path d="m21 21-4.35-4.35" />
    </template>
    <template v-else-if="name === 'plus'">
      <path d="M5 12h14" /><path d="M12 5v14" />
    </template>
    <template v-else-if="name === 'folder'">
      <path
        d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"
      />
    </template>
    <template v-else-if="name === 'terminal'">
      <polyline points="4 17 10 11 4 5" /><line x1="12" y1="19" x2="20" y2="19" />
    </template>
    <template v-else-if="name === 'pencil'">
      <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
    </template>
    <template v-else-if="name === 'trash'">
      <path d="M3 6h18" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
      <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    </template>
    <template v-else-if="name === 'star'">
      <polygon
        points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
      />
    </template>
    <template v-else-if="name === 'x'">
      <path d="M18 6 6 18" /><path d="m6 6 12 12" />
    </template>
    <template v-else-if="name === 'sliders'">
      <line x1="21" y1="4" x2="14" y2="4" /><line x1="10" y1="4" x2="3" y2="4" />
      <line x1="21" y1="12" x2="12" y2="12" /><line x1="8" y1="12" x2="3" y2="12" />
      <line x1="21" y1="20" x2="16" y2="20" /><line x1="12" y1="20" x2="3" y2="20" />
      <line x1="14" y1="2" x2="14" y2="6" /><line x1="8" y1="10" x2="8" y2="14" />
      <line x1="16" y1="18" x2="16" y2="22" />
    </template>
    <template v-else-if="name === 'help'">
      <circle cx="12" cy="12" r="10" />
      <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
      <line x1="12" y1="17" x2="12.01" y2="17" />
    </template>
    <template v-else-if="name === 'code'">
      <polyline points="16 18 22 12 16 6" /><polyline points="8 6 2 12 8 18" />
    </template>
    <!-- 以下为本次扩充（Tabler 风格线性图标） -->
    <template v-else-if="name === 'play'">
      <polygon points="7 4 19 12 7 20 7 4" />
    </template>
    <template v-else-if="name === 'history'">
      <path d="M12 8v4l3 3" />
      <path d="M3.05 11a9 9 0 1 1 .5 4" /><path d="M3 4v5h5" />
    </template>
    <template v-else-if="name === 'activity'">
      <path d="M3 12h4l3 8 4-16 3 8h4" />
    </template>
    <template v-else-if="name === 'send'">
      <path d="M10 14 21 3" />
      <path d="M21 3l-6.5 18a.55.55 0 0 1-1 0L10 14l-7-3.5a.55.55 0 0 1 0-1L21 3" />
    </template>
    <template v-else-if="name === 'arrow-back'">
      <path d="M9 14 4 9l5-5" /><path d="M4 9h11a4 4 0 0 1 0 8h-1" />
    </template>
    <template v-else-if="name === 'file'">
      <path d="M14 3v4a1 1 0 0 0 1 1h4" />
      <path d="M17 21h-10a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7l5 5v11a2 2 0 0 1-2 2Z" />
    </template>
    <template v-else-if="name === 'package'">
      <path d="M12 3l8 4.5v9L12 21l-8-4.5v-9L12 3" />
      <path d="M12 12l8-4.5" /><path d="M12 12v9" /><path d="M12 12 4 7.5" />
    </template>
    <template v-else-if="name === 'bot'">
      <rect x="7" y="8" width="10" height="10" rx="2" />
      <path d="M12 8V5" /><circle cx="12" cy="4" r="1" />
      <path d="M9.5 13h.01" /><path d="M14.5 13h.01" />
    </template>
    <template v-else-if="name === 'world'">
      <circle cx="12" cy="12" r="9" />
      <path d="M3.6 9h16.8" /><path d="M3.6 15h16.8" />
      <path d="M12 3a15 15 0 0 1 0 18a15 15 0 0 1 0-18" />
    </template>
    <template v-else-if="name === 'briefcase'">
      <rect x="3" y="7" width="18" height="13" rx="2" />
      <path d="M8 7V5a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    </template>
    <template v-else-if="name === 'home'">
      <path d="M3 10l9-7 9 7" /><path d="M5 10v10h14V10" />
    </template>
    <template v-else-if="name === 'book'">
      <path d="M3 5a9 9 0 0 1 9 0v14a9 9 0 0 0-9 0V5" />
      <path d="M21 5a9 9 0 0 0-9 0v14a9 9 0 0 1 9 0V5" />
    </template>
    <template v-else-if="name === 'music'">
      <path d="M9 19V6l12-3v13" /><circle cx="6" cy="19" r="3" /><circle cx="18" cy="16" r="3" />
    </template>
    <template v-else-if="name === 'camera'">
      <path d="M5 7h2l2-2h6l2 2h2a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2Z" />
      <circle cx="12" cy="13" r="3" />
    </template>
    <template v-else-if="name === 'heart'">
      <path d="M12 20l-7-7a4.5 4.5 0 1 1 7-5a4.5 4.5 0 1 1 7 5l-7 7" />
    </template>
    <template v-else-if="name === 'flask'">
      <path d="M9 3h6" /><path d="M10 3v6l-5 8a3 3 0 0 0 3 4h8a3 3 0 0 0 3-4l-5-8V3" />
      <path d="M7.5 15h9" />
    </template>
    <template v-else-if="name === 'settings'">
      <circle cx="12" cy="12" r="3" />
      <path d="M12 3v2M12 19v2M5.6 5.6 7 7M17 17l1.4 1.4M3 12h2M19 12h2M5.6 18.4 7 17M17 7l1.4-1.4" />
    </template>
    <template v-else-if="name === 'layers'">
      <path d="M12 3l9 5-9 5-9-5 9-5" /><path d="m3 13 9 5 9-5" />
    </template>
    <template v-else>
      <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
    </template>
  </svg>
</template>

<style scoped>
.icon {
  width: 1em;
  height: 1em;
  display: block;
}
</style>
