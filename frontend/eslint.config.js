import js from "@eslint/js"
import reactHooks from "eslint-plugin-react-hooks"
import tseslint from "typescript-eslint"

// 最小集（frontend-migration §10 步骤 13）：react-hooks 两条规则一票否决；
// TS 层走 typescript-eslint recommended。dist / node_modules 不扫。
export default tseslint.config(
  { ignores: ["dist", "node_modules", "coverage"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    plugins: {
      "react-hooks": reactHooks,
    },
    rules: {
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "error",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
)
