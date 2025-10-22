# Bizhi — Windows 动态壁纸系统技术方案

本仓库用于沉淀 Windows 10/11 平台视频壁纸解决方案的总体架构、关键 API 选型以及自动化交付脚手架。方案目标是以 **最低能耗** 实现全屏视频壁纸，同时能够自适应仅 iGPU 的设备或具备 dGPU 的高性能形态，并通过 **Tauri 2.0** 提供系统托盘交互界面。

## 总体目标

- 以视频作为桌面背景播放，同时保持桌面交互可用。
- 托盘菜单支持「暂停/继续、选择新视频、视频模式（拉伸/裁切/适应）、退出」。
- 在仅有集显时最大限度降低显存与内存占用；在具备独显时，优先使用 dGPU 以降低 CPU 占用与功耗。
- 当屏幕关闭、窗口被遮挡或系统空闲时自动降耗或暂停。

## 架构总览

```
┌─────────────────────────────┐
│           Tauri 2 UI         │
│  • 托盘菜单、对话框、状态展示 │
│  • @tauri-apps/api + IPC     │
└───────▲───────────────┬──────┘
        │invoke / events │
┌───────┴───────────────▼──────┐
│      Wallpaper Host (Rust)   │
│  • Progman/WorkerW 桌面挂载   │
│  • DXGI + D3D11 + MF 零拷贝   │
│  • 电源与遮挡状态策略        │
└──────────────▲───────────────┘
               │
      Windows 桌面图层/显示设备
```

仓库中预先创建了 `common`、`ipc`、`wallpaper_host` 三个 crate，为后续实现提供类型与依赖骨架：

- `common`：运行时配置、枚举、日志工具等跨进程共享的轻量模块。
- `ipc`：Tauri ↔ Rust 宿主间的命令与事件协议定义。
- `wallpaper_host`：Win32 原生宿主入口，将负责 DXGI/Media Foundation 管线与系统策略。

## 核心技术要点

### 桌面挂载（Progman/WorkerW 技术）

1. 使用 `FindWindow("Progman", ..)` 获取桌面窗口。
2. 向 Progman 发送 `0x052C` 消息以拆分出新的 WorkerW 层。
3. 枚举包含 `SHELLDLL_DefView` 的 WorkerW，隐藏无子窗口的 WorkerW。
4. 将自定义渲染窗口 `SetParent` 到合适的 WorkerW/Progman，使其位于桌面图标背后。

> 微软的 `IDesktopWallpaper` 仅支持静态壁纸，因此需要上述工程方案。

### 硬件解码与零拷贝

- 通过 `IDXGIFactory6::EnumAdapterByGpuPreference` 选择 iGPU/dGPU。
- 使用 `D3D11CreateDevice(D3D11_CREATE_DEVICE_VIDEO_SUPPORT)` 与 `IMFDXGIDeviceManager`。
- 采用 **Media Foundation** 硬件解码，输出 NV12/P010 DXGI 表面。
- 借助 `ID3D11VideoProcessor` 完成缩放、裁切、色彩空间转换，将结果投递到交换链零拷贝显示。

### 自适应 GPU 策略

| 场景            | 枚举策略                                     | 行为摘要                                                                 |
|-----------------|----------------------------------------------|--------------------------------------------------------------------------|
| 仅有集显        | `DXGI_GPU_PREFERENCE_MINIMUM_POWER`          | 使用 iGPU，降低呈现分辨率/帧率，优先 NV12 直通，减少内存/显存占用。       |
| 同时具备 dGPU   | `DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE`       | 使用 dGPU 承担解码/缩放，结合 EcoQoS 让 CPU 保持低功耗。                  |
| 动态/自动模式   | 根据电源状态、电池电量和遮挡情况即时切换策略 | 必要时重建设备并平滑切换，保持播放不中断。                               |

### 省电与状态联动

- 监听 `GUID_CONSOLE_DISPLAY_STATE`、`GUID_MONITOR_POWER_ON` 等电源事件。
- 当 `Present` 返回 `DXGI_STATUS_OCCLUDED` 时暂停或降低刷新率。
- 在后台线程启用 **EcoQoS** (`SetThreadInformation`)，降低 CPU 睿频。
- 根据 `GUID_ACDC_POWER_SOURCE` 与 `GUID_BATTERY_PERCENTAGE_REMAINING` 调整帧率/分辨率。

### 多显示器与 DPI

- 启动时调用 `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)`。
- 使用 `EnumDisplayMonitors` 或 `IDesktopWallpaper::GetMonitorRect` 获取显示器几何信息。
- 为每个显示器维护独立交换链，处理 `WM_DPICHANGED` 自适应缩放。

### 三种视频模式几何

设视频尺寸 `(vw, vh)`、显示尺寸 `(sw, sh)`：

- **拉伸 Stretch**：`dest = (0, 0, sw, sh)`。
- **裁切 Cover**：`s = max(sw/vw, sh/vh)`；`dw = vw * s`，`dh = vh * s`，`dx = (sw - dw)/2`，`dy = (sh - dh)/2`。
- **适应 Contain**：`s = min(sw/vw, sh/vh)`；其余同上。

这些矩形可直接交给 `ID3D11VideoProcessor`，避免自写 shader 造成额外功耗。

## 托盘交互（Tauri 2）

- Rust 侧利用 `tauri::tray::TrayIconBuilder` + `MenuBuilder` 注册菜单项。
- JS 侧可选地用 `@tauri-apps/api/tray`/`menu` 更新状态与监听事件。
- `@tauri-apps/plugin-dialog` 提供文件选择对话框用于更换视频。
- 菜单结构：暂停/继续、选择视频、视频模式（拉伸/裁切/适应）、退出。

## GitHub Actions — Windows 可执行构建

仓库包含 `.github/workflows/windows-build.yml`，在 `windows-latest` 环境执行：

1. 安装 Rust 稳定工具链。
2. 运行 `cargo build --workspace --release` 产出包含 `wallpaper_host.exe` 的构建工件。
3. 上传 `target/release` 目录供 PR 审查或发布使用。

后续集成完整的 Tauri 前端后，可扩展为 `tauri-apps/tauri-action` 打包安装程序。

## 开发里程碑

1. **壁纸宿主基础**：完成 Win32 窗口创建、Progman 挂载、交换链初始化。
2. **播放与缩放**：整合 Media Foundation 硬件解码与三种缩放模式。
3. **托盘 UI**：实现 Tauri 托盘菜单、文件对话框与 IPC 调度。
4. **能耗策略**：加入电源事件监听、遮挡检测与 EcoQoS。
5. **多屏与 DPI**：支持多显示器同步播放与动态 DPI。
6. **GPU 策略切换**：封装基于 DXGI6 的 iGPU/dGPU 优先级调整。

## 许可证

MIT
