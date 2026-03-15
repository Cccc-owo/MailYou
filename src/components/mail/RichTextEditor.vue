<template>
  <div class="rich-text-editor">
    <div v-if="editor" class="rich-text-editor__toolbar d-flex flex-wrap align-center ga-1 pa-1">
      <!-- Clear formatting -->
      <v-btn
        size="x-small"
        variant="text"
        icon="mdi-format-clear"
        :title="t('composer.toolbar.clearFormatting')"
        @click="editor.chain().focus().clearNodes().unsetAllMarks().run()"
      />

      <v-divider vertical class="mx-1" />

      <!-- Bold / Italic / Underline / Strikethrough -->
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('bold') ? 'primary' : undefined"
        icon="mdi-format-bold"
        :title="t('composer.toolbar.bold')"
        @click="editor.chain().focus().toggleBold().run()"
      />
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('italic') ? 'primary' : undefined"
        icon="mdi-format-italic"
        :title="t('composer.toolbar.italic')"
        @click="editor.chain().focus().toggleItalic().run()"
      />
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('underline') ? 'primary' : undefined"
        icon="mdi-format-underline"
        :title="t('composer.toolbar.underline')"
        @click="editor.chain().focus().toggleUnderline().run()"
      />
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('strike') ? 'primary' : undefined"
        icon="mdi-format-strikethrough"
        :title="t('composer.toolbar.strikethrough')"
        @click="editor.chain().focus().toggleStrike().run()"
      />

      <v-divider vertical class="mx-1" />

      <!-- Text color -->
      <v-menu :close-on-content-click="false">
        <template #activator="{ props: menuProps }">
          <div class="toolbar-color-btn" :title="t('composer.toolbar.textColor')">
            <v-btn
              v-bind="menuProps"
              size="x-small"
              variant="text"
              icon="mdi-format-color-text"
            />
            <span class="toolbar-color-indicator" :style="{ background: activeTextColor }" />
          </div>
        </template>
        <div class="color-palette pa-2">
          <div
            v-for="color in textColors"
            :key="color"
            class="color-swatch"
            :style="{ background: color }"
            @click="setTextColor(color)"
          />
          <div
            class="color-swatch color-swatch--reset"
            :title="t('composer.toolbar.resetColor')"
            @click="editor.chain().focus().unsetColor().run()"
          >
            <v-icon size="14" icon="mdi-close" />
          </div>
        </div>
      </v-menu>

      <!-- Highlight color -->
      <v-menu :close-on-content-click="false">
        <template #activator="{ props: menuProps }">
          <div class="toolbar-color-btn" :title="t('composer.toolbar.highlight')">
            <v-btn
              v-bind="menuProps"
              size="x-small"
              variant="text"
              icon="mdi-marker"
            />
            <span class="toolbar-color-indicator" :style="{ background: activeHighlightColor }" />
          </div>
        </template>
        <div class="color-palette pa-2">
          <div
            v-for="color in highlightColors"
            :key="color"
            class="color-swatch"
            :style="{ background: color }"
            @click="setHighlight(color)"
          />
          <div
            class="color-swatch color-swatch--reset"
            :title="t('composer.toolbar.resetColor')"
            @click="editor.chain().focus().unsetHighlight().run()"
          >
            <v-icon size="14" icon="mdi-close" />
          </div>
        </div>
      </v-menu>

      <v-divider vertical class="mx-1" />

      <!-- Headings -->
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('heading', { level: 1 }) ? 'primary' : undefined"
        icon="mdi-format-header-1"
        :title="t('composer.toolbar.heading1')"
        @click="editor.chain().focus().toggleHeading({ level: 1 }).run()"
      />
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('heading', { level: 2 }) ? 'primary' : undefined"
        icon="mdi-format-header-2"
        :title="t('composer.toolbar.heading2')"
        @click="editor.chain().focus().toggleHeading({ level: 2 }).run()"
      />
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('heading', { level: 3 }) ? 'primary' : undefined"
        icon="mdi-format-header-3"
        :title="t('composer.toolbar.heading3')"
        @click="editor.chain().focus().toggleHeading({ level: 3 }).run()"
      />

      <v-divider vertical class="mx-1" />

      <!-- Lists -->
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('bulletList') ? 'primary' : undefined"
        icon="mdi-format-list-bulleted"
        :title="t('composer.toolbar.bulletList')"
        @click="editor.chain().focus().toggleBulletList().run()"
      />
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('orderedList') ? 'primary' : undefined"
        icon="mdi-format-list-numbered"
        :title="t('composer.toolbar.orderedList')"
        @click="editor.chain().focus().toggleOrderedList().run()"
      />

      <v-divider vertical class="mx-1" />

      <!-- Alignment -->
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive({ textAlign: 'left' }) ? 'primary' : undefined"
        icon="mdi-format-align-left"
        :title="t('composer.toolbar.alignLeft')"
        @click="editor.chain().focus().setTextAlign('left').run()"
      />
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive({ textAlign: 'center' }) ? 'primary' : undefined"
        icon="mdi-format-align-center"
        :title="t('composer.toolbar.alignCenter')"
        @click="editor.chain().focus().setTextAlign('center').run()"
      />
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive({ textAlign: 'right' }) ? 'primary' : undefined"
        icon="mdi-format-align-right"
        :title="t('composer.toolbar.alignRight')"
        @click="editor.chain().focus().setTextAlign('right').run()"
      />

      <v-divider vertical class="mx-1" />

      <!-- Blockquote / Link / Code / Horizontal Rule -->
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('blockquote') ? 'primary' : undefined"
        icon="mdi-format-quote-close"
        :title="t('composer.toolbar.blockquote')"
        @click="editor.chain().focus().toggleBlockquote().run()"
      />
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('link') ? 'primary' : undefined"
        icon="mdi-link-variant"
        :title="t('composer.toolbar.link')"
        @click="toggleLink"
      />
      <v-btn
        size="x-small"
        variant="text"
        :color="editor.isActive('codeBlock') ? 'primary' : undefined"
        icon="mdi-code-tags"
        :title="t('composer.toolbar.code')"
        @click="editor.chain().focus().toggleCodeBlock().run()"
      />
      <v-btn
        size="x-small"
        variant="text"
        icon="mdi-minus"
        :title="t('composer.toolbar.horizontalRule')"
        @click="editor.chain().focus().setHorizontalRule().run()"
      />

      <v-divider vertical class="mx-1" />

      <!-- Undo / Redo -->
      <v-btn
        size="x-small"
        variant="text"
        icon="mdi-undo"
        :disabled="!editor.can().undo()"
        :title="t('composer.toolbar.undo')"
        @click="editor.chain().focus().undo().run()"
      />
      <v-btn
        size="x-small"
        variant="text"
        icon="mdi-redo"
        :disabled="!editor.can().redo()"
        :title="t('composer.toolbar.redo')"
        @click="editor.chain().focus().redo().run()"
      />
    </div>

    <EditorContent :editor="editor" class="rich-text-editor__content" />
  </div>
