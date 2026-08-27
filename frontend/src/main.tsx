import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { BrowserRouter } from "react-router-dom"
import "./index.css"
// 副作用 import：事件总线只在 lib/events.ts 模块加载期装配（裁定见该文件），
// 但它没有其他运行时引用者——缺这行会被整体排除出 bundle，
// 所有窗口的 boot:*/app:update 监听都不会注册。
import "./lib/events"
import App from "./App"

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </StrictMode>,
)
