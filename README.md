# LessAI

一个基于 **Tauri 2 + React + Vite** 的桌面改写工作台。

核心目标：**打开文档 → 生成修改对 → 在审阅区可追溯地应用/取消 → 导出或写回覆盖原文件**。

---

## 功能概览

- **工作台单屏**：左侧整篇文档视图，右侧审阅时间线
- **三种文档视图**：修改前 / 修改后 / 含修订标记
- **审阅区时间线**：按顺序展示所有修改对（可应用、取消、删除，持续可见）
- **断点续跑**：同一文件可恢复进度（会话保存在系统应用数据目录）
- **写回原文件（Finalize）**：将“已应用”的结果覆盖原文件，并清空该文档的全部记录
- **设置弹窗**：按分类分页，支持选择 `prompt/` 里的提示词预设

---

## 开发环境

建议版本（仅供参考）：

- Node.js 20+（仓库内提供了 `.nvmrc`）
- pnpm 10+（CI 使用 pnpm 10）
- Rust stable（配套的 Cargo）
- Tauri 依赖（Windows/macOS/Linux 按官方指引安装）

---

## 本地运行（Dev）

### Windows

1. 安装依赖：
   - `pnpm install`
2. 启动开发版：
   - `pnpm run tauri:dev`
   - 或直接双击 `start-lessai.bat`

建议先用仓库自带的 `test.txt` 做一次完整流程演练（打开文件 → 生成修改对 → 审阅 → 导出/写回）。

### 常见问题

- **`'tauri' 不是内部或外部命令`**
  - 不要全局装 tauri；请先 `pnpm install`，再用 `pnpm run tauri:dev`

- **`vite build` / `rollup` 报缺少 `@rollup/rollup-<platform>`**
  - 这通常是“不同系统共用同一份 `node_modules`”导致的（例如 Windows 装的依赖拿到 WSL/Linux 下跑）。
  - 处理方式：删除 `node_modules/` 后在当前环境重新 `pnpm install`。

---

## 构建（Build）

前端构建：

- `pnpm run build`

桌面打包（Tauri）：

- `pnpm run tauri:build`
- 或直接双击 `build-lessai.bat`

---

## 全平台打包（GitHub Actions）

仓库内已提供 GitHub Actions 工作流：`Tauri Bundles`（`.github/workflows/tauri-bundles.yml`）。

触发方式二选一：

1. **手动触发**：GitHub → Actions → `Tauri Bundles` → Run workflow
2. **打 Tag 触发**：创建并推送 `v*` tag（例如 `v0.1.0`），会自动开始三平台打包（Windows / macOS / Linux）
   - 同时会自动创建对应的 **GitHub Release**，并把各平台安装包作为 Release Assets 上传

Tag 触发示例：

```bash
git tag v0.1.0
git push origin v0.1.0
```

打包完成后：

- 产物会在对应 workflow run 的 **Artifacts** 中可下载
- 同时也会出现在 GitHub 的 **Releases** 页面（建议优先从 Release 下载）

---

## 数据存储位置

LessAI 的设置与文档会话不保存在仓库中，运行时会落到系统应用数据目录（Tauri `app_data_dir`）。

---

## License

MIT（见 `LICENSE`）。
