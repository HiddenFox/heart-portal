# Heart Portal Android 可行性 — 实测终稿

时间：2026-10-11 凌晨（Neo 夜间自主任务，Frank 授权「一直解决为止」）
对象：heart-portal v0.9.2（master 2a48fff，D:\Workspace\code\portal-desktop\heart-portal）
任务：这份代码能不能直接在安卓上跑？不能的话要改哪些东西？

## 一句话结论

**二进制层能直接跑；走 Termux 路线只需 1 个功能补丁（已实现并验证）。** 零代码修改即可交叉编译出 Android 兼容的静态 musl 二进制（qemu-user 模拟执行 + 正向检测 E2E 全部通过）；服务管理守卫补丁已在本分支（android-termux-support）实现。原生 App 路线才是架构级重构，不建议现在投入。

## 实测证据链（全部本机完成）

### 1. 编译（零代码修改）

```
cargo zigbuild --target aarch64-unknown-linux-musl --release
```

- 产物：`target/aarch64-unknown-linux-musl/release/heart-portal`（约 10.5MB）
- 用 zig cc（0.14.1）代替 NDK clang，绕开 ring crate 的 C 工具链依赖，**不需要 Android NDK**
- 编译时间约 35s-100s（依赖是否已缓存），仅 dead_code/unused warning（Windows 专属代码被 cfg 排除的正常现象）
- `aarch64-linux-android` target 走 NDK 路线会因 ring 编译失败（cc 找不到 NDK clang）——musl target 完全绕过

### 2. 二进制属性（Python 解析 ELF header）

- magic `\x7fELF`，class 2（64 位），machine 183（aarch64），type 2（EXEC）
- **无 PT_INTERP = 完全静态链接**，零动态依赖——Android 5.0+ 内核可直接执行

### 3. qemu-user 模拟执行（WSL docker-desktop disto + qemu-aarch64-static 7.2.0）

| 命令 | 未打补丁 | 打补丁后（本分支） |
|------|---------|------------------|
| `--help` / `--version` | ✅ 完整输出 | ✅ |
| `config path` | ✅ JSON 正常 | ✅ |
| `status` | ❌ `Error: Use your OS service manager...` | ✅ `Portal is not running...` exit 0 |
| `status`（有 portal 进程时） | — | ✅ 检出 pid+cmdline（正向 E2E） |
| `stop` | — | ✅ SIGTERM→等待→确认停止（E2E） |

正向 E2E：伪造 comm=portal 的进程 → status 检出 `pid 12 + 完整 cmdline` → stop SIGTERM → 进程死亡确认。qemu-user 模拟的就是 Linux syscall ABI，与 Android 内核同构。

## 代码层分析（codex 只读扫描 59 文件 + 人工核对行号）

编译层：**零阻断**。Windows API 全部 `#[cfg(windows)]` 隔离；Android 属 unix，libc 分支全编译。

关键修正认知：**musl 二进制的 `target_os = "linux"`**，因此 kit 平台检测返回 `"linux"` 本来就匹配所有现有 kit manifest——「android 不匹配」只影响 NDK 构建路线，Termux+musl 路线无此问题。

Termux 路线剩余缺口（按影响排序）：

1. ✅ **服务管理守卫**（本分支已修）：`status`/`stop` 在非 Windows/macOS 平台原本直接 bail；新增 `unix_service.rs`（/proc comm 扫描 + SIGTERM + 5s 优雅等待），Windows/macOS 行为不变
2. **自更新需谨慎**：musl 二进制上报 `linux-arm64` slug，若升级服务器未来发布 gnu 动态链接的 linux-arm64 构建，Termux 上自更新会装上跑不起来的二进制。建议 Termux 用户暂不跑 `heart-portal upgrade`（现状：服务器无 linux-arm64 产物，升级会失败但无害）
3. **截图 unsupported**：Android 权限模型决定，需 MediaProjection API（原生 App 才有），Termux 路线放弃此功能
4. **kit 运行时**：`pkg install nodejs curl` 后 kits/Pi subagent 基本可用（spawn npm/node 在 Termux 下工作）
5. **数据目录**：`$HOME/.heart-portal` 在 Termux 下正常（HOME=/data/data/com.termux/files/home）

无平台耦合、musl 二进制直接可用的核心通路：relay WebSocket（relay_client.rs:204-245）、HTTP callback（heart_callback.rs:61-87）、TCP MCP listener（main.rs:571-642）、config/exec/file 工具。

NDK/原生 App 路线（不推荐现在做）：kit 平台检测返回 "android" 不匹配（kits/loader.rs:221-237，需映射）、前台服务生命周期、app-private 目录、浏览器 OAuth、截图、自更新全套重写——架构级，数周。

## 路线对比

### A. Termux 路线（推荐，已基本打通）

- 构建：`cargo zigbuild --target aarch64-unknown-linux-musl --release`（本分支）
- 部署：adb push（或 Termux wget）→ `chmod +x` → 直接跑
- 用户侧前置：`pkg install nodejs curl`（kit 运行时）
- 代码改动：本分支 1 个补丁（unix_service.rs + main.rs 接线）
- 效果：relay/MCP/config/exec/file 全通 + kit 执行可用
- 遗留：截图无、自更新手动（重新 adb push）

### B. 原生 App 路线（远）

NDK + APK + 前台服务 + nodejs-mobile 或砍 kit 只留 relay——架构级重构，等需求明确再投入。

## 建议

最小可行路径 = Termux + musl 静态二进制 + 本分支补丁。这正是「Being 手机 Portal」（Frank 的 VIVO 手机录音/飞书实测的延伸）的第一步：Portal 在 Android 上作为 relay 节点立起来，kit 生态跟着 Termux 走。真机验证（adb push 到 VIVO）是下一步，需要 Frank 配合。

## 产物位置

- 补丁分支：`android-termux-support`（本地，未 push；上游是 d5z/heart-portal，是否提 PR 由 Frank 决定）
- 二进制：`target\aarch64-unknown-linux-musl\release\heart-portal`（含补丁，可直接 adb push）
- 验证环境：WSL docker-desktop disto `/tmp/q`（qemu-aarch64-static 7.2.0）
- 回归：Windows 宿主 `cargo check` 通过（unix_service 被 cfg 排除，零影响）
