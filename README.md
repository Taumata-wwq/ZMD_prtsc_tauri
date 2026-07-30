# 终末地俯瞰模式截图工具

基于 Tauri 2 + Vue 3 + Rust 构建的终末地游戏俯瞰模式自动截图工具。通过蛇形遍历游戏画面、模拟中键拖拽移动相机、BitBlt 截图并自动拼接，生成高分辨率的基建与大地图全景图。

- **版本**：3.1.0
- **标识符**：com.zmd.prtsc.tauri
- **平台**：仅支持 Windows 10 / 11

## 功能特性

### 自动截图

- 蛇形遍历网格（偶数行左→右，奇数行右→左），减少相机回程移动
- Win32 SendInput 模拟鼠标中键拖拽移动游戏相机，BitBlt + GetDIBits 截取客户区中心区域
- 截图前自动最小化主窗口，避免抢焦点；完成后自动恢复
- 支持随机延迟与 ±30ms 抖动，模拟人类操作节奏

### 图像拼接与导出

- 两阶段拼接：rayon 并行横向拼接每行，再串行纵向粘贴到最终大图
- 支持裁剪选区导出，JPG（质量可调）/ PNG 两种格式
- 缩略图自动生成（Lanczos3 重采样，最大宽度 400px，JPEG 质量 85），用于历史记录预览
- 导出文件名支持占位符自定义（`{region}`、`{timestamp}`、`{scrollMode}`）
- 同名文件自动追加 `_1`、`_2` 后缀避免覆盖

### 区域配置

- **基建区域**（8 个）：武陵-武陵城、武陵-景玉谷、武陵-首敦、武陵-应龙关、四号谷地-枢纽区、四号谷地-供能高地、四号谷地-谷地通道、四号谷地-源石研究园
- **大地图子区域**（14 个）：四号谷地 6 个（枢纽区、谷地通道、源石研究园、阿伯莉采石场、矿脉园区、供能高地）+ 武陵 8 个（武陵城、景玉谷、清波寨、首敦、藏剑谷、试验园区、应龙关、北部禁区）
- **自定义区域**：行列数可自由编辑，默认 2×2
- 基建截图比例 0.626 × 0.648，大地图专用比例 0.378 × 0.388
- 滚动次数 0-8 次（0 为最近地面、8 为最远视野），1-8 次参数由 0 次基准实时推导

### 数据管理

- 左右分栏布局，可拖拽分隔条调整宽度
- 基建模式：两级目录（四号谷地/武陵 → 子区域），右键支持重命名/新增/删除
- 大地图模式：一级目录（四号谷地/武陵），二级标题显示子区域数量
- 右侧编辑面板最大宽度 650px，3×3 网格展示推导数据
- 修改 target_w/target_h 后自动推导行列数、拖拽距离、重叠率
- 所有编辑操作自动保存

### 历史记录

- 按时间倒序列出所有截图会话
- 显示缩略图、区域、滚动次数、网格、状态、原图/截图路径
- 支持重新编辑（加载历史原图到截图视图继续裁剪）
- 点击路径用系统默认程序打开图像
- 「打开截图位置」按钮在文件管理器中定位
- 删除记录时可选删除原图/截图文件，焦点自动定位到首条记录

### 设置

- **主题**：明暗模式切换 + 主题色选择器
- **语言**：中英文切换
- **输出**：格式（JPG/PNG）、JPG 质量
- **延迟参数**：稳定延迟、截图间隔、拖拽时长（30-500ms 可控，默认 130/130/70ms）
- **文件夹**：原图、截图、缩略图输出目录（默认 AppData/Roaming/com.zmd.prtsc.tauri 下）
- **文件名模式**：自定义占位符组合
- **数据重置**：清空区域配置/设置/窗口状态，可选清空历史记录

### 系统集成

