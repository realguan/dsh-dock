// yaml-editor.tsx —— YAML 配置编辑面共享原语（CodeMirror 6）。
// 2026-09-18 裁定：配置编辑体验对齐专业编辑器（行号 / 折叠 / 高亮 / ⌘F 查找），
// CodeMirror 6 回写 AGENTS.md §4.4.1 依赖白名单。主题色全部走 @theme 的 term-*
// token（编辑器面恒为终端暗色，与启动日志终端同一视觉族），禁硬编码 hex。
import { useEffect, useRef } from "react"
import { EditorState } from "@codemirror/state"
import { EditorView, keymap, placeholder } from "@codemirror/view"
import { basicSetup } from "codemirror"
import { indentWithTab } from "@codemirror/commands"
import { yaml } from "@codemirror/lang-yaml"
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language"
import { tags as tag } from "@lezer/highlight"
import { cn } from "@/lib/utils"

const LINE_HEIGHT = 1.625
const FONT_SIZE = 12

const termTheme = EditorView.theme(
  {
    "&": {
      color: "var(--color-term-ink)",
      fontSize: `${FONT_SIZE}px`,
      fontFamily: "var(--font-mono)",
    },
    ".cm-scroller": {
      lineHeight: String(LINE_HEIGHT),
      padding: "8px 0",
    },
    ".cm-content": {
      caretColor: "var(--color-term-brand)",
      padding: "0 12px 0 0",
    },
    ".cm-gutters": {
      backgroundColor: "transparent",
      color: "var(--color-term-faint)",
      border: "none",
    },
    ".cm-activeLine": {
      backgroundColor: "color-mix(in srgb, var(--color-term-brand) 7%, transparent)",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "color-mix(in srgb, var(--color-term-brand) 10%, transparent)",
      color: "var(--color-term-dim)",
    },
    ".cm-foldPlaceholder": {
      backgroundColor: "var(--color-term-line)",
      border: "none",
      color: "var(--color-term-dim)",
      borderRadius: "4px",
    },
    ".cm-panels": {
      backgroundColor: "var(--color-term-panel)",
      color: "var(--color-term-ink)",
      borderColor: "var(--color-term-line)",
    },
    ".cm-panel.cm-search input, .cm-panel.cm-search textarea": {
      backgroundColor: "var(--color-term)",
      color: "var(--color-term-ink)",
      borderColor: "var(--color-term-line)",
      fontFamily: "var(--font-mono)",
    },
    ".cm-panel.cm-search button": {
      color: "var(--color-term-ink)",
    },
    ".cm-cursor": {
      borderLeftColor: "var(--color-term-brand)",
    },
  },
  { dark: true },
)

// YAML 语义配色：键 = 品牌蓝，字符串 = ok 绿，注释 = faint 斜体。
// 2026-09-18 实测（浏览器验证）：裸数字/布尔在 lang-yaml 6.1 里**不带 token**
// （lezer 只在引号/流式上下文产标量标签），不为其虚构样式规则。
const yamlHighlight = HighlightStyle.define([
  { tag: tag.comment, color: "var(--color-term-faint)", fontStyle: "italic" },
  { tag: tag.propertyName, color: "var(--color-term-brand)" },
  { tag: tag.string, color: "var(--color-term-ok)" },
  { tag: tag.variableName, color: "var(--color-term-danger)" },
  { tag: tag.separator, color: "var(--color-term-dim)" },
])

export function YamlEditor({
  value,
  onChange,
  placeholder: ph,
  rows = 16,
  maxRows,
  readOnly = false,
  className,
}: {
  value: string
  onChange?: (next: string) => void
  placeholder?: string
  /** 初始可视行数（与旧 textarea 口径一致），超出后编辑器内部滚动。 */
  rows?: number
  /** 高度封顶行数（只读长文视窗用，如旧 Patch YAML 视窗的 420px 上限口径）。 */
  maxRows?: number
  /** 只读视窗（如 Patch YAML 展示）：保留高亮/行号/折叠/查找，禁编辑与改光标。 */
  readOnly?: boolean
  className?: string
}) {
  const hostRef = useRef<HTMLDivElement>(null)
  const viewRef = useRef<EditorView | null>(null)
  const onChangeRef = useRef(onChange)
  onChangeRef.current = onChange
  // 挂载只吃首帧 doc（受控同步走下面的 effect）；用 ref 避开陈旧闭包歧义。
  const initialValueRef = useRef(value)

  useEffect(() => {
    const host = hostRef.current
    if (!host) return
    const state = EditorState.create({
      doc: initialValueRef.current,
      extensions: [
        basicSetup,
        // basicSetup 依无障碍惯例把 Tab 留给「移动焦点」，编辑器内按 Tab 会跳走
        // （维护者实证）。显式补 indentWithTab 让 Tab 缩进、Shift-Tab 反缩进。
        keymap.of([indentWithTab]),
        yaml(),
        EditorView.lineWrapping,
        EditorView.darkTheme.of(true),
        termTheme,
        maxRows
          ? EditorView.theme(
              {
                "&": { maxHeight: `${maxRows * FONT_SIZE * LINE_HEIGHT + 16}px` },
                ".cm-scroller": { overflowY: "auto" },
              },
              { dark: true },
            )
          : [],
        syntaxHighlighting(yamlHighlight),
        EditorView.editable.of(!readOnly),
        // 刻意**不用** `EditorState.readOnly`：它连程序化 dispatch 一起拦，
        // 只读视窗换 profile / 重读时外部新内容会同步不进去（表现为陈旧原文）。
        ph ? placeholder(ph) : [],
        EditorView.updateListener.of((update) => {
          if (update.docChanged) onChangeRef.current?.(update.state.doc.toString())
        }),
      ],
    })
    const view = new EditorView({ state, parent: host })
    viewRef.current = view
    return () => {
      view.destroy()
      viewRef.current = null
    }
    // ph / readOnly 仅挂载期生效：占位与可编辑性随视图挂载即定，切换语言不重建编辑器。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // 外部改写（重新读取 / 复制源）同步进编辑器；用户输入的回环由等值比较截断。
  useEffect(() => {
    const view = viewRef.current
    if (!view) return
    const current = view.state.doc.toString()
    if (value === current) return
    view.dispatch({
      changes: { from: 0, to: current.length, insert: value },
    })
  }, [value])

  const minHeight = rows * FONT_SIZE * LINE_HEIGHT + 16

  return (
    <div
      ref={hostRef}
      style={{ minHeight }}
      className={cn(
        "overflow-hidden rounded-xl border border-term-line bg-term",
        "focus-within:border-term-brand",
        className,
      )}
    />
  )
}