</template>

<script setup lang="ts">
import { watch, computed, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import { useEditor, EditorContent } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import Underline from '@tiptap/extension-underline'
import Link from '@tiptap/extension-link'
import Image from '@tiptap/extension-image'
import Placeholder from '@tiptap/extension-placeholder'
import { TextStyle } from '@tiptap/extension-text-style'
import Color from '@tiptap/extension-color'
import Highlight from '@tiptap/extension-highlight'
import TextAlign from '@tiptap/extension-text-align'

const { t } = useI18n()

const props = defineProps<{
  modelValue: string
  placeholder?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const textColors = [
  '#000000', '#434343', '#666666', '#999999',
  '#E03131', '#E8590C', '#F08C00', '#2B8A3E',
  '#1971C2', '#6741D9', '#C2255C', '#0C8599',
]

const highlightColors = [
  '#FFF3BF', '#FFE8CC', '#FFD8D8', '#D3F9D8',
  '#D0EBFF', '#E5DBFF', '#FFE0F0', '#C3FAE8',
  '#FFF9DB', '#FFF0F0', '#F3D9FA', '#DBE4FF',
]

const activeTextColor = computed(() => {
  if (!editor.value) return '#E03131'
  return (editor.value.getAttributes('textStyle').color as string) || '#E03131'
})

const activeHighlightColor = computed(() => {
  if (!editor.value) return '#FFF3BF'
  return (editor.value.getAttributes('highlight').color as string) || '#FFF3BF'
})

const editor = useEditor({
  content: props.modelValue,
  extensions: [
    StarterKit,
    Underline,
    Link.configure({ openOnClick: false }),
    Image,
    Placeholder.configure({
      placeholder: props.placeholder ?? '',
    }),
    TextStyle,
    Color,
    Highlight.configure({ multicolor: true }),
    TextAlign.configure({ types: ['heading', 'paragraph'] }),
  ],
  onUpdate: ({ editor }) => {
    emit('update:modelValue', editor.getHTML())
  },
})

watch(
  () => props.modelValue,
  (val) => {
    if (editor.value && editor.value.getHTML() !== val) {
      editor.value.commands.setContent(val, { emitUpdate: false })
    }
  },
)

onBeforeUnmount(() => {
  editor.value?.destroy()
})

const toggleLink = () => {
  if (!editor.value) return
  if (editor.value.isActive('link')) {
    editor.value.chain().focus().unsetLink().run()
    return
  }
  const url = window.prompt('URL')
  if (url) {
    editor.value.chain().focus().setLink({ href: url }).run()
  }
}

const setTextColor = (color: string) => {
  editor.value?.chain().focus().setColor(color).run()
}

const setHighlight = (color: string) => {
  editor.value?.chain().focus().setHighlight({ color }).run()
}
</script>

<style scoped>
.rich-text-editor {
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 8px;
  overflow: hidden;
}

.rich-text-editor__toolbar {
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  background: rgb(var(--v-theme-surface));
}

.rich-text-editor__content {
  min-height: 260px;
  max-height: 400px;
  overflow-y: auto;
  padding: 12px 16px;
}

.rich-text-editor__content :deep(.tiptap) {
  outline: none;
  min-height: 236px;
}

.rich-text-editor__content :deep(.tiptap p.is-editor-empty:first-child::before) {
  content: attr(data-placeholder);
  float: left;
  color: rgba(var(--v-theme-on-surface), 0.4);
  pointer-events: none;
  height: 0;
}

.rich-text-editor__content :deep(.tiptap blockquote) {
  border-left: 3px solid rgba(var(--v-border-color), var(--v-border-opacity));
  padding-left: 12px;
  margin-left: 0;
  color: rgba(var(--v-theme-on-surface), 0.7);
}

.rich-text-editor__content :deep(.tiptap pre) {
  background: rgba(var(--v-theme-on-surface), 0.05);
  border-radius: 4px;
  padding: 8px 12px;
  font-family: monospace;
}

.rich-text-editor__content :deep(.tiptap a) {
  color: rgb(var(--v-theme-primary));
}

.rich-text-editor__content :deep(.tiptap hr) {
  border: none;
  border-top: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  margin: 12px 0;
}

.rich-text-editor__content :deep(.tiptap mark) {
  border-radius: 2px;
  padding: 0 2px;
}

/* Color button with indicator bar */
.toolbar-color-btn {
  position: relative;
  display: inline-flex;
}

.toolbar-color-indicator {
  position: absolute;
  bottom: 2px;
  left: 50%;
  transform: translateX(-50%);
  width: 16px;
  height: 3px;
  border-radius: 1px;
  pointer-events: none;
}

/* Color palette popup */
.color-palette {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 4px;
  background: rgb(var(--v-theme-surface));
  border-radius: 8px;
}

.color-swatch {
  width: 24px;
  height: 24px;
  border-radius: 4px;
  cursor: pointer;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  transition: transform 0.1s;
}

.color-swatch:hover {
  transform: scale(1.15);
}

.color-swatch--reset {
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgb(var(--v-theme-surface)) !important;
}
</style>