- **全局热键**：F3 开始/停止截图
- **窗口状态持久化**：位置、大小、最大化、置顶状态自动保存恢复（500ms 防抖）
- **UAC 提权**：release 通过 manifest 嵌入 `requireAdministrator`，dev 通过终端/IDE 继承权限；提权失败时 fallback 到 `ShellExecuteExW` relaunch
- **原生弹窗**：所有确认/提示弹窗使用软件原生实现，非 web 原生
- **路径点击打开**：通过 `opener:allow-open-path`（scope `**`）支持任意路径点击打开

## 使用方法

### 准备工作

1. 将游戏分辨率调整为 16:9（最小 1280×720）
2. 在游戏中按 X 键进入批量选择模式
3. 滚轮向上滚动，缩放至紧贴地面的高度，并移动画面到目标区域左上角
4. 截图过程中请勿移动鼠标

### 截图流程

1. 在主界面选择目标区域（如 武陵-武陵城）
2. 选择滚动次数（0 次为最近地面，8 次为最远视野）
3. 点击「开始截图」或按 F3 键启动
4. 等待完成后，在预览区拖拽选择裁剪范围
5. 点击「保存裁剪」保存图像，或「清除选区」取消当前选区

### 大地图模式

1. 在区域下拉框选择「大地图」
2. 选择子地图（四号谷地/武陵）和具体区域
3. 选择「自定义」可禁用区域选择并自由编辑行列数
4. 其余流程与基建模式一致

### 数据管理

1. 切换到「数据管理」页面
2. 左侧目录树选择基建或大地图分类
3. 右键区域可重命名、新增子项或删除
4. 右侧编辑 target_w/target_h 后自动推导其他参数并保存

## 注意事项

- **权限要求**：应用必须以管理员权限运行才能模拟输入到游戏窗口。release 构建通过 manifest 自动触发 UAC，dev 构建需以管理员身份运行终端/IDE
- **游戏窗口检测**：通过窗口标题包含 "Endfield" 识别游戏，启动前请确保游戏已打开
- **窗口最小尺寸**：800×600，默认 900×600，支持最大化与置顶
- **截图过程**：主窗口默认自动最小化让出焦点，完成后恢复；可在设置中关闭
- **数据库重置**：清理后已存在的数据库需通过「设置 → 数据重置」重建

## 数据存储

应用数据位于 `%APPDATA%\com.zmd.prtsc.tauri\`：

| 路径             | 说明                                     |
| ---------------- | ---------------------------------------- |
| `zmd.db`       | SQLite 数据库（区域配置、会话、设置）    |
| `originals/`   | 拼接后的原图                             |
| `screenshots/` | 裁剪导出的截图                           |
| `thumbnails/`  | 历史记录缩略图（{原图名}.jpg，400px 宽） |

可在设置中自定义三个文件夹路径。

---

## 开发者指南

### 环境要求

- Windows 10 / 11
- Node.js 20+
- Rust 1.78+（stable）
- 终末地游戏客户端（需以管理员权限运行）
- 构建工具链：MSVC（需 Windows SDK 的 rc.exe）或 GNU（需 MinGW-w64 的 windres）；缺失时跳过 manifest 编译，UAC 提权将不生效

### 构建

```bash
# 安装前端依赖
npm install

# 开发模式（需以管理员身份运行终端/IDE 继承权限）
npm run tauri:dev

# 类型检查
npm run typecheck

