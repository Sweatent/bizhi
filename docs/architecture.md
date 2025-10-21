# 架构设计细化

本文档补充 README 中的总体方案，按模块拆解后端 Rust 宿主、Tauri 2 UI、IPC 协议以及能耗策略，实现动态壁纸时的关键注意事项。

## 1. 宿主进程（Rust）

### 1.1 窗口与桌面层级

- 调用 `FindWindowW("Progman", None)` 获取桌面进程句柄。
- 发送 `0x052C` 自定义消息触发 WorkerW 重构。
- 枚举窗口查找包含 `SHELLDLL_DefView` 的 WorkerW，隐藏空 WorkerW。
- 使用 `SetParent` 将自定义隐藏窗口挂到目标 WorkerW/Progman，从而位于桌面图标背后。
- 监听虚拟桌面切换广播（`WM_SETTINGCHANGE` + `"VirtualDesktop"`）以便重新绑定父窗口。

### 1.2 图形管线

1. `CreateDXGIFactory2` → `IDXGIFactory6::EnumAdapterByGpuPreference` 选择设备。
2. `D3D11CreateDevice` 时启用 `D3D11_CREATE_DEVICE_VIDEO_SUPPORT` 与调试/单线程等必要标志。
3. `IMFDXGIDeviceManager::ResetDevice` 绑定 D3D11 设备给 Media Foundation。
4. 使用 `IMFMediaEngine` 或 Source Reader + 视频处理管线：
   - 通过 `MFCreateMediaEngine` 获得 `IMFMediaEngineEx`。
   - 接收 `MF_MEDIA_ENGINE_EVENT_*` 事件（缓冲、播放、结束）。
   - 解码输出 NV12/P010 表面，交给 `ID3D11VideoProcessor`。
5. 交换链选用 `DXGI_SWAP_CHAIN_DESC1`，设置 `BufferCount = 2`、`Format = DXGI_FORMAT_B8G8R8A8_UNORM`，启用 `DXGI_SCALING_NONE` 以便手动控制缩放。

### 1.3 视频处理

- `ID3D11VideoProcessorEnumerator` 根据输入/输出尺寸生成能力描述。
- `ID3D11VideoContext::VideoProcessorBlt` 实现缩放、色彩空间转换以及裁切模式。
- 支持的三种模式（Stretch/Cover/Contain）通过设定目标矩形实现。

### 1.4 事件与降耗

- 通过 `RegisterPowerSettingNotification` 订阅以下 GUID：
  - `GUID_CONSOLE_DISPLAY_STATE`
  - `GUID_MONITOR_POWER_ON`
  - `GUID_ACDC_POWER_SOURCE`
  - `GUID_BATTERY_PERCENTAGE_REMAINING`
- 当收到显示器关闭或转入电池等事件时：
  - 暂停 `IMFMediaEngine` 播放。
  - 释放或降低交换链刷新频率。
- `IDXGISwapChain::Present` 返回 `DXGI_STATUS_OCCLUDED` 时进入节流状态（延迟解码、降低帧率）。
- 背景线程调用 `SetThreadInformation` 以启用 `THREAD_POWER_THROTTLING_EXECUTION_SPEED`（EcoQoS）。

## 2. Tauri 2 UI

- 仅提供系统托盘、菜单、状态面板与文件对话框。
- Rust 侧通过 `tauri::Builder` 初始化 `TrayIconBuilder`，为菜单项绑定 `on_menu_event`。
- 前端可选使用 `@tauri-apps/api/tray` 与 `@tauri-apps/api/menu` 更新选中状态。
- `@tauri-apps/plugin-dialog` 实现「选择新视频」对话框，结果通过 `invoke` 发送到 Rust。

菜单结构示例：

```
Tray
├── 暂停/继续（根据状态切换标题）
├── 选择新的视频...
├── 视频模式
│   ├── 拉伸
│   ├── 裁切
│   └── 适应
└── 退出
```

## 3. IPC 协议

- `HostCommand`：UI → 宿主，包含 `LoadVideo`、`TogglePause`、`SetScaling`、`SetGpuPreference`、`Exit` 等。
- `UiEvent`：宿主 → UI，包含播放状态、错误信息以及配置快照。
- `ipc` crate 使用 `serde` 序列化，`thiserror` 定义错误类型，保证消息兼容性。
- 命令传递方式：Tauri 的 `invoke` 与自定义事件（`app.emit_all`）。

## 4. 能耗策略

| 状态                     | 策略                                                         |
|--------------------------|--------------------------------------------------------------|
| 屏幕关闭/熄灭            | 暂停解码器、停止 Present，降低线程 QoS。                    |
| 窗口被遮挡 (`DXGI_STATUS_OCCLUDED`) | 降低帧率、必要时暂停。                                     |
| 切换到电池供电           | 改用 `DXGI_GPU_PREFERENCE_MINIMUM_POWER`，降低分辨率/帧率。 |
| 电量低阈值               | 停止播放或提示用户，保持静态壁纸。                          |
| dGPU 可用且接通电源      | 切换至高性能适配器，确保播放流畅同时减轻 CPU 负担。       |

## 5. 多显示器

- 启动时枚举所有显示器，为每个显示器创建独立交换链或视口。
- 在 DPI 变化时重新计算视频模式矩形。
- 后续可支持不同显示器加载不同视频，或共享同步帧。

## 6. 自动化构建

GitHub Actions 工作流 `windows-build.yml` 负责：

1. 触发条件：`push`、`pull_request`、手动 `workflow_dispatch`。
2. 环境：`windows-latest`。
3. 步骤：
   - 安装 Rust 稳定版与缓存依赖。
   - 执行 `cargo build --workspace --release`。
   - 使用 `actions/upload-artifact` 上传编译产物（包含 `wallpaper_host.exe`）。

随着项目演进，可在同一工作流追加 `tauri-apps/tauri-action` 生成 `.msi` 或 `.exe` 安装包。

## 7. 后续工作

1. 在 `wallpaper_host` 中实现 Win32 消息循环、窗口创建、DXGI/Media Foundation 管线。
2. 搭建 `app` 目录并初始化 Tauri 2 项目，连接现有 IPC crate。
3. 编写端到端测试：
   - 单元测试几何计算。
   - 集成测试验证命令/事件序列。
4. 扩展 GitHub Actions：
   - 增加 `tauri-action` 生成安装包。
   - 发布工件到 GitHub Releases。