# 生产构建（输出 NSIS 安装包，启动时自动触发 UAC）
npm run tauri:build
```

### 项目结构

```
ZMD_prtsc_tauri/
├── src/                          # 前端（Vue 3 + TypeScript）
│   ├── api/                      # Tauri 命令调用封装
│   ├── components/
│   │   ├── capture/              # 截图配置面板、预览裁剪、日志面板
│   │   ├── data/                 # 数据管理编辑面板（基建/大地图）
│   │   └── titlebar/             # 标题栏（窗口控制、主题切换）
│   ├── composables/              # 组合式函数（i18n、分栏拖拽、弹窗、图像缩放、选区拖拽、自动保存等）
│   ├── i18n/locales/             # 中英文语言包
│   ├── stores/                   # Pinia 状态管理（capture/config/history/settings/ui）
│   ├── types/                    # TypeScript 类型定义
│   ├── utils/                    # 工具函数（时间、颜色、数学、区域名、路径、opener 等）
│   ├── constants.ts              # 全局常量
│   └── views/                    # 页面视图（截图、历史、数据管理、设置）
├── src-tauri/                    # 后端（Rust）
│   ├── src/
│   │   ├── commands/             # Tauri 命令（capture/config/history/image/settings）
│   │   ├── models/               # 数据模型（region/scroll_mode/session/setting/window_state）
│   │   ├── services/             # 业务服务
│   │   │   ├── auto_capture.rs   # 蛇形遍历自动截图
│   │   │   ├── capture_pipeline.rs # 截图结果处理管线
│   │   │   ├── screenshot.rs     # Win32 GDI 截图
│   │   │   ├── input.rs          # 高层输入 API
│   │   │   ├── input_win.rs      # Win32 SendInput 实现
│   │   │   ├── stitcher.rs       # 图像拼接与编码
│   │   │   ├── game_window.rs    # 游戏窗口查找与激活
│   │   │   ├── hotkey.rs         # 全局热键注册
│   │   │   ├── persistence.rs    # SQLite 持久化
│   │   │   ├── seed_data.rs      # 默认区域种子数据
│   │   │   ├── window_state.rs   # 窗口状态持久化
│   │   │   ├── shared.rs         # 共享事件与工具函数
│   │   │   └── uac.rs            # UAC 提权
│   │   ├── error.rs              # 统一错误类型
│   │   ├── lib.rs                # 库入口（命令注册、初始化）
│   │   └── main.rs               # 程序入口（日志、UAC、隐藏控制台）
│   ├── capabilities/             # Tauri 权限声明
│   ├── app.manifest              # Windows 应用清单（requireAdministrator）
│   ├── app.rc                    # Windows 资源文件
│   ├── build.rs                  # 构建脚本（manifest 嵌入）
│   ├── tauri.conf.json           # Tauri 配置
│   └── Cargo.toml
├── package.json
└── vite.config.ts
```

### 技术栈

| 层     | 技术                                        |
| ------ | ------------------------------------------- |
| 前端   | Vue 3 + TypeScript + Pinia + Vite           |
| 后端   | Rust + Tauri 2                              |
| 数据库 | SQLite（rusqlite，bundled）                 |
| 图像   | image crate + rayon 并行拼接                |
| 输入   | Windows SendInput API（中键拖拽 + 滚轮）    |
| 截图   | Win32 GDI BitBlt + GetDIBits                |
| 热键   | tauri-plugin-global-shortcut                |
| 提权   | Windows manifest + ShellExecuteExW fallback |

### Tauri 命令一览

| 命令                          | 说明                                      |
| ----------------------------- | ----------------------------------------- |
| `start_capture`             | 启动自动截图，返回 session_id             |
| `stop_capture`              | 请求停止当前截图任务                      |
| `get_preview_image`         | 拉取预览图 PNG 字节流（fallback 模式）    |
| `get_preview_path`          | 拉取预览图磁盘路径（主模式）              |
| `set_preview_path`          | 设置预览图路径（用于重新编辑）            |
| `export_image`              | 导出裁剪图像并回写 session                |
| `list_regions`              | 列出所有区域配置                          |
| `upsert_region`             | 新增或更新区域配置                        |
| `delete_region`             | 删除区域配置                              |
| `list_scroll_modes`         | 列出所有滚动模式                          |
| `derive_all_counts`         | 从 0 次基准推导所有 0-8 次参数            |
| `derive_region_from_target` | 从 target 推导指定次数的完整 RegionConfig |
| `set_setting`               | 设置单个键值                              |
| `get_all_settings`          | 获取全部设置                              |
| `set_many_settings`         | 批量设置多个键值                          |
| `reset_data`                | 重置数据（可选清空历史）                  |
| `list_sessions`             | 列出所有历史会话                          |
| `delete_session`            | 删除历史会话（可选删除原图/截图）         |

## 许可

本项目仅供学习交流使用。
